use axum::{
    Router,
    routing::{any, delete, get, post},
};

use super::handlers;
use crate::utils::types::AppState;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<AppState> {
    Router::new().route("/chat", any(handlers::handler))
}
