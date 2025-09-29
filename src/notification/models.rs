use axum_shop::schema::user_subscriptions;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize, Serialize)]
pub struct WelcomeNotification {
    pub event: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Notification {
    Discount(DiscountNotification),
    WelcomeUser(WelcomeNotification),
}

#[derive(Debug, Serialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = user_subscriptions)]
pub struct UserSubscriptions {
    pub user_id: uuid::Uuid,
    pub channel: String,
    pub orders_notifications: bool,
    pub discount_notifications: bool,
    pub newsletter_notifications: bool,
}
