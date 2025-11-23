use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Join { username: String },
    Leave { username: String },
    Chat { username: String, message: String },
    Error { message: String },
    System { message: String },
}

pub struct ChatServer {
    users: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
    tx: broadcast::Sender<String>,
}

impl ChatServer {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            tx,
        }
    }

    pub async fn run(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("Server listening on {}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let users = Arc::clone(&self.users);
            let tx = self.tx.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_client(stream, users, tx).await {
                    eprintln!("Error handling client: {}", e);
                }
            });
        }
    }

    async fn handle_client(
        stream: TcpStream,
        users: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
        tx: broadcast::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        #[allow(unused_assignments)]
        let mut username: Option<String> = None;
        #[allow(unused_assignments)]
        let mut rx: Option<broadcast::Receiver<String>> = None;

        reader.read_line(&mut line).await?;
        let msg: Message = serde_json::from_str(line.trim())?;

        match msg {
            Message::Join { username: user } => {
                let mut users_guard = users.write().await;
                if users_guard.contains_key(&user) {
                    let error = Message::Error {
                        message: format!("Username '{}' is already taken", user),
                    };
                    let error_json = serde_json::to_string(&error)?;
                    writer.write_all(error_json.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    return Ok(());
                }

                let user_tx = tx.clone();
                let user_rx = user_tx.subscribe();
                users_guard.insert(user.clone(), user_tx);
                username = Some(user.clone());

                // notify others
                let join_msg = Message::System {
                    message: format!("{} joined the chat", user),
                };
                let join_json = serde_json::to_string(&join_msg)?;
                let _ = tx.send(join_json.clone());

                let confirm = Message::System {
                    message: "You joined the chat".to_string(),
                };
                let confirm_json = serde_json::to_string(&confirm)?;
                writer.write_all(confirm_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;

                rx = Some(user_rx);
            }
            _ => {
                let error = Message::Error {
                    message: "First message must be a Join message".to_string(),
                };
                let error_json = serde_json::to_string(&error)?;
                writer.write_all(error_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                return Ok(());
            }
        }

        let username = username.unwrap();
        let mut rx = rx.unwrap();

        let mut writer_clone = writer;
        let username_clone = username.clone();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                // filter out messages from self
                if let Ok(Message::Chat {
                    username: sender, ..
                }) = serde_json::from_str::<Message>(&msg)
                {
                    if sender == username_clone {
                        continue;
                    }
                }

                if writer_clone.write_all(msg.as_bytes()).await.is_err() {
                    break;
                }
                if writer_clone.write_all(b"\n").await.is_err() {
                    break;
                }
                if writer_clone.flush().await.is_err() {
                    break;
                }
            }
        });

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    let msg: Message = match serde_json::from_str(line.trim()) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };

                    match msg {
                        Message::Chat { message, .. } => {
                            let chat_msg = Message::Chat {
                                username: username.clone(),
                                message,
                            };
                            let chat_json = serde_json::to_string(&chat_msg)?;
                            let _ = tx.send(chat_json);
                        }
                        Message::Leave { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }

        // cleanup user
        let mut users_guard = users.write().await;
        users_guard.remove(&username);
        drop(users_guard);

        let leave_msg = Message::System {
            message: format!("{} left the chat", username),
        };
        let leave_json = serde_json::to_string(&leave_msg)?;
        let _ = tx.send(leave_json);

        Ok(())
    }
}

impl Default for ChatServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn test_server_accepts_connections() {
        let server = ChatServer::new();
        let addr = "127.0.0.1:0";
        let listener = TcpListener::bind(addr).await.unwrap();
        let local_addr = listener.local_addr().unwrap();
        drop(listener);

        tokio::spawn(async move {
            server.run(&local_addr.to_string()).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let _stream = TcpStream::connect(local_addr).await;
        assert!(_stream.is_ok());
    }

    #[tokio::test]
    async fn test_username_uniqueness() {
        let server = ChatServer::new();
        let addr = "127.0.0.1:0";
        let listener = TcpListener::bind(addr).await.unwrap();
        let local_addr = listener.local_addr().unwrap();
        drop(listener);

        tokio::spawn(async move {
            server.run(&local_addr.to_string()).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut stream1 = TcpStream::connect(local_addr).await.unwrap();
        let join1 = Message::Join {
            username: "alice".to_string(),
        };
        let join1_json = serde_json::to_string(&join1).unwrap();
        stream1.write_all(join1_json.as_bytes()).await.unwrap();
        stream1.write_all(b"\n").await.unwrap();
        stream1.flush().await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut stream2 = TcpStream::connect(local_addr).await.unwrap();
        let join2 = Message::Join {
            username: "alice".to_string(),
        };
        let join2_json = serde_json::to_string(&join2).unwrap();
        stream2.write_all(join2_json.as_bytes()).await.unwrap();
        stream2.write_all(b"\n").await.unwrap();
        stream2.flush().await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut reader = BufReader::new(&mut stream2);
        let mut response = String::new();
        reader.read_line(&mut response).await.unwrap();
        let error_msg: Message = serde_json::from_str(response.trim()).unwrap();

        match error_msg {
            Message::Error { .. } => {}
            _ => panic!("Expected error message for duplicate username"),
        }
    }
}
