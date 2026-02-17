// WebSocket message model

use serde::{Deserialize, Serialize};

/// Message types for the chat protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Message {
    /// Join the chat room with a username
    Join { username: String },
    /// Leave the chat room
    Leave,
    /// Send a message to all users in the room
    Send { content: String },
    /// Broadcast message from server to clients
    Broadcast { username: String, content: String },
    /// Error message from server
    Error { message: String },
}

impl Message {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
