// Repository for managing connected clients

use crate::websocket::error::{Result, WebSocketError};
use crate::websocket::models::client::Client;
use crate::websocket::models::message::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Repository for managing chat room state
#[derive(Clone)]
pub struct ChatRepository {
    clients: Arc<RwLock<HashMap<String, Client>>>,
}

impl ChatRepository {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new client to the repository
    pub async fn add_client(&self, username: String, client: Client) -> Result<()> {
        let mut clients = self.clients.write().await;
        if clients.contains_key(&username) {
            return Err(WebSocketError::UsernameExists(username));
        }
        clients.insert(username, client);
        Ok(())
    }

    /// Remove a client from the repository
    pub async fn remove_client(&self, username: &str) {
        let mut clients = self.clients.write().await;
        clients.remove(username);
    }

    /// Broadcast a message to all clients except the sender
    pub async fn broadcast(&self, sender_username: &str, message: Message) {
        let clients = self.clients.read().await;
        for (username, client) in clients.iter() {
            if username != sender_username {
                let _ = client.sender.send(message.clone());
            }
        }
    }

    /// Get the count of connected clients
    #[allow(dead_code)]
    pub async fn client_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }
}

impl Default for ChatRepository {
    fn default() -> Self {
        Self::new()
    }
}
