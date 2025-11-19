use axum::http::StatusCode;
use futures_util::stream::StreamExt;
use lapin::{BasicProperties, Connection, ConnectionProperties, options::*, types::FieldTable};
use std::env;
use tokio_executor_trait::Tokio as TokioExec;
use tokio_reactor_trait::Tokio as TokioReactor;

use crate::error::{AppError, AppErrorKind};
use crate::utils::{internal_error, types::Pool};
use anyhow::Context;

async fn connect(url: &str) -> Result<Connection, AppError> {
    let conn = Connection::connect(
        url,
        ConnectionProperties::default()
            .with_executor(TokioExec::current())
            .with_reactor(TokioReactor::current()),
    )
    .await
    .context("Failed establish rmq connection")?;

    Ok(conn)
}

pub async fn publish_event(queue: &str, payload: &str) -> Result<(), AppError> {
    let url = env::var("RMQ_URL").context("Rmq url must be set")?;

    let channel = connect(&url)
        .await?
        .create_channel()
        .await
        .context("Failed to create rmq channel")?;

    channel
        .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
        .await
        .context("Failed to declare queue")?;

    channel
        .basic_publish(
            "",
            queue,
            BasicPublishOptions::default(),
            payload.as_bytes(),
            BasicProperties::default(),
        )
        .await
        .context("Failed to create channel")?
        .await
        .context("Failed to publish queue")?;

    Ok(())
}

pub async fn consume<
    T: for<'a> serde::Deserialize<'a> + std::fmt::Debug,
    // H: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), String>> + Send,
>(
    queue: &str,
    consumer_tag: &str,
    pool: crate::utils::types::Pool,
    handler: impl Fn(T, Pool) -> Fut + Send + Sync + 'static,
) -> Result<(), AppError> {
    let url = env::var("RMQ_URL").context("Rmq url must be set")?;

    let channel = connect(&url)
        .await?
        .create_channel()
        .await
        .context("Failed to create channel")?;

    channel
        .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
        .await
        .context("Failed to declare queue")?;

    let mut consumer = channel
        .basic_consume(
            queue,
            consumer_tag,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .context("Failed to create consumer")?;

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.context("Failed to get data")?;
        let data = String::from_utf8_lossy(&delivery.data);

        println!("Data received: {}", data);

        if let Ok(notification) = serde_json::from_str::<T>(&data) {
            println!("Parsed data: {:?}", notification);

            let pool = pool.clone();

            if let Err(er) = handler(notification, pool).await {
                eprintln!("Failed so send an email: {:?}", er);
            }
        } else {
            eprintln!("Failed to parse a message: {:?}", data);
        }

        delivery
            .ack(BasicAckOptions::default())
            .await
            .context("Delivery acknowledge failed")?;
    }

    Ok(())
}

pub fn spawn_consumer<
    T: for<'a> serde::Deserialize<'a> + std::fmt::Debug + Send + Sync,
    Fut: Future<Output = Result<(), String>> + Send,
>(
    queue: &'static str,
    tag: &'static str,
    pool: Pool,
    handler: impl Fn(T, Pool) -> Fut + Send + Sync + 'static,
) {
    tokio::spawn(async move {
        if let Err(er) = consume(queue, tag, pool, handler).await {
            eprintln!("Error: {:?}", er);
        }
    });
}
