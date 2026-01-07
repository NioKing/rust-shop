use axum::{Router, routing::get};

use super::handlers;
use crate::utils::types::{AppState, Pool};

pub fn get_routes() -> Router<AppState> {
    Router::new().route(
        "/carts",
        get(handlers::get_all_cart)
            .post(handlers::add_products_to_cart)
            .delete(handlers::remove_product_from_cart),
    )
}
