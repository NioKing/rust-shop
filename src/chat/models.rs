use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct Room {
    pub tx: broadcast::Sender<ChatMessage>,
    pub users_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ChatMessage {
    Chat { email: String, text: String },
    Join { email: String, room_id: uuid::Uuid },
    Disconnect { email: String, room_id: uuid::Uuid },
}

#[derive(Serialize, Debug)]
pub struct Rooms {
    pub id: uuid::Uuid,
}
