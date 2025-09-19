use super::models::{DiscountNotification, Notification};
use axum::http::StatusCode;

pub async fn send_email(data: DiscountNotification) -> Result<(), String> {
    println!("Data: {:?}", data);

    Ok(())
}
