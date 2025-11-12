use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ApplicationHealthResponse {
    pub database: DbHealth,
    pub rabbitmq: RmqHealth,
}

#[derive(Debug, Serialize)]
pub struct DbHealth {
    pub status: Status,
}

#[derive(Debug, Serialize)]
pub struct RmqHealth {
    pub status: Status,
}

#[derive(Debug, Serialize)]
pub enum Status {
    Up,
    Down,
}
