use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

use crate::protocol::ServerMessage;

/// Messages that can be sent to the Room actor
#[derive(Debug)]
pub enum RoomCommand {
    /// Join the room with a username
    Join {
        username: String,
        tx: mpsc::Sender<ServerMessage>,
        response: oneshot::Sender<Result<(), String>>,
    },
    /// Leave the room
    Leave { username: String },
    /// Broadcast a message to all users except the sender
    Broadcast { from: String, message: String },
}

/// Handle to interact with the Room actor
#[derive(Clone)]
pub struct Room {
    tx: mpsc::Sender<RoomCommand>,
}

impl Default for Room {
    fn default() -> Self {
        Self::new()
    }
}

impl Room {
    /// Create a new room and spawn its actor task
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let actor = RoomActor::new(rx);
        tokio::spawn(actor.run());
        Room { tx }
    }

    /// Attempt to join the room
    pub async fn join(
        &self,
        username: String,
        tx: mpsc::Sender<ServerMessage>,
    ) -> Result<(), String> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(RoomCommand::Join {
                username,
                tx,
                response: response_tx,
            })
            .await
            .map_err(|_| "Room actor died".to_string())?;

        response_rx
            .await
            .map_err(|_| "Room actor died".to_string())?
    }

    /// Leave the room
    pub async fn leave(&self, username: String) -> Result<(), String> {
        self.tx
            .send(RoomCommand::Leave { username })
            .await
            .map_err(|_| "Room actor died".to_string())
    }

    /// Broadcast a message to all users except sender
    pub async fn broadcast(&self, from: String, message: String) -> Result<(), String> {
        self.tx
            .send(RoomCommand::Broadcast { from, message })
            .await
            .map_err(|_| "Room actor died".to_string())
    }
}

/// The actual Room actor that manages users
struct RoomActor {
    rx: mpsc::Receiver<RoomCommand>,
    users: HashMap<String, mpsc::Sender<ServerMessage>>,
}

impl RoomActor {
    fn new(rx: mpsc::Receiver<RoomCommand>) -> Self {
        RoomActor {
            rx,
            users: HashMap::new(),
        }
    }

    async fn run(mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                RoomCommand::Join {
                    username,
                    tx,
                    response,
                } => {
                    let result = if self.users.contains_key(&username) {
                        Err(format!("Username '{}' is already taken", username))
                    } else {
                        // Notify all existing users that someone joined
                        let join_msg = ServerMessage::Joined(username.clone());
                        for user_tx in self.users.values() {
                            let _ = user_tx.send(join_msg.clone()).await;
                        }

                        // Add the new user
                        self.users.insert(username, tx);
                        Ok(())
                    };
                    let _ = response.send(result);
                }
                RoomCommand::Leave { username } => {
                    if self.users.remove(&username).is_some() {
                        // Notify all remaining users
                        let leave_msg = ServerMessage::Left(username);
                        for user_tx in self.users.values() {
                            let _ = user_tx.send(leave_msg.clone()).await;
                        }
                    }
                }
                RoomCommand::Broadcast { from, message } => {
                    let msg = ServerMessage::Message {
                        from: from.clone(),
                        text: message,
                    };
                    // Send to all users except the sender
                    for (username, user_tx) in &self.users {
                        if username != &from {
                            let _ = user_tx.send(msg.clone()).await;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_join_success() {
        let room = Room::new();
        let (tx, _rx) = mpsc::channel(10);

        let result = room.join("alice".to_string(), tx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_join_duplicate_username() {
        let room = Room::new();
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);

        // First join should succeed
        let result1 = room.join("alice".to_string(), tx1).await;
        assert!(result1.is_ok());

        // Second join with same username should fail
        let result2 = room.join("alice".to_string(), tx2).await;
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("already taken"));
    }

    #[tokio::test]
    async fn test_broadcast_excludes_sender() {
        let room = Room::new();

        // Create three users
        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);
        let (tx3, mut rx3) = mpsc::channel(10);

        room.join("alice".to_string(), tx1).await.unwrap();
        room.join("bob".to_string(), tx2).await.unwrap();
        room.join("charlie".to_string(), tx3).await.unwrap();

        // Give time for join notifications
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Drain all join notifications
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        while rx3.try_recv().is_ok() {}

        // Alice broadcasts a message
        room.broadcast("alice".to_string(), "hello everyone".to_string())
            .await
            .unwrap();

        // Give messages time to arrive
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Bob and Charlie should receive the broadcast
        let bob_msg = rx2.try_recv();
        let charlie_msg = rx3.try_recv();

        assert!(bob_msg.is_ok());
        assert!(charlie_msg.is_ok());

        if let Ok(ServerMessage::Message { from, text }) = bob_msg {
            assert_eq!(from, "alice");
            assert_eq!(text, "hello everyone");
        } else {
            panic!("Expected Message, got {:?}", bob_msg);
        }

        // Alice should NOT receive her own message
        let alice_msg = rx1.try_recv();
        assert!(alice_msg.is_err());
    }

    #[tokio::test]
    async fn test_join_notification() {
        let room = Room::new();

        // Alice joins first
        let (tx1, mut rx1) = mpsc::channel(10);
        room.join("alice".to_string(), tx1).await.unwrap();

        // Bob joins - Alice should be notified
        let (tx2, _rx2) = mpsc::channel(10);
        room.join("bob".to_string(), tx2).await.unwrap();

        // Give messages time to arrive
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Alice should receive notification
        let msg = rx1.try_recv().unwrap();
        assert_eq!(msg, ServerMessage::Joined("bob".to_string()));
    }

    #[tokio::test]
    async fn test_leave_notification() {
        let room = Room::new();

        // Two users join
        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);

        room.join("alice".to_string(), tx1).await.unwrap();
        room.join("bob".to_string(), tx2).await.unwrap();

        // Clear any join notifications
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let _ = rx1.try_recv();

        // Bob leaves
        room.leave("bob".to_string()).await.unwrap();

        // Give messages time to arrive
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Alice should be notified
        let msg = rx1.try_recv().unwrap();
        assert_eq!(msg, ServerMessage::Left("bob".to_string()));
    }

    #[tokio::test]
    async fn test_user_can_rejoin_after_leaving() {
        let room = Room::new();

        let (tx1, _rx1) = mpsc::channel(10);
        room.join("alice".to_string(), tx1).await.unwrap();

        room.leave("alice".to_string()).await.unwrap();

        // Alice should be able to rejoin
        let (tx2, _rx2) = mpsc::channel(10);
        let result = room.join("alice".to_string(), tx2).await;
        assert!(result.is_ok());
    }
}
