use axum::http::StatusCode;
use futures_util::stream::StreamExt;
use lapin::{BasicProperties, Connection, ConnectionProperties, options::*, types::FieldTable};
use std::env;
use tokio_executor_trait::Tokio as TokioExec;
use tokio_reactor_trait::Tokio as TokioReactor;

use crate::utils::internal_error;

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
    T: for<'a> serde::Deserialize<'a> + std::fmt::Debug,
    H: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), String>> + Send,
>(
    queue: &str,
    consumer_tag: &str,
    handler: H,
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

        println!("Data recieved: {}", data);

        if let Ok(notification) = serde_json::from_str::<T>(&data) {
            println!("Parsed data: {:?}", notification);

            if let Err(er) = handler(notification).await {
                eprint!("Failed so send email: {:?}", er);
            }
        }

        delivery
            .ack(BasicAckOptions::default())
            .await
            .map_err(internal_error)?;
    }

    Ok(())
}
