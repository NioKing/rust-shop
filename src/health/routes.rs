use axum::{
    Router,
    routing::{delete, get, post},
};

use super::handlers;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new().route("/health", get(handlers::check_application_health))
}
