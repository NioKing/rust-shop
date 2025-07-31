#![allow(dead_code, unused)]
use super::models::{
    AccessTokenClaims, LoginUser, NewUser, SafeUser, UpdateUser, UpdateUserPayload, User, UserEmail,
};
use crate::auth::models::{AccessToken, AuthError, NewRefreshToken, RefreshTokenClaims, Tokens};
use crate::cart::models::NewCart;
use crate::utils::internal_error;
use crate::utils::types::Pool;
use axum::RequestPartsExt;
use axum::extract::FromRequestParts;
use axum::http::HeaderMap;
use axum::http::request::Parts;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use axum_validated_extractors::ValidatedJson;
use bcrypt::{BcryptError, BcryptResult, DEFAULT_COST, hash, verify};
use chrono::{Duration, Local, Utc};
use diesel::{prelude::*, update};
use diesel_async::RunQueryDsl;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::env;
use std::time::Instant;
use uuid::Uuid;

pub async fn create_user(
    State(pool): State<Pool>,
    ValidatedJson(payload): ValidatedJson<NewUser>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use axum_shop::schema::carts;
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let hashed_pass = create_hash(payload.password_hash).await?;

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
        new_hash = Some(create_hash(new).await?);

        if !validate_hash(cur, user.password_hash).await? {
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

pub async fn login_user(
    State(pool): State<Pool>,
    Json(payload): Json<LoginUser>,
) -> Result<Json<Tokens>, (StatusCode, String)> {
    use axum_shop::schema::users;
    // dotenv::dotenv().ok();
    let now = Instant::now();
    let mut conn = pool.get().await.map_err(internal_error)?;

    let user = users::table
        .filter(users::email.eq(payload.email))
        .first::<User>(&mut conn)
        .await
        .map_err(internal_error)?;

    if !validate_hash(payload.password, user.password_hash).await? {
        return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_owned()));
    }

    let access_exprires = Utc::now() + Duration::minutes(5);
    let access_claims = AccessTokenClaims {
        sub: user.id.to_string().clone(),
        email: user.email.clone(),
        exp: access_exprires.timestamp() as usize,
    };

    let at_secret = env::var("AT_SECRET").expect("Access token secret must be set");
    let rt_secret = env::var("RT_SECRET").expect("Refresh token secret must be set");

    let refresh_expires = Utc::now() + Duration::days(7);
    let refresh_claims = RefreshTokenClaims {
        sub: user.id.to_string().clone(),
        // email: user.email.clone(),
        exp: refresh_expires.timestamp() as usize,
    };

    let (access_token, refresh_token) = tokio::try_join!(
        encode_token(access_claims, &at_secret),
        encode_token(refresh_claims, &rt_secret),
    )?;

    let refresh_token_hash = create_hash(refresh_token.clone()).await?;

    diesel::update(users::table.filter(users::id.eq(user.id)))
        .set(users::hashed_rt.eq(&refresh_token_hash))
        .execute(&mut conn)
        .await
        .map_err(internal_error)?;

    // let cookie_value = format!(
    //     "refresh_token={}; HttpOnly; Max-Age={}; Secure; Path=/auth/refresh; SameSite=Strict",
    //     refresh_token,
    //     7 * 24 * 60 * 60
    // );

    // let mut headers = HeaderMap::new();
    // headers.insert("Set-Cookie", cookie_value.parse().unwrap());

    let tokens = Tokens {
        access_token,
        refresh_token,
    };

    println!("Time: {:.2?}", now.elapsed());

    Ok(Json(tokens))
}

