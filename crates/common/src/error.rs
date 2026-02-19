use thiserror::Error;

use crate::RejectionReason;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Exchange API error: {0}")]
    Exchange(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Order rejected: {reason}")]
    OrderRejected { reason: RejectionReason },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
