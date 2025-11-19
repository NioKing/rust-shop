use crate::error::{AppError, AppErrorKind};
use anyhow::Context;
use reqwest::StatusCode;
use uuid::Uuid;

pub fn parse_user_id(id: &str) -> Result<Uuid, AppError> {
    let user_id = Uuid::parse_str(id).context("Failed to parse uuid")?;

    Ok(user_id)
}
