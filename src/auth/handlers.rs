#![allow(dead_code, unused)]
use super::models::{
    AccessToken, AccessTokenClaims, AuthError, LoginUser, NewRefreshToken, NewUser,
    RefreshTokenClaims, SafeUser, SafeUserWithCart, Tokens, UpdateUser, UpdateUserPayload, User,
    UserEmail,
};
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
use chrono::{Duration, Local, TimeZone, Utc};
use diesel::dsl::sql;
use diesel::{prelude::*, update};
use diesel_async::{AsyncConnection, RunQueryDsl};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::env;
use std::time::Instant;
use uuid::Uuid;

const QUEUE_NAME: &str = "user";

pub async fn create_user(
    State(pool): State<Pool>,
    ValidatedJson(payload): ValidatedJson<NewUser>,
) -> Result<Json<SafeUser>, (StatusCode, String)> {
    use crate::cart::models::NewCart;
    use crate::notification::models::UserSubscriptions;
    use crate::user::models::Profile;

    use axum_shop::schema::{carts, profiles, user_subscriptions, users};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let hashed_pass = create_hash(payload.password_hash).await?;

    let user_id = Uuid::new_v4();

    let res = conn
        .transaction::<SafeUser, diesel::result::Error, _>(move |mut conn| {
            Box::pin(async move {
                let user_data = User {
                    id: user_id,
                    email: payload.email,
                    password_hash: hashed_pass,
                    hashed_rt: None,
                    role: "user".to_owned(),
                };

                let user = diesel::insert_into(users::table)
                    .values(&user_data)
                    .returning(SafeUser::as_returning())
                    .get_result(&mut conn)
                    .await?;

                let updated_at = Local::now().date_naive();

                let cart_data = NewCart {
                    user_id,
                    updated_at,
                };

                let subs_data = UserSubscriptions {
                    user_id,
                    channel: "email".to_owned(),
                    orders_notifications: true,
                    discount_notifications: true,
                    newsletter_notifications: true,
                };

                let profile_data = Profile {
                    id: Uuid::new_v4(),
                    user_id,
                    first_name: None,
                    last_name: None,
                    phone_number: None,
                    birth_date: None,
                    language: "en".to_owned(),
                    currency: "usd".to_owned(),
                };

                diesel::insert_into(carts::table)
                    .values(&cart_data)
                    .execute(&mut conn)
                    .await?;

                diesel::insert_into(user_subscriptions::table)
                    .values(&subs_data)
                    .execute(&mut conn)
                    .await?;

                diesel::insert_into(profiles::table)
                    .values(&profile_data)
                    .execute(&mut conn)
                    .await?;

                Ok(user)
            })
        })
        .await
        .map_err(internal_error)?;

    let event = serde_json::json!({
        "type": "WelcomeUser",
        "event": "user_created",
        "email": res.email,
    })
    .to_string();

    if let Err(er) = crate::rmq::client::publish_event(QUEUE_NAME, &event).await {
        eprintln!("Failed to publish event: {:?}", er);
    }

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
            "At least one field to update must be provided".to_owned(),
        ));
    }

    if payload.new_password.is_some() && payload.current_password.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Current password is required to update password".to_owned(),
        ));
    }

    let mut new_hash: Option<String> = None;

    if let (Some(cur), Some(new)) = (payload.current_password, payload.new_password) {
        new_hash = Some(create_hash(new).await?);

        if !validate_hash(cur, user.password_hash).await? {
            return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_owned()));
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
) -> Result<Json<Vec<SafeUser>>, (StatusCode, String)> {
    use axum_shop::schema::users;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let rows = users::table
        // .inner_join(carts::table)
        // .inner_join(profiles::table)
        // .left_join(cart_products::table.on(carts::id.eq(cart_products::cart_id)))
        // .left_join(products::table.on(cart_products::product_id.eq(products::id)))
        // .select((
        //     SafeUser::as_select(),
        //     Cart::as_select(),
        //     Profile::as_select(),
        //     sql::<diesel::sql_types::Json>(
        //         "COALESCE(
        //         json_agg(
        //         json_build_object(
        //             'id', products.id,
        //             'title', products.title,
        //             'price', products.price,
        //             'description', products.description,
        //             'image', products.image,
        //             'quantity', cart_products.quantity
        //         )
        //     ) FILTER (WHERE products.id IS NOT NULL),
        //     '[]'
        // )",
        //     ),
        // ))
        // .load::<(SafeUser, Cart, Profile, serde_json::Value)>(&mut conn)
        .select(SafeUser::as_select())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    // let addresses = addresses::table
    //     .select(Address::as_select())
    //     .load(&mut conn)
    //     .await
    //     .map_err(internal_error)?;

    // let mut addr_map: HashMap<Uuid, Vec<Address>> = HashMap::new();
    //
    // for addr in addresses {
    //     addr_map.entry(addr.user_id).or_default().push(addr);
    // }

    // let res = rows
    //     .into_iter()
    //     .map(|(user, cart, profile, products_json)| {
    //         let address = addr_map.remove(&user.id).unwrap_or_default();
    //         let products = serde_json::from_value(products_json).unwrap_or_default();
    //         let cart = CartWithProducts { cart, products };
    //         SafeUserWithCart {
    //             user,
    //             cart,
    //             address,
    //             profile: profile,
    //         }
    //     })
    //     .collect();

    Ok(Json(rows))
}

