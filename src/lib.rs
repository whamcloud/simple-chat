use serde::{Deserialize, Serialize};
use std::fmt;

/// Message types that can be sent between client and server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    #[serde(rename = "chat")]
    Chat { username: String, content: String },
    #[serde(rename = "user_joined")]
    UserJoined { username: String },
    #[serde(rename = "user_left")]
    UserLeft { username: String },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Message types that can be sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ClientMessage {
    #[serde(rename = "join")]
    Join { username: String },
    #[serde(rename = "send")]
    Send { content: String },
    #[serde(rename = "leave")]
    Leave,
}

impl ServerMessage {
    /// Serialize a ServerMessage to JSON with newline terminator
    pub fn to_bytes(&self) -> Result<bytes::Bytes, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(format!("{}\n", json).into())
    }

    /// Deserialize a ServerMessage from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::str::from_utf8(bytes)?;
        Ok(serde_json::from_str(text)?)
    }
}

impl ClientMessage {
    /// Serialize a ClientMessage to JSON with newline terminator
    pub fn to_bytes(&self) -> Result<bytes::Bytes, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(format!("{}\n", json).into())
    }

    /// Deserialize a ClientMessage from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::str::from_utf8(bytes)?;
        Ok(serde_json::from_str(text)?)
    }
}

impl fmt::Display for ServerMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerMessage::Chat { username, content } => {
                write!(f, "{}: {}", username, content)
            }
            ServerMessage::UserJoined { username } => {
                write!(f, ">> {} joined the chat", username)
            }
            ServerMessage::UserLeft { username } => {
                write!(f, ">> {} left the chat", username)
            }
            ServerMessage::Error { message } => {
                write!(f, "ERROR: {}", message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Chat {
            username: "alice".to_string(),
            content: "hello".to_string(),
        };
        let bytes = msg.to_bytes().unwrap();
        let deserialized = ServerMessage::from_bytes(&bytes).unwrap();
        match deserialized {
            ServerMessage::Chat { username, content } => {
                assert_eq!(username, "alice");
                assert_eq!(content, "hello");
            }
            _ => panic!("Expected Chat message"),
        }
    }

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::Send {
            content: "hello".to_string(),
        };
        let bytes = msg.to_bytes().unwrap();
        let deserialized = ClientMessage::from_bytes(&bytes).unwrap();
        match deserialized {
            ClientMessage::Send { content } => {
                assert_eq!(content, "hello");
            }
            _ => panic!("Expected Send message"),
        }
    }

    #[test]
    fn test_server_message_display() {
        let msg = ServerMessage::Chat {
            username: "bob".to_string(),
            content: "test message".to_string(),
        };
        assert_eq!(msg.to_string(), "bob: test message");
    }
}
