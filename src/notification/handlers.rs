use super::models::{DiscountNotification, Notification};
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;
use tera::{Context, Tera};

use crate::utils::{internal_error, types::Pool};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use diesel::{dsl::sql, prelude::*};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

const NOTIFICATION_TEMPLATES_PATH: &str = "src/templates/**/*";

pub async fn send_email(notification: Notification, pool: Pool) -> Result<(), String> {
    use axum_shop::schema::users;

    let mut conn = pool
        .get()
        .await
        .map_err(|e| format!("Failed to get pool: {}", e))?;

    match notification {
        Notification::Discount(data) => {
            let users: Vec<String> = users::table
                .select(users::email)
                .load(&mut conn)
                .await
                .map_err(|e| format!("Failed to get users: {}", e))?;

            let html_body = render_html(&data, "discount")?;

            // for user in users {
            //     let name = &user.split("@").collect::<Vec<_>>()[0];
            //     let email = &user;
            //
            //     build_email(
            //         name,
            //         email,
            //         "Checko out our new discount",
            //         html_body.clone(),
            //     ).await?;
            // }

            build_email(
                "kenny3850",
                "kenny3850@gmail.com",
                "Check out our new discounts",
                html_body,
            )
            .await?;
        }
        Notification::WelcomeUser(data) => {
            let html_body = render_html(&data, "welcome")?;

            // build_email(&data.email, &data.email, "Welcome to Rust shop!", html_body).await?;
        }
        _ => return Err("Failed to send an email".to_owned()),
    }

    Ok(())
}

async fn build_email(
    receiver_name: &str,
    receiver_email: &str,
    subject: &str,
    body: String,
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
        .header(ContentType::TEXT_HTML)
        .body(body)
        .map_err(|e| format!("Failed to build a message: {}", e))?;

    let creds = Credentials::new(
        env::var("SMTP_USERNAME").map_err(|e| format!("smtp username must be set: {}", e))?,
        env::var("SMTP_PASSWORD").map_err(|e| format!("smtp password must be set: {}", e))?,
    );

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .map_err(|e| format!("Wrong smtp transport: {}", e))?
        .credentials(creds)
        .build();

    tokio::task::spawn_blocking(move || {
        mailer
            .send(&email)
            .map_err(|e| format!("failed to send an email: {}", e))
    })
    .await
    .map_err(|e| format!("Email send task has failed: {}", e))??;

    println!("email has been sent");

    Ok(())
}

fn render_html<T>(data: &T, filename: &str) -> Result<String, String>
where
    T: std::fmt::Debug + serde::Serialize,
{
    let tera =
        Tera::new(NOTIFICATION_TEMPLATES_PATH).map_err(|e| format!("Template not found: {}", e))?;

    let mut ctx = Context::new();
    ctx.insert("data", data);

    let html_body = tera
        .render(&format!("notifications/{}.html", filename), &ctx)
        .map_err(|e| format!("Failed to render html body: {}", e))?;

    Ok(html_body)
}
