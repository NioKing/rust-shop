use super::models::{DiscountNotification, Notification};
use axum::http::StatusCode;
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;

pub async fn send_email(notification: Notification) -> Result<(), String> {
    println!("Data: {:?}", notification);

    match notification {
        Notification::Discount(data) => {
            build_email(
                "kenny3850",
                "kenny3850@gmail.com",
                "Check out our new discounts",
                "Hi, new discount has arrived",
            )?;
        }
        Notification::WelcomeUser(data) => {
            build_email(
                "kenny3850",
                "kenny3850@gmail.com",
                "Check out our new discounts",
                "Hi, new discount has arrived",
            )?;
        }
        _ => return Err("Failed to send an email".to_owned()),
    }

    Ok(())
}

fn build_email(
    receiver_name: &str,
    receiver_email: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let email = Message::builder()
        .from(Mailbox::new(
            Some("Rust shop".to_owned()),
            "example@mail.com"
                .parse()
                .map_err(|e| format!("Failed to parse sender email: {}", e))?,
        ))
        .reply_to(Mailbox::new(
            Some("no-reply".to_owned()),
            "no-reply@rust.shop"
                .parse()
                .map_err(|e| format!("Failed to parse reply to email: {}", e))?,
        ))
        .to(Mailbox::new(
            Some(receiver_name.to_owned()),
            receiver_email
                .parse()
                .map_err(|e| format!("Failed to parse receiver email: {}", e))?,
        ))
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body("Hi and welcome to rust shop".to_owned())
        .map_err(|e| format!("Failed to build a message: {}", e))?;

    let creds = Credentials::new(
        env::var("SMTP_USERNAME").map_err(|e| format!("smtp username must be set: {}", e))?,
        env::var("SMTP_PASSWORD").map_err(|e| format!("smtp password must be set: {}", e))?,
    );

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .map_err(|e| format!("Wrong smtp transport: {}", e))?
        .credentials(creds)
        .build();

    mailer
        .send(&email)
        .map_err(|e| format!("failed to send an email: {}", e))?;

    println!("email has been sent");

    Ok(())
}
