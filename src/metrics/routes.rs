use axum::{Router, routing::get};

use super::metrics;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new()
}
