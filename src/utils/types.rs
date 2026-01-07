use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

use redis::Client;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub redis: Client,
    // chat
    pub tx: tokio::sync::broadcast::Sender<String>,
    pub user_count: Arc<Mutex<usize>>,
}
