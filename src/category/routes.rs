use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use super::handlers;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new()
        .route(
            "/categories",
            get(handlers::get_categories).post(handlers::create_category),
        )
        .route(
            "/categories/{id}",
            patch(handlers::update_category).get(handlers::get_category_by_id),
        )
}
