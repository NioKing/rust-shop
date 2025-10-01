use super::models::{Address, NewAddress, Profile, UpdateAddress, UpdateProfile};

use crate::auth::models::AccessTokenClaims;
use crate::utils::{internal_error, types::Pool};
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

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Failed to parse user id".to_owned(),
        )
    })?;

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

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Failed to parse user id".to_owned(),
        )
    })?;

    let res = diesel::update(profiles::table.filter(profiles::user_id.eq(&user_id)))
        .set(&payload)
        .returning(Profile::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn create_address(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NewAddress>,
) -> Result<Json<Address>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let address = Address {
        id: Uuid::new_v4(),
        user_id: id,
        label: payload.label,
        address_line: payload.address_line,
        city: payload.city,
        postal_code: payload.postal_code,
        country: payload.country,
    };

    let res = diesel::insert_into(addresses::table)
        .values(&address)
        .returning(Address::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
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

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Failed to parse user id".to_owned(),
        )
    })?;

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
    Json(payload): Json<UpdateAddress>,
) -> Result<Json<Address>, (StatusCode, String)> {
    use axum_shop::schema::addresses;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Failed to parse user id".to_owned(),
        )
    })?;

    let res = diesel::delete(
        addresses::table.filter(addresses::user_id.eq(&user_id).and(addresses::id.eq(&id))),
    )
    .returning(Address::as_returning())
    .get_result(&mut conn)
    .await
    .map_err(internal_error)?;

    Ok(Json(res))
}
