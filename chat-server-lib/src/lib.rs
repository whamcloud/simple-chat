use anyhow::Result;
use chat_common::{BroadcastMessage, MAX_USERNAME_LEN};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{info, warn};

const BROADCAST_CAPACITY: usize = 100;

pub struct ServerState {
    pub active_users: DashMap<String, ()>,
    pub tx: broadcast::Sender<BroadcastMessage>,
}

impl ServerState {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            active_users: DashMap::new(),
            tx,
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn handle_connection(socket: TcpStream, state: Arc<ServerState>) -> Result<()> {
    let addr = socket
        .peer_addr()
        .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());

    let mut lines = Framed::new(socket, LinesCodec::new());

    lines.send("Welcome! Please enter your username:").await?;

    let username = match lines.next().await {
        Some(Ok(line)) => line.trim().to_string(),
        _ => return Ok(()),
    };

    if username.is_empty() || username.len() > MAX_USERNAME_LEN {
        lines.send("Invalid username. Bye.").await?;
        return Ok(());
    }

    if state.active_users.contains_key(&username) {
        lines.send("Username already taken. Bye.").await?;
        return Ok(());
    }
    state.active_users.insert(username.clone(), ());

    info!("User '{}' joined from {}.", username, addr);
    lines
        .send(format!("Welcome {}, you are now connected.", username))
        .await?;

    let _ = state.tx.send(BroadcastMessage::system_msg(format!(
        "{} has joined the room.",
        username
    )));

    let mut rx = state.tx.subscribe();
    let my_username = username.clone();

    loop {
        tokio::select! {
            result = lines.next() => {
                match result {
                    Some(Ok(msg)) => {
                        let msg = msg.trim();
                        if msg == "leave" {
                            break;
                        } else if let Some(content) = msg.strip_prefix("send ") {
                             let _ = state.tx.send(BroadcastMessage::user_msg(my_username.clone(), content));
                        } else {
                            lines.send("Usage: 'send <MSG>' or 'leave'.").await?;
                        }
                    }
                    Some(Err(e)) => {
                        warn!("Error reading from {}: {}", my_username, e);
                        break;
                    }
                    None => break,
                }
            }

            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if msg.sender_id!= my_username {
                            let text = if msg.is_system {
                                format!("System: {}", msg.content)
                            } else {
                                format!("{}: {}", msg.sender_id, msg.content)
                            };

                            if let Err(e) = lines.send(text).await {
                                warn!("Failed to send to {}: {}", my_username, e);
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        warn!("Client {} lagged by {} messages", my_username, count);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    state.active_users.remove(&my_username);
    let _ = state.tx.send(BroadcastMessage::system_msg(format!(
        "{} has left the room.",
        my_username
    )));
    info!("User '{}' disconnected.", my_username);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_initialization() {
        let state = ServerState::new();
        assert!(state.active_users.is_empty());
        assert!(state.tx.receiver_count() == 0);
    }

    #[tokio::test]
    async fn test_user_registration() {
        let state = ServerState::new();
        state.active_users.insert("Bob".to_string(), ());

        assert!(state.active_users.contains_key("Bob"));
        assert!(!state.active_users.contains_key("Alice"));
    }
}
