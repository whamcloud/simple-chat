use protocol::ServerMessage;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: usize = 256;

/// A chat room that manages connected users and message broadcasting.
pub struct Room {
    state: Mutex<RoomState>,
}

struct RoomState {
    users: HashMap<String, broadcast::Sender<ServerMessage>>,
}

impl Default for Room {
    fn default() -> Self {
        Self::new()
    }
}

impl Room {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RoomState {
                users: HashMap::new(),
            }),
        }
    }

    /// Add a user to the room. Returns a receiver for incoming messages,
    /// or an error string if the username is taken.
    pub fn join(&self, username: &str) -> Result<broadcast::Receiver<ServerMessage>, String> {
        let mut state = self.state.lock().unwrap();

        if state.users.contains_key(username) {
            return Err(format!("Username '{username}' is already taken"));
        }

        let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY);
        state.users.insert(username.to_string(), tx);

        // Notify all other users
        let join_msg = ServerMessage::UserJoined {
            username: username.to_string(),
        };
        for (name, sender) in &state.users {
            if name != username {
                let _ = sender.send(join_msg.clone());
            }
        }

        Ok(rx)
    }

    /// Remove a user from the room and notify others.
    pub fn leave(&self, username: &str) {
        let mut state = self.state.lock().unwrap();
        state.users.remove(username);

        let leave_msg = ServerMessage::UserLeft {
            username: username.to_string(),
        };
        for sender in state.users.values() {
            let _ = sender.send(leave_msg.clone());
        }
    }

    /// Broadcast a chat message to all users except the sender.
    pub fn broadcast(&self, sender_username: &str, content: String) {
        let state = self.state.lock().unwrap();

        let chat_msg = ServerMessage::Chat {
            username: sender_username.to_string(),
            content,
        };

        for (name, sender) in &state.users {
            if name != sender_username {
                let _ = sender.send(chat_msg.clone());
            }
        }
    }

    /// Returns the number of connected users.
    #[cfg(test)]
    pub fn user_count(&self) -> usize {
        self.state.lock().unwrap().users.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_and_leave() {
        let room = Room::new();
        assert_eq!(room.user_count(), 0);

        let _rx = room.join("alice").unwrap();
        assert_eq!(room.user_count(), 1);

        let _rx2 = room.join("bob").unwrap();
        assert_eq!(room.user_count(), 2);

        room.leave("alice");
        assert_eq!(room.user_count(), 1);

        room.leave("bob");
        assert_eq!(room.user_count(), 0);
    }

    #[test]
    fn test_duplicate_username_rejected() {
        let room = Room::new();
        let _rx = room.join("alice").unwrap();
        let result = room.join("alice");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already taken"));
    }

    #[tokio::test]
    async fn test_broadcast_excludes_sender() {
        let room = Room::new();
        let _rx_alice = room.join("alice").unwrap();
        let mut rx_bob = room.join("bob").unwrap();

        room.broadcast("alice", "hello".to_string());

        let msg = rx_bob.recv().await.unwrap();
        match msg {
            ServerMessage::Chat { username, content } => {
                assert_eq!(username, "alice");
                assert_eq!(content, "hello");
            }
            _ => panic!("Expected Chat message"),
        }
    }

    #[tokio::test]
    async fn test_join_notification() {
        let room = Room::new();
        let mut rx_alice = room.join("alice").unwrap();

        let _rx_bob = room.join("bob").unwrap();

        let msg = rx_alice.recv().await.unwrap();
        match msg {
            ServerMessage::UserJoined { username } => {
                assert_eq!(username, "bob");
            }
            _ => panic!("Expected UserJoined message"),
        }
    }

    #[tokio::test]
    async fn test_leave_notification() {
        let room = Room::new();
        let _rx_alice = room.join("alice").unwrap();
        let mut rx_bob = room.join("bob").unwrap();

        room.leave("alice");

        let msg = rx_bob.recv().await.unwrap();
        match msg {
            ServerMessage::UserLeft { username } => {
                assert_eq!(username, "alice");
            }
            _ => panic!("Expected UserLeft message"),
        }
    }
}
