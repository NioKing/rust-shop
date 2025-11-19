use super::models::{
    Address, GeocodeResponse, NewAddress, NewSubscriptionPayload, NewUserSubscriptions, Profile,
    UpdateAddress, UpdateAddressPayload, UpdateProfile, UpdateUserSubscriptions, UserSubscription,
};

use crate::auth::models::AccessTokenClaims;
use crate::error::{AppError, AppErrorKind};
use crate::utils::{internal_error, parse_user_id, types::Pool};
use anyhow::Context;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use diesel::{dsl::sql, prelude::*};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

pub async fn get_user_profile_by_id(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Profile>, AppError> {
    use axum_shop::schema::profiles;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let res = profiles::table
        .filter(profiles::user_id.eq(&id))
        .select(Profile::as_select())
        .get_result(&mut conn)
        .await
        .context("Failed to get user")?;

    Ok(Json(res))
}

pub async fn get_current_user_profile(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<Json<Profile>, AppError> {
    use axum_shop::schema::profiles;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = profiles::table
        .filter(profiles::user_id.eq(&user_id))
        .select(Profile::as_select())
        .get_result(&mut conn)
        .await
        .context("Failed to get profile")?;

    Ok(Json(res))
}

pub async fn update_profile(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProfile>,
) -> Result<Json<Profile>, AppError> {
    use axum_shop::schema::profiles;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let res = diesel::update(profiles::table.find(&id))
        .set(&payload)
        .returning(Profile::as_returning())
        .get_result(&mut conn)
        .await
        .context("Failed to update profile")?;

    Ok(Json(res))
}

pub async fn update_current_user_profile(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    claims: AccessTokenClaims,
    Json(payload): Json<UpdateProfile>,
) -> Result<Json<Profile>, AppError> {
    use axum_shop::schema::profiles;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = diesel::update(profiles::table.filter(profiles::user_id.eq(&user_id)))
        .set(&payload)
        .returning(Profile::as_returning())
        .get_result(&mut conn)
        .await
        .context("Failed to update profile")?;

    Ok(Json(res))
}

pub async fn create_address_for_current_user(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
    Json(payload): Json<NewAddress>,
) -> Result<Json<Address>, AppError> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let current_default: i64 = addresses::table
        .filter(
            addresses::user_id
                .eq(&user_id)
                .and(addresses::is_default.eq(true)),
        )
        .select(diesel::dsl::count_star())
        .get_result(&mut conn)
        .await
        .context("Failed to get user default address")?;

    let current_address = geocode_address(&payload.address_line).await?;

    let (lon, lat) = (
        current_address
            .lon
            .parse::<f64>()
            .context("Failed to get longitude")?,
        current_address
            .lat
            .parse::<f64>()
            .context("Failed to get latitude")?,
    );

    let address = Address {
        id: Uuid::new_v4(),
        user_id,
        label: payload.label,
        address_line: current_address.display_name,
        city: Some(current_address.address.city),
        postal_code: Some(current_address.address.postcode),
        country: Some(current_address.address.country),
        longitude: Some(lon),
        latitude: Some(lat),
        is_default: current_default <= 0,
    };

    println!("Address: {:?}", address);

    // let res = diesel::insert_into(addresses::table)
    //     .values(&address)
    //     .returning(Address::as_returning())
    //     .get_result(&mut conn)
    //     .await
    // .context("Failed to create address")?;

    Ok(Json(address))
    // Ok(Json(res))
}

pub async fn get_user_addresses_by_id(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Address>>, AppError> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let res = addresses::table
        .filter(addresses::user_id.eq(&id))
        .select(Address::as_select())
        .load(&mut conn)
        .await
        .context("Failed to get address")?;

    Ok(Json(res))
}

pub async fn update_address(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAddress>,
) -> Result<Json<Address>, AppError> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let res = diesel::update(addresses::table.find(&id))
        .set(&payload)
        .returning(Address::as_returning())
        .get_result(&mut conn)
        .await
        .context("Failed to update address")?;

    Ok(Json(res))
}

