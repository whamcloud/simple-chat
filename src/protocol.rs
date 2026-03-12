use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientEvent {
    Join { username: String },
    Send { message: String },
    Leave,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerEvent {
    Message { username: String, message: String },
    Error { reason: String },
}