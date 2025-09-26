use axum::{
    Json, RequestPartsExt,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_shop::schema::users;
use diesel::prelude::*;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use validator::Validate;

#[derive(Queryable, Selectable, Debug, Serialize, Insertable, AsChangeset, Validate)]
#[diesel(table_name=users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: uuid::Uuid,
    pub email: String,
    pub password_hash: String,
    pub hashed_rt: Option<String>,
    pub role: String,
}

#[derive(Insertable, Deserialize, Debug, Validate)]
#[diesel(table_name=users)]
pub struct NewUser {
    #[validate(email)]
    pub email: String,
    #[validate(length(
        min = 6,
        max = 50,
        message = "Your password should be at least 6 symbols long"
    ))]
    #[serde(rename = "password")]
    pub password_hash: String,
}

#[derive(Deserialize, Insertable, Validate)]
#[diesel(table_name=users)]
pub struct UserEmail {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Selectable, Queryable, Serialize)]
#[diesel(table_name=users)]
pub struct SafeUser {
    pub id: uuid::Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Insertable, Deserialize, Debug, Validate, AsChangeset)]
#[diesel(table_name=users)]
pub struct UpdateUser {
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 6, message = "Your password should be at least 6 symbols long"))]
    pub password_hash: Option<String>,
}

#[derive(Deserialize, Debug, Validate)]
pub struct UpdateUserPayload {
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 6, message = "Your password should be at least 6 symbols long"))]
    pub current_password: Option<String>,
    #[validate(length(min = 6, message = "Your password should be at least 6 symbols long"))]
    pub new_password: Option<String>,
}

#[derive(Deserialize, Debug, Validate)]
pub struct LoginUser {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Debug)]
pub struct SafeUserWithCart {
    #[serde(flatten)]
    pub user: SafeUser,
    pub cart: crate::cart::models::SafeCart,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshTokenClaims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct AccessToken {
    pub access_token: String,
}

#[derive(Serialize, Debug)]
pub struct NewRefreshToken<'a> {
    pub user_id: &'a uuid::Uuid,
    pub token_hash: &'a String,
    pub expires_at: chrono::NaiveDateTime,
}

#[derive(Serialize, Debug)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    FailedTask,
    MissingSecret,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    User,
    Seller,
    Admin,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::MissingSecret => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Token secret must be set",
            ),
            AuthError::FailedTask => (StatusCode::INTERNAL_SERVER_ERROR, "Task has failed"),
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
        };

        (status, error_message).into_response()
    }
}
