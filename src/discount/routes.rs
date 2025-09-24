use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use super::handlers;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new()
        .route(
            "/discounts",
            get(handlers::get_all_discounts).post(handlers::create_discount),
        )
        .route(
            "/discounts/{id}",
            patch(handlers::update_discount).delete(handlers::delete_discount),
        )
        .route(
            "/discounts/{id}/products",
            post(handlers::add_discount_products).delete(handlers::remove_products_from_discount),
        )
}