pub async fn get_current_user(
    State(pool): State<Pool>,
    claims: AccessTokenClaims,
) -> Result<Json<SafeUserWithCart>, (StatusCode, String)> {
    use crate::cart::models::{Cart, CartWithProducts, SafeCart};
    use crate::product::models::ProductWithQty;
    use crate::user::models::{Address, Profile};
    use axum_shop::schema::{addresses, cart_products, carts, products, profiles, users};

    let mut conn = pool.get().await.map_err(internal_error)?;

    let user_id = Uuid::parse_str(&claims.sub).unwrap();

    let (user, cart, profile) = users::table
        .filter(users::id.eq(&user_id))
        .inner_join(carts::table)
        .inner_join(profiles::table)
        .select((
            SafeUser::as_select(),
            Cart::as_select(),
            Profile::as_select(),
        ))
        .get_result::<(SafeUser, Cart, Profile)>(&mut conn)
        .await
        .map_err(internal_error)?;

    let products_json = cart_products::table
        .inner_join(products::table.on(cart_products::product_id.eq(products::id)))
        .filter(cart_products::cart_id.eq(cart.id))
        .select(sql::<diesel::sql_types::Json>(
            "json_build_object(
                'id', products.id,
                'title', products.title,
                'price', products.price,
                'description', products.description,
                'image', products.image,
                'quantity', cart_products.quantity
            )",
        ))
        .load::<serde_json::Value>(&mut conn)
        .await
        .map_err(internal_error)?;

    let address = addresses::table
        .filter(addresses::user_id.eq(&user_id))
        .select(Address::as_select())
        .order_by(addresses::is_default.desc())
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    let products = products_json
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    let cart_with_products = CartWithProducts { cart, products };

    let res = SafeUserWithCart {
        user,
        cart: cart_with_products,
        address,
        profile,
    };

    Ok(Json(res))
}

pub async fn login_user(
    State(pool): State<Pool>,
    Json(payload): Json<LoginUser>,
) -> Result<Json<Tokens>, (StatusCode, String)> {
    use axum_shop::schema::users;
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

    let (access_token, refresh_token) = create_tokens_pair(
        Duration::minutes(5),
        Duration::days(7),
        &user.id.to_string(),
        &user.email,
        &user.role,
    )
    .await?;

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

    let id = Uuid::parse_str(&claims.sub).map_err(internal_error)?;

    let user = users::table
        .find(&id)
        .select(User::as_select())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    if let Some(hash) = &user.hashed_rt {
        validate_hash(token.to_owned(), hash.to_owned()).await?;
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Please, use login instead".to_owned(),
        ));
    };

    let (access_token, refresh_token) = create_tokens_pair(
        Duration::minutes(5),
        Duration::days(7),
        &user.id.to_string(),
        &user.email,
        &user.role,
    )
    .await?;

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

async fn create_tokens_pair(
    access_duration: Duration,
    refresh_duration: Duration,
    id: &str,
    email: &str,
    role: &str,
) -> Result<(String, String), (StatusCode, String)> {
    let access_exprires = Utc::now() + access_duration;
    let access_claims = AccessTokenClaims {
        sub: id.to_owned(),
        email: email.to_owned(),
        role: role.to_owned(),
        exp: access_exprires.timestamp() as usize,
    };

    let refresh_expires = Utc::now() + refresh_duration;
    let refresh_claims = RefreshTokenClaims {
        sub: id.to_owned(),
        exp: refresh_expires.timestamp() as usize,
    };

    let at_secret = env::var("AT_SECRET").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AT_SECRET not set".to_owned(),
        )
    })?;
    let rt_secret = env::var("RT_SECRET").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "RT_SECRET not set".to_owned(),
        )
    })?;

    let (access_token, refresh_token) = tokio::try_join!(
        encode_token(access_claims, &at_secret),
        encode_token(refresh_claims, &rt_secret),
    )?;

    Ok((access_token, refresh_token))
}

impl<S> FromRequestParts<S> for AccessTokenClaims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let secret = env::var("AT_SECRET").map_err(|_| AuthError::MissingSecret)?;

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
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let secret = env::var("RT_SECRET").map_err(|_| AuthError::MissingSecret)?;

        let token_data = decode_token(&bearer.token(), &secret).await?;

        println!("Token data: {:?}", token_data);

        Ok(token_data.claims)
    }
}

async fn encode_token<T: Sync + DeserializeOwned + 'static + Serialize + Send>(
    claims: T,
    secret: &str,
) -> Result<String, (StatusCode, String)> {
    let secret = secret.to_owned();
    // let claims = claims.clone();

    let token = tokio::task::spawn_blocking({
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

    diesel::update(users::table.filter(users::id.eq(id).and(users::hashed_rt.is_not_null())))
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
