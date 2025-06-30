#![allow(dead_code, unused)]
use super::models::{NewUser, SafeUser, UpdateUser, User, UserEmail};
use crate::cart::models::NewCart;
use crate::utils::internal_error;
use crate::utils::types::Pool;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::Local;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

pub async fn create_user(
    State(pool): State<Pool>,
    ValidatedJson(payload): ValidatedJson<NewUser>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::carts;
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let hashed_pass =
        tokio::task::spawn_blocking(move || hash(payload.password_hash, DEFAULT_COST))
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Hashing task failed: {}", e),
                )
            })?
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Hashing error: {}", e),
                )
            })?;

    let user_id = Uuid::new_v4();

    let user_data = User {
        id: user_id,
        email: payload.email,
        password_hash: hashed_pass,
        hashed_rt: None,
    };

    let res = diesel::insert_into(users::table)
        .values(&user_data)
        .returning(SafeUser::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    let updated_at = Local::now().date_naive();

    let cart_data = NewCart {
        user_id,
        updated_at,
    };

    diesel::insert_into(carts::table)
        .values(&cart_data)
        .execute(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_user_by_id(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = users::table
        .filter(users::id.eq(&id))
        .select(SafeUser::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_user_by_email(
    State(pool): State<Pool>,
    Json(payload): Json<UserEmail>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = users::table
        .filter(users::email.eq(&payload.email))
        .select(SafeUser::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_user_email_or_password(
    State(pool): State<Pool>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(internal_error)?;
    todo!()
}

pub async fn get_all_users(
    State(pool): State<Pool>,
) -> Result<Json<Vec<SafeUser>>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = users::table
        .select(SafeUser::as_select())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}
