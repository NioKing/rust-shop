use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use super::handlers;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new().route(
        "/users/{id}/profiles",
        get(handlers::get_user_profile_by_id),
    )
}
