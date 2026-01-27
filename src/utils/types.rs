use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
pub type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

use crate::chat::models::ChatMessage;
use redis::Client;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, broadcast};

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub redis: Client,
    pub rooms: Arc<Mutex<HashMap<uuid::Uuid, broadcast::Sender<ChatMessage>>>>,
}