pub async fn refresh_token(
    State(pool): State<Pool>,
    claims: RefreshTokenClaims,
    bearer: TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Tokens>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let token = bearer.token();

    let id = Uuid::parse_str(&claims.sub).unwrap();

    let user = users::table
        .find(&id)
        .select(User::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    if let Some(hash) = &user.hashed_rt {
        validate_hash(token.to_owned(), hash.to_owned()).await?;
        println!("is valid");
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Please, use login instead".to_owned(),
        ));
    };

    println!("user: {:?}", user);

    let access_exprires = Utc::now() + Duration::minutes(5);
    let access_claims = AccessTokenClaims {
        sub: user.id.to_string().clone(),
        email: user.email.clone(),
        exp: access_exprires.timestamp() as usize,
    };

    let at_secret = env::var("AT_SECRET").expect("Access token secret must be set");
    let rt_secret = env::var("RT_SECRET").expect("Refresh token secret must be set");

    let refresh_expires = Utc::now() + Duration::days(7);
    let refresh_claims = RefreshTokenClaims {
        sub: user.id.to_string().clone(),
        // email: user.email.clone(),
        exp: refresh_expires.timestamp() as usize,
    };

    let (access_token, refresh_token) = tokio::try_join!(
        encode_token(access_claims, &at_secret),
        encode_token(refresh_claims, &rt_secret),
    )?;

    let refresh_token_hash = create_hash(refresh_token.clone()).await?;

    diesel::update(users::table.filter(users::id.eq(user.id)))
        .set(users::hashed_rt.eq(&refresh_token_hash))
        .execute(&mut conn)
        .await
        .map_err(internal_error)?;

    let tokens = Tokens {
        access_token,
        refresh_token,
    };

    Ok(Json(tokens))
}

impl<S> FromRequestParts<S> for AccessTokenClaims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // let path = parts.uri.path().split('/').last().unwrap();

        // let secret = match path {
        //     "refresh" => env::var("RT_SECRET").expect("RT_SECRET missing"),
        //     _ => env::var("AT_SECRET").expect("AT_SECRET missing"),
        // };

        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let secret = env::var("AT_SECRET").expect("AT_SECRET missing");
        let token_data = decode_token(&bearer.token(), &secret).await?;

        println!("Token data: {:?}", token_data);

        Ok(token_data.claims)
    }
}

impl<S> FromRequestParts<S> for RefreshTokenClaims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // let path = parts.uri.path().split('/').last().unwrap();
        //
        // let secret = match path {
        //     "refresh" => env::var("RT_SECRET").expect("RT_SECRET missing"),
        //     _ => env::var("AT_SECRET").expect("AT_SECRET missing"),
        // };

        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let secret = env::var("RT_SECRET").expect("RT_SECRET missing");
        let token_data = decode_token(&bearer.token(), &secret).await?;

        println!("Token data: {:?}", token_data);

        Ok(token_data.claims)
    }
}

async fn encode_token<T: Sync + DeserializeOwned + 'static + Serialize + Send>(
    claims: T,
    secret: &String,
) -> Result<String, (StatusCode, String)> {
    let token = tokio::task::spawn_blocking({
        // let claims = claims.clone();
        let secret = secret.clone();
        move || {
            let refresh_token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            );

            refresh_token
        }
    })
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    Ok(token)
}

pub async fn logout(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<(), (StatusCode, String)> {
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;
    let id = Uuid::parse_str(&claims.sub).unwrap();

    diesel::update(users::table.filter(users::id.eq(id)))
        .set(users::hashed_rt.eq(None::<String>))
        .execute(&mut conn)
        .await
        .map_err(internal_error)?;

    Ok(())
}

async fn decode_token<T: Send + DeserializeOwned + 'static>(
    token: &str,
    secret: &str,
) -> Result<TokenData<T>, AuthError> {
    let secret = secret.to_owned();
    let token = token.to_owned();

    let data = tokio::task::spawn_blocking(move || {
        decode::<T>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
    })
    .await
    .map_err(|_| AuthError::FailedTask)?
    .map_err(|_| AuthError::InvalidToken)?;

    Ok(data)
}

async fn create_hash(password: String) -> Result<String, (StatusCode, String)> {
    let hashed_password = tokio::task::spawn_blocking(move || hash(password, DEFAULT_COST))
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(hashed_password)
}

async fn validate_hash(password: String, hash: String) -> Result<bool, (StatusCode, String)> {
    let is_valid = tokio::task::spawn_blocking(move || verify(password, &hash))
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;

    Ok(is_valid)
}
