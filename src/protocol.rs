use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientEvent {
    Join { username: String },
    Send { message: String },
    Leave,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerEvent {
    Chat { user: String, message: String },
    Error { reason: String },
}