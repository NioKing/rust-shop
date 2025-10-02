use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use super::handlers;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new()
        .route("/users/{id}/profile", get(handlers::get_user_profile_by_id))
        .route("/profiles/{id}", patch(handlers::update_profile))
        .route(
            "/me/profile",
            patch(handlers::update_current_user_profile).get(handlers::get_current_user_profile),
        )
        .route(
            "/users/{id}/addresses",
            post(handlers::create_address).get(handlers::get_user_addresses_by_id),
        )
        .route("/addresses/{id}", patch(handlers::update_address))
        .route(
            "/me/addresses/{id}",
            patch(handlers::update_current_user_address)
                .delete(handlers::delete_current_user_address),
        )
        .route("/me/addresses", get(handlers::get_current_user_addresses))
}