pub async fn update_current_user_address(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    claims: AccessTokenClaims,
    Json(payload): Json<UpdateAddressPayload>,
) -> Result<Json<Address>, AppError> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    if let Some(is_default) = payload.is_default {
        if is_default == true {
            let cur_default: i64 = addresses::table
                .filter(
                    addresses::user_id
                        .eq(&user_id)
                        .and(addresses::is_default.eq(true)),
                )
                .select(diesel::dsl::count_star())
                .get_result(&mut conn)
                .await
                .context("Failed to get current address")?;

            if cur_default > 0 {
                return Err(AppError::validation("Only one address can be default"));
            }
        }
    };

    if let Some(address_line) = &payload.address_line {
        let geo = geocode_address(address_line).await?;
        let lat = geo.lat.parse::<f64>().context("Failed to parse latitude")?;

        let lon = geo
            .lon
            .parse::<f64>()
            .context("Failed to parse longitude")?;

        diesel::update(addresses::table.find(&id))
            .set((
                addresses::address_line.eq(&geo.display_name),
                addresses::city.eq(&geo.address.city),
                addresses::country.eq(&geo.address.country),
                addresses::postal_code.eq(&geo.address.postcode),
                addresses::latitude.eq(&lat),
                addresses::longitude.eq(&lon),
            ))
            .execute(&mut conn)
            .await
            .context("Failed to update address")?;
    };

    let address_update = UpdateAddress {
        label: payload.label,
        is_default: payload.is_default,
    };

    let res = diesel::update(
        addresses::table.filter(addresses::user_id.eq(&user_id).and(addresses::id.eq(&id))),
    )
    .set(&address_update)
    .returning(Address::as_returning())
    .get_result(&mut conn)
    .await
    .context("Failed to update address")?;

    Ok(Json(res))
}

pub async fn delete_current_user_address(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    claims: AccessTokenClaims,
) -> Result<Json<Address>, AppError> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = diesel::delete(
        addresses::table.filter(addresses::user_id.eq(&user_id).and(addresses::id.eq(&id))),
    )
    .returning(Address::as_returning())
    .get_result(&mut conn)
    .await
    .context("Failed to delete address")?;

    Ok(Json(res))
}

pub async fn get_current_user_addresses(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<Json<Vec<Address>>, AppError> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = addresses::table
        .filter(addresses::user_id.eq(&user_id))
        .select(Address::as_select())
        .order_by(addresses::is_default.desc())
        .load(&mut conn)
        .await
        .context("Failed to get addresses")?;

    Ok(Json(res))
}

pub async fn get_current_user_default_address(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<Json<Address>, AppError> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = addresses::table
        .filter(
            addresses::user_id
                .eq(&user_id)
                .and(addresses::is_default.eq(true)),
        )
        .select(Address::as_select())
        .get_result(&mut conn)
        .await
        .context("Failed to get default address")?;

    Ok(Json(res))
}

pub async fn set_address_as_default(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    claims: AccessTokenClaims,
) -> Result<Json<Address>, AppError> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = conn
        .transaction::<Address, diesel::result::Error, _>(move |mut conn| {
            Box::pin(async move {
                let addresses = diesel::update(
                    addresses::table.filter(
                        addresses::user_id
                            .eq(&user_id)
                            .and(addresses::is_default.eq(true)),
                    ),
                )
                .set(addresses::is_default.eq(false))
                .execute(&mut conn)
                .await?;

                let res = diesel::update(
                    addresses::table
                        .filter(addresses::id.eq(&id).and(addresses::user_id.eq(&user_id))),
                )
                .set(addresses::is_default.eq(true))
                .returning(Address::as_returning())
                .get_result(&mut conn)
                .await?;

                Ok(res)
            })
        })
        .await
        .context("Failed to set address as default")?;

    Ok(Json(res))
}

