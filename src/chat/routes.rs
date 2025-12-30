use axum::{
    Router,
    routing::{any, delete, get, post},
};

use super::handlers;
use crate::utils::types::Pool;

pub fn get_routes() -> Router<Pool> {
    Router::new()
}
