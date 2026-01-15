use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct Room {
    // id: uuid::Uuid,
    pub sender: broadcast::Sender<String>,
    pub receiver_count: i32,
}
