use axum::{
    Router,
    routing::{get, patch, post},
};

use super::handlers;
use crate::utils::types::AppState;

pub fn get_routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(handlers::get_all_users))
        .route("/users/me", get(handlers::get_current_user))
        .route(
            "/users/{id}",
            patch(handlers::update_user_email_or_password)
                .get(handlers::get_user_by_id)
                .delete(handlers::delete_user),
        )
        .route("/auth/login", post(handlers::login_user))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/refresh", post(handlers::refresh_token))
        .route("/auth/register", post(handlers::create_user))
}
