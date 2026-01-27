use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct Room {
    pub sender: broadcast::Sender<String>,
    pub users_count: i32,
}

// #[derive(Debug)]
// pub struct Message {
//     pub email: String,
//     pub text: String,
// }
//
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ChatMessage {
    Chat { email: String, text: String },
}
