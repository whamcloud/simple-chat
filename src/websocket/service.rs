// WebSocket service implementation with business logic

use crate::websocket::error::Result;
use crate::websocket::models::client::Client;
use crate::websocket::models::message::Message;
use crate::websocket::repository::ChatRepository;

/// WebSocket service for handling chat operations
#[derive(Clone)]
pub struct ChatService {
    pub repository: ChatRepository,
}

impl ChatService {
    pub fn new(repository: ChatRepository) -> Self {
        Self { repository }
    }

    /// Handle a user joining the chat
    pub async fn handle_join(&self, username: String, client: Client) -> Result<()> {
        self.repository.add_client(username, client).await
    }

    /// Handle a user leaving the chat
    pub async fn handle_leave(&self, username: &str) {
        self.repository.remove_client(username).await;
    }

    /// Handle a user sending a message
    pub async fn handle_send(&self, username: &str, content: String) {
        let broadcast_msg = Message::Broadcast {
            username: username.to_string(),
            content,
        };
        self.repository.broadcast(username, broadcast_msg).await;
    }
}
