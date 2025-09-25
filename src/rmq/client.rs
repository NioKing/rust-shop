use axum::http::StatusCode;
use futures_util::stream::StreamExt;
use lapin::{BasicProperties, Connection, ConnectionProperties, options::*, types::FieldTable};
use std::env;
use tokio_executor_trait::Tokio as TokioExec;
use tokio_reactor_trait::Tokio as TokioReactor;

use crate::utils::{internal_error, types::Pool};

async fn connect(url: &str) -> Result<Connection, (StatusCode, String)> {
    let conn = Connection::connect(
        url,
        ConnectionProperties::default()
            .with_executor(TokioExec::current())
            .with_reactor(TokioReactor::current()),
    )
    .await
    .map_err(internal_error)?;

    Ok(conn)
}

pub async fn publish_event(queue: &str, payload: &str) -> Result<(), (StatusCode, String)> {
    let url = env::var("RMQ_URL").map_err(internal_error)?;

    let channel = connect(&url)
        .await?
        .create_channel()
        .await
        .map_err(internal_error)?;

    channel
        .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
        .await
        .map_err(internal_error)?;

    channel
        .basic_publish(
            "",
            queue,
            BasicPublishOptions::default(),
            payload.as_bytes(),
            BasicProperties::default(),
        )
        .await
        .map_err(internal_error)?
        .await
        .map_err(internal_error)?;

    Ok(())
}

pub async fn consume<
    // T: for<'a> serde::Deserialize<'a> + std::fmt::Debug,
    // H: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), String>> + Send,
>(
    queue: &str,
    consumer_tag: &str,
    pool: crate::utils::types::Pool,
    handler: impl Fn(crate::notification::models::Notification, Pool) -> Fut + Send + Sync + 'static,
) -> Result<(), (StatusCode, String)> {
    let url = env::var("RMQ_URL").map_err(internal_error)?;

    let channel = connect(&url)
        .await?
        .create_channel()
        .await
        .map_err(internal_error)?;

    channel
        .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
        .await
        .map_err(internal_error)?;

    let mut consumer = channel
        .basic_consume(
            queue,
            consumer_tag,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(internal_error)?;

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.map_err(internal_error)?;
        let data = String::from_utf8_lossy(&delivery.data);

        println!("Data received: {}", data);

        if let Ok(notification) =
            serde_json::from_str::<crate::notification::models::Notification>(&data)
        {
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
            .map_err(internal_error)?;
    }

    Ok(())
}

pub fn spawn_consumer(queue: &'static str, tag: &'static str, pool: Pool) {
    tokio::spawn(async move {
        if let Err(er) = consume(queue, tag, pool, crate::notification::handlers::send_email).await
        {
            eprintln!("Error: {:?}", er);
        }
    });
}
