use axum::{Router, routing::get};

use super::metrics;
use crate::utils::types::AppState;

pub fn get_routes() -> Router<AppState> {
    Router::new()
}
