#![allow(dead_code, unused)]
use super::models::{NewUser, SafeUser, UpdateUser, UpdateUserPayload, User, UserEmail};
use crate::cart::models::NewCart;
use crate::utils::internal_error;
use crate::utils::types::Pool;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use bcrypt::{BcryptError, BcryptResult, DEFAULT_COST, hash, verify};
use chrono::Local;
use diesel::{prelude::*, update};
use diesel_async::RunQueryDsl;
use std::time::Instant;
use uuid::Uuid;

pub async fn create_user(
    State(pool): State<Pool>,
    ValidatedJson(payload): ValidatedJson<NewUser>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::carts;
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let hashed_pass = create_password_hash(payload.password_hash).await?;

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

pub async fn delete_user(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::carts;
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    diesel::delete(carts::table.filter(carts::user_id.eq(&id)))
        .execute(&mut conn)
        .await
        .map_err(internal_error)?;

    let res = diesel::delete(users::table.find(&id))
        .returning(SafeUser::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

pub async fn update_user_email_or_password(
    State(pool): State<Pool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUserPayload>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::users;
    let now = Instant::now();
    let mut conn = pool.get().await.map_err(internal_error)?;

    let user = users::table
        .find(id)
        .select(User::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    if payload.email.is_none() && payload.new_password.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "At least one field to update must be provided".to_string(),
        ));
    }

    if payload.new_password.is_some() && payload.current_password.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Current password is required to update password".to_string(),
        ));
    }

    // let mut is_valid = false;
    // if let Some(pass) = &payload.current_password {
    //     is_valid = validate_password(pass.to_owned(), user.password_hash).await?;
    // };

    let mut new_hash: Option<String> = None;

    if let (Some(cur), Some(new)) = (payload.current_password, payload.new_password) {
        // match validate_password(cur, user.password_hash).await? {
        //     true => new_hash = Some(create_password_hash(new).await?),
        //     false => {
        //         return Err((
        //             StatusCode::INTERNAL_SERVER_ERROR,
        //             "Password is invalid".to_string(),
        //         ));
        //     }
        // }
        new_hash = Some(create_password_hash(new).await?);

        if !validate_password(cur, user.password_hash).await? {
            return Err((StatusCode::UNAUTHORIZED, "Invalid password".into()));
        }
    };

    let updated_user = UpdateUser {
        email: payload.email,
        password_hash: new_hash,
    };

    println!("updated user: {:?}", updated_user);

    let res = diesel::update(users::table.find(&id))
        .set(&updated_user)
        .returning(SafeUser::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    println!("Time: {:.2?}", now.elapsed());
    Ok(Json(res))
}

pub async fn get_all_users(
    State(pool): State<Pool>,
) -> Result<Json<Vec<User>>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let res = users::table
        .select(User::as_select())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(Json(res))
}

async fn create_password_hash(password: String) -> Result<String, (StatusCode, String)> {
    let hashed_password = tokio::task::spawn_blocking(move || hash(password, DEFAULT_COST))
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

    Ok(hashed_password)
}

async fn validate_password(password: String, hash: String) -> Result<bool, (StatusCode, String)> {
    let is_valid = tokio::task::spawn_blocking(move || verify(password, &hash))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Verifying task failed: {}", e),
            )
        })?
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Verifying error: {}", e),
            )
        })?;

    Ok(is_valid)
}
