use axum::{
    Router,
    routing::{delete, get, post},
};

use super::handlers;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new()
        .route(
            "/products",
            get(handlers::get_products).post(handlers::create_product_with_categories),
        )
        .route(
            "/products/{id}",
            delete(handlers::delete_product)
                .patch(handlers::update_product)
                .get(handlers::get_product_by_id),
        )
        .route("/products/{id}/image", post(handlers::upload_image))
}