pub async fn geocode_address(address: &str) -> Result<GeocodeResponse, AppError> {
    let address = format!("{}", address.replace(" ", "+"));

    println!("current adress: {:?}", address);

    let client = reqwest::Client::new();

    let mut data = client
        .get(&format!(
            "https://nominatim.openstreetmap.org/search?format=json&q={}&addressdetails=1",
            address
        ))
        .header("User-Agent", "axum-shop/1.0")
        .send()
        .await
        .context("Geocode request failed")?
        .json::<Vec<GeocodeResponse>>()
        .await
        .context("Failed to get json geodata")?;

    println!("data: {:?}", data);

    let res = if let Some(val) = data.pop() {
        val
    } else {
        return Err(AppError::validation("Invalid address"));
    };

    Ok(res)
}

pub async fn get_all_current_user_subscriptions(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<Json<Vec<UserSubscription>>, AppError> {
    use axum_shop::schema::user_subscriptions;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = user_subscriptions::table
        .filter(user_subscriptions::user_id.eq(&user_id))
        .select(UserSubscription::as_select())
        .load(&mut conn)
        .await
        .context("Failed to get current user subscriptions")?;

    Ok(Json(res))
}

pub async fn update_current_user_subscription(
    State(pool): State<Pool>,
    Path(channel): Path<String>,
    claims: AccessTokenClaims,
    Json(payload): Json<UpdateUserSubscriptions>,
) -> Result<Json<UserSubscription>, AppError> {
    use axum_shop::schema::user_subscriptions;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = diesel::update(
        user_subscriptions::table.filter(
            user_subscriptions::channel
                .eq(&channel)
                .and(user_subscriptions::user_id.eq(&user_id)),
        ),
    )
    .set(&payload)
    .returning(UserSubscription::as_returning())
    .get_result(&mut conn)
    .await
    .context("Failed to update current user subscriptions")?;

    Ok(Json(res))
}

pub async fn create_current_user_subscription(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
    Json(payload): Json<NewSubscriptionPayload>,
) -> Result<Json<UserSubscription>, AppError> {
    use axum_shop::schema::user_subscriptions;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let new_subscription = NewUserSubscriptions {
        user_id,
        channel: payload.channel,
        orders_notifications: true,
        discount_notifications: true,
        newsletter_notifications: true,
    };

    let res = diesel::insert_into(user_subscriptions::table)
        .values(&new_subscription)
        .returning(UserSubscription::as_returning())
        .get_result(&mut conn)
        .await
        .context("Failed to create user subscription")?;

    Ok(Json(res))
}

pub async fn delete_current_user_subscription(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
    Path(channel): Path<String>,
    Json(payload): Json<NewSubscriptionPayload>,
) -> Result<Json<UserSubscription>, AppError> {
    use axum_shop::schema::user_subscriptions;

    let mut conn = pool.get().await.context("Failed to get db connection")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = diesel::delete(
        user_subscriptions::table.filter(
            user_subscriptions::channel
                .eq(&channel)
                .and(user_subscriptions::user_id.eq(&user_id)),
        ),
    )
    .returning(UserSubscription::as_returning())
    .get_result(&mut conn)
    .await
    .context("Failed to delete subscription")?;

    Ok(Json(res))
}

pub async fn get_current_user_subscription(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
    Path(channel): Path<String>,
) -> Result<Json<UserSubscription>, AppError> {
    use axum_shop::schema::user_subscriptions;

    let mut conn = pool.get().await.context("Failed to get db connecton")?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = user_subscriptions::table
        .filter(
            user_subscriptions::user_id
                .eq(&user_id)
                .and(user_subscriptions::channel.eq(&channel)),
        )
        .select(UserSubscription::as_select())
        .get_result(&mut conn)
        .await
        .context("Failed to get subscription")?;

    Ok(Json(res))
}
