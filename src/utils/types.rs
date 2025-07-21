#![allow(dead_code)]

use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

pub type Result<T> = std::result::Result<axum::Json<T>, (axum::http::StatusCode, String)>;
