use axum::{Router, routing::get};

use super::handlers;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new().route(
        "/carts",
        get(handlers::get_all_cart).post(handlers::add_products_to_cart),
    )
}
