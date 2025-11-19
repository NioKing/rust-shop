use crate::utils::types::Pool;
use anyhow::{Context, anyhow};
use axum::{http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppErrorKind {
    #[error("resourse not found")]
    NotFound,
    #[error("invalid input")]
    Validation,
    #[error("databese error")]
    Database,
    #[error("rabbitmq error")]
    RabbitMq,
    #[error("unauthorized")]
    Unauthorized,
    #[error("internal error")]
    Internal,
    #[error("nothing to update")]
    NoUpdates,
}

#[derive(Debug)]
pub struct AppError {
    pub kind: AppErrorKind,
    pub source: anyhow::Error,
}

impl AppError {
    pub fn validation(msg: &'static str) -> Self {
        Self {
            kind: AppErrorKind::Validation,
            source: anyhow::anyhow!(msg),
        }
    }

    pub fn no_updated() -> Self {
        Self {
            kind: AppErrorKind::NoUpdates,
            source: anyhow::anyhow!("no fields provided to update"),
        }
    }

    pub fn pool_context() -> &'static str {
        "Failed to get a db connection"
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
    details: Option<String>,
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        AppError {
            kind: AppErrorKind::Internal,
            source: value,
        }
    }
}

impl From<diesel::result::Error> for AppError {
    fn from(value: diesel::result::Error) -> Self {
        let kind = match value {
            diesel::result::Error::NotFound => AppErrorKind::NotFound,
            _ => AppErrorKind::Database,
        };

        AppError {
            kind,
            source: value.into(),
        }
    }
}

impl<E> From<bb8::RunError<E>> for AppError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: bb8::RunError<E>) -> Self {
        AppError {
            kind: AppErrorKind::Database,
            source: err.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.kind {
            AppErrorKind::NotFound => StatusCode::NOT_FOUND,
            AppErrorKind::Validation => StatusCode::BAD_REQUEST,
            AppErrorKind::Database => StatusCode::INTERNAL_SERVER_ERROR,
            AppErrorKind::RabbitMq => StatusCode::SERVICE_UNAVAILABLE,
            AppErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
            AppErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            AppErrorKind::NoUpdates => StatusCode::BAD_REQUEST,
        };

        let body = format!("{}", self.source);

        (status, body).into_response()
    }
}
