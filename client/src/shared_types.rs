use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientMessage {
    Join {
        username: String,
    },
    Leave,
    SendMessage {
        content: String,
    },
    AckReceived {
        message_id: String,
        intended_recipient: String, // 👈 NEW: who sent the original message
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: u64,
    pub delivered_to: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome {
        username: String,
    },
    Joined {
        username: String,
    },
    Left {
        username: String,
    },
    NewMessage(ChatMessage),
    DeliveryReceipt {
        message_id: String,
        received_by: String,
    },
    Error {
        message: String,
    },
}
