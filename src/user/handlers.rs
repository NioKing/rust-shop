use super::models::{Address, GeocodeResponse, NewAddress, Profile, UpdateAddress, UpdateProfile};

use crate::auth::models::AccessTokenClaims;
use crate::utils::{internal_error, parse_user_id, types::Pool};
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
) -> Result<Json<Profile>, (StatusCode, String)> {
    use axum_shop::schema::profiles;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = profiles::table
        .filter(profiles::user_id.eq(&id))
        .select(Profile::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_current_user_profile(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<Json<Profile>, (StatusCode, String)> {
    use axum_shop::schema::profiles;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = profiles::table
        .filter(profiles::user_id.eq(&user_id))
        .select(Profile::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_profile(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProfile>,
) -> Result<Json<Profile>, (StatusCode, String)> {
    use axum_shop::schema::profiles;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = diesel::update(profiles::table.find(&id))
        .set(&payload)
        .returning(Profile::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_current_user_profile(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    claims: AccessTokenClaims,
    Json(payload): Json<UpdateProfile>,
) -> Result<Json<Profile>, (StatusCode, String)> {
    use axum_shop::schema::profiles;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = diesel::update(profiles::table.filter(profiles::user_id.eq(&user_id)))
        .set(&payload)
        .returning(Profile::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn create_address_for_current_user(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
    Json(payload): Json<NewAddress>,
) -> Result<Json<Address>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

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
        .map_err(internal_error)?;

    let current_address = geocode_address(&payload.address_line).await?;

    let (lon, lat) = (
        current_address.lon.parse::<f64>().map_err(internal_error)?,
        current_address.lat.parse::<f64>().map_err(internal_error)?,
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
    //     .map_err(internal_error)?;

    Ok(Json(address))
    // Ok(Json(res))
}

pub async fn get_user_addresses_by_id(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Address>>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = addresses::table
        .filter(addresses::user_id.eq(&id))
        .select(Address::as_select())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_address(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAddress>,
) -> Result<Json<Address>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = diesel::update(addresses::table.find(&id))
        .set(&payload)
        .returning(Address::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_current_user_address(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    claims: AccessTokenClaims,
    Json(payload): Json<UpdateAddress>,
) -> Result<Json<Address>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

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
                .map_err(internal_error)?;

            if cur_default > 0 {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Only one address can be set as default".to_owned(),
                ));
            }
        }
    };

    let res = diesel::update(
        addresses::table.filter(addresses::user_id.eq(&user_id).and(addresses::id.eq(&id))),
    )
    .set(&payload)
    .returning(Address::as_returning())
    .get_result(&mut conn)
    .await
    .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn delete_current_user_address(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    claims: AccessTokenClaims,
) -> Result<Json<Address>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = diesel::delete(
        addresses::table.filter(addresses::user_id.eq(&user_id).and(addresses::id.eq(&id))),
    )
    .returning(Address::as_returning())
    .get_result(&mut conn)
    .await
    .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_current_user_addresses(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<Json<Vec<Address>>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let user_id = parse_user_id(&claims.sub)?;

    let res = addresses::table
        .filter(addresses::user_id.eq(&user_id))
        .select(Address::as_select())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_current_user_default_address(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<Json<Address>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

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
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn geocode_address(address: &str) -> Result<GeocodeResponse, (StatusCode, String)> {
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
        .map_err(internal_error)?
        .json::<Vec<GeocodeResponse>>()
        .await
        .map_err(internal_error)?;

    println!("data: {:?}", data);

    let res = if let Some(val) = data.pop() {
        val
    } else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid address".to_owned(),
        ))?;
    };

    Ok(res)
}
