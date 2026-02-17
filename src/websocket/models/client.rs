// WebSocket client model

use crate::websocket::models::message::Message;
use tokio::sync::mpsc;

/// Represents a connected client
#[allow(dead_code)]
pub struct Client {
    pub username: String,
    pub sender: mpsc::UnboundedSender<Message>,
}

impl Client {
    pub fn new(username: String, sender: mpsc::UnboundedSender<Message>) -> Self {
        Self { username, sender }
    }
}
