use super::models::{NewUser, SafeUser, UpdateUser, User, UserEmail};
use crate::utils::internal_error;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use bcrypt::{DEFAULT_COST, hash, verify};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use uuid::Uuid;

pub async fn create_user(
    State(pool): State<Pool>,
    Json(payload): Json<NewUser>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let conn = pool.get().await.map_err(internal_error)?;

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

    let res = conn
        .interact(|conn| {
            diesel::insert_into(users::table)
                .values(user_data)
                .returning(SafeUser::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_user_by_id(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            users::table
                .filter(users::id.eq(&id))
                .select(SafeUser::as_select())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn get_user_by_email(
    State(pool): State<Pool>,
    Json(payload): Json<UserEmail>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(move |conn| {
            users::table
                .filter(users::email.eq(&payload.email))
                .select(SafeUser::as_select())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_user_email_or_password(
    State(pool): State<Pool>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    let conn = pool.get().await.map_err(internal_error)?;
    todo!()
}

pub async fn get_all_users(
    State(pool): State<Pool>,
) -> Result<Json<Vec<SafeUser>>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let conn = pool.get().await.map_err(internal_error)?;

    let res = conn
        .interact(|conn| users::table.select(SafeUser::as_select()).load(conn))
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(Json(res))
}
