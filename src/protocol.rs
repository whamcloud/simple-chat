use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Join { username: String },
    Send { message: String },
    Leave,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    Message { username: String, message: String },
    Error { reason: String },
}