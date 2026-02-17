// Custom error types for WebSocket service

use thiserror::Error;

/// WebSocket service error types
#[derive(Error, Debug)]
pub enum WebSocketError {
    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Username already exists: {0}")]
    UsernameExists(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, WebSocketError>;
