use axum::http::StatusCode;
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
