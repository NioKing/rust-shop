use axum::{
    Router,
    routing::{delete, get, post},
};

use super::handlers;
use crate::utils::types::AppState;

pub fn get_routes() -> Router<AppState> {
    Router::new().route("/health", get(handlers::check_application_health))
}
