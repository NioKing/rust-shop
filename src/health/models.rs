use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ApplicationHealthResponse {
    pub database: DbHealth,
    pub rabbitmq: RmqHealth,
    pub redis: RedisHealth,
}

#[derive(Debug, Serialize, Default)]
pub struct DbHealth {
    pub status: Status,
    pub response_time_ms: u128,
}

#[derive(Debug, Serialize, Default)]
pub struct RmqHealth {
    pub status: Status,
    pub response_time_ms: u128,
}

#[derive(Debug, Serialize, Default)]
pub struct RedisHealth {
    pub status: Status,
    pub response_time_ms: u128,
}

#[derive(Debug, Serialize, Default)]
pub enum Status {
    Up,
    #[default]
    Down,
}
