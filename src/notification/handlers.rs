use super::models::{DiscountNotification, Notification};
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::env;
use tera::Tera;

use crate::error::{AppError, AppErrorKind};
use crate::utils::{internal_error, parse_user_id, types::Pool};
use anyhow::Context;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use diesel::{dsl::sql, prelude::*};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

const NOTIFICATION_TEMPLATES_PATH: &str = "src/templates/**/*";

pub async fn send_email(notification: Notification, pool: Pool) -> Result<(), AppError> {
    use axum_shop::schema::{user_subscriptions, users};

    let mut conn = pool.get().await.context(AppError::pool_context())?;

    match notification {
        Notification::Discount(data) => {
            let users: Vec<String> = users::table
                .inner_join(user_subscriptions::table.on(user_subscriptions::user_id.eq(users::id)))
                .filter(
                    user_subscriptions::channel
                        .eq("email")
                        .and(user_subscriptions::discount_notifications.eq(true)),
                )
                .select(users::email)
                .load(&mut conn)
                .await
                .context("Failed to get users")?;

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
        _ => return Err(AppError::validation("Failed to send an email")),
    }

    Ok(())
}

async fn build_email(
    receiver_name: &str,
    receiver_email: &str,
    subject: &str,
    body: String,
) -> Result<(), AppError> {
    let email = Message::builder()
        .from(Mailbox::new(
            Some("Rust shop".to_owned()),
            "example@mail.com"
                .parse()
                .context("Failed to parse email")?,
        ))
        .reply_to(Mailbox::new(
            Some("no-reply".to_owned()),
            "no-reply@rust.shop"
                .parse()
                .context("Failed to parse email")?,
        ))
        .to(Mailbox::new(
            Some(receiver_name.to_owned()),
            receiver_email
                .parse()
                .context("Failed to parse a reciever email")?,
        ))
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(body)
        .context("Failed to build a message")?;

    let creds = Credentials::new(
        env::var("SMTP_USERNAME").context("Smtp username must be set")?,
        env::var("SMTP_PASSWORD").context("Smtp password must be set")?,
    );

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .context("Wrong smtp transport")?
        .credentials(creds)
        .build();

    tokio::task::spawn_blocking(move || mailer.send(&email).context("Failed to send an email"))
        .await
        .context("Email send task failed")??;

    println!("email has been sent");

    Ok(())
}

fn render_html<T>(data: &T, filename: &str) -> Result<String, AppError>
where
    T: std::fmt::Debug + serde::Serialize,
{
    let tera = Tera::new(NOTIFICATION_TEMPLATES_PATH).context("Template not found")?;

    let mut ctx = tera::Context::new();
    ctx.insert("data", data);

    let html_body = tera
        .render(&format!("notifications/{}.html", filename), &ctx)
        .context("Failed to render html body")?;

    Ok(html_body)
}
