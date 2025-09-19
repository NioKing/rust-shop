use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Notification {
    pub event: String,
    pub id: i32,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DiscountNotification {
    pub event: String,
    pub id: i32,
    pub title: String,
    pub amount: bigdecimal::BigDecimal,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub discount_type: String,
}
