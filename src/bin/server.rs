use anyhow::Result;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use simple_chat::{ChatResult, Message};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{timeout, Duration};
use tokio_util::codec::{FramedRead, FramedWrite};

#[derive(Parser)]
#[command(author, version, about = "Simple Chat Server", long_about = None)]
struct ServerArgs {
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    #[arg(long, default_value_t = 100)]
    max_users: usize,
}

#[derive(Clone)]
struct UserConnection {
    #[allow(dead_code)]
    sender: broadcast::Sender<Message>,
    #[allow(dead_code)]
    addr: SocketAddr,
}

type UserManager = Arc<RwLock<HashMap<String, UserConnection>>>;

#[derive(Clone)]
struct ChatServer {
    users: UserManager,
    message_tx: broadcast::Sender<Message>,
    max_users: usize,
}

impl ChatServer {
    fn new(max_users: usize) -> Self {
        let (message_tx, _) = broadcast::channel(1000);

        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            max_users,
        }
    }

    async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        info!("New connection from {}", addr);

        let (reader, writer) = stream.into_split();
        let mut framed_read = FramedRead::new(reader, simple_chat::codec::MessageCodec);
        let mut framed_write = FramedWrite::new(writer, simple_chat::codec::MessageCodec);

        let mut username: Option<String> = None;
        let mut message_rx = self.message_tx.subscribe();

        // Set connection timeout for initial handshake
        let handshake_timeout = Duration::from_secs(30);

        loop {
            tokio::select! {
                // Handle incoming messages from client
                result = timeout(handshake_timeout, framed_read.next()) => {

                    match result {
                        Ok(Some(Ok(message))) => {
                            match self.process_client_message(message, &mut username, addr, &mut framed_write).await {
                                Ok(should_continue) => {
                                    if !should_continue {
                                        break;
                                    }
                                },
                                Err(e) => {
                                    error!("Error processing message from {}: {}", addr, e);
                                    let error_msg = Message::Error {
                                        message: format!("Server error: {}", e)
                                    };
                                    let _ = framed_write.send(error_msg).await;
                                    break;
                                }
                            }
                        },
                        Ok(Some(Err(e))) => {
                            error!("Decode error from {}: {}", addr, e);
                            break;
                        },
                        Ok(None) => {
                            info!("Client {} disconnected", addr);
                            break;
                        },
                        Err(_) => {
                            warn!("Connection timeout from {}", addr);
                            break;
                        }
                    }
                },

                // Handle broadcast messages to send to client
                result = message_rx.recv() => {
                    match result {
                        Ok(message) => {
                            // Only send messages to authenticated users
                            if username.is_some() {
                                if let Err(e) = framed_write.send(message).await {
                                    error!("Failed to send message to {}: {}", addr, e);
                                    break;
                                }
                            }
                        },
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Message channel closed, shutting down connection {}", addr);
                            break;
                        },
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            warn!("Message channel lagged for {}", addr);
                            continue;
                        }
                    }
                }
            }
        }

        // Cleanup on disconnect
        if let Some(ref user) = username {
            self.remove_user(user).await;
            info!("User '{}' disconnected from {}", user, addr);

            // Broadcast user left message
            let leave_message = Message::UserLeft {
                username: user.clone(),
            };
            let _ = self.message_tx.send(leave_message);
        }

        Ok(())
    }

    async fn process_client_message(
        &self,
        message: Message,
        username: &mut Option<String>,
        addr: SocketAddr,
        writer: &mut FramedWrite<tokio::net::tcp::OwnedWriteHalf, simple_chat::codec::MessageCodec>,
    ) -> Result<bool> {
        match message {
            Message::Join {
                username: requested_username,
            } => {
                if username.is_some() {
                    let error_msg = Message::Error {
                        message: "Already authenticated".to_string(),
                    };
                    writer.send(error_msg).await?;
                    return Ok(true);
                }

                match self.add_user(&requested_username, addr).await {
                    Ok(()) => {
                        *username = Some(requested_username.clone());
                        info!("User '{}' joined from {}", requested_username, addr);

                        // Send confirmation to the joining user
                        let join_confirm = Message::UserJoined {
                            username: requested_username.clone(),
                        };
                        writer.send(join_confirm).await?;

                        // Broadcast to all other users
                        let broadcast_message = Message::UserJoined {
                            username: requested_username,
                        };
                        let _ = self.message_tx.send(broadcast_message);

                        Ok(true)
                    }
                    Err(e) => {
                        let error_msg = Message::Error {
                            message: e.to_string(),
                        };
                        writer.send(error_msg).await?;
                        Ok(false) // Disconnect user
                    }
                }
            }

            Message::Leave { .. } => {
                // Client requesting to leave
                Ok(false) // This will trigger cleanup in the main loop
            }

            Message::ChatMessage {
                username: msg_username,
                content,
            } => {
                if let Some(ref current_user) = username {
                    if msg_username != *current_user {
                        let error_msg = Message::Error {
                            message: "Username mismatch".to_string(),
                        };
                        writer.send(error_msg).await?;
                        return Ok(true);
                    }

                    // Broadcast the chat message
                    let chat_message = Message::ChatMessage {
                        username: current_user.clone(),
                        content,
                    };
                    let _ = self.message_tx.send(chat_message);
                    Ok(true)
                } else {
                    let error_msg = Message::Error {
                        message: "Not authenticated".to_string(),
                    };
                    writer.send(error_msg).await?;
                    Ok(true)
                }
            }

            _ => {
                let error_msg = Message::Error {
                    message: "Invalid message type".to_string(),
                };
                writer.send(error_msg).await?;
                Ok(true)
            }
        }
    }

    async fn add_user(&self, username: &str, addr: SocketAddr) -> ChatResult<()> {
        let mut users = self.users.write().await;

        if users.len() >= self.max_users {
            return Err(simple_chat::ChatError::InvalidUsername(
                "Server is full".to_string(),
            ));
        }

        if users.contains_key(username) {
            return Err(simple_chat::ChatError::InvalidUsername(
                "Username already taken".to_string(),
            ));
        }

        let (sender, _) = broadcast::channel(100);
        let connection = UserConnection { sender, addr };
        users.insert(username.to_string(), connection);

        Ok(())
    }

    async fn remove_user(&self, username: &str) {
        let mut users = self.users.write().await;
        users.remove(username);
    }

    async fn get_user_count(&self) -> usize {
        let users = self.users.read().await;
        users.len()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = ServerArgs::parse();
    let addr = format!("{}:{}", args.host, args.port);

    info!("Starting chat server on {}", addr);
    info!("Maximum users: {}", args.max_users);

    // Create the chat server
    let server = ChatServer::new(args.max_users);

    // Bind TCP listener
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on {}", addr);

    // Setup graceful shutdown
    let server_clone = server.clone();
    let mut shutdown_task = tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(_) => {
                info!("Received SIGINT, shutting down gracefully...");
                let user_count = server_clone.get_user_count().await;
                if user_count > 0 {
                    info!("Disconnecting {} active users...", user_count);
                    // The broadcast channel will be closed when server is dropped
                }
            }
            Err(e) => {
                error!("Failed to listen for shutdown signal: {}", e);
            }
        }
    });

    // Main server loop
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        let server_clone = server.clone();
                        tokio::spawn(async move {
                            if let Err(e) = server_clone.handle_connection(stream, addr).await {
                                error!("Connection handler error for {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                        continue;
                    }
                }
            }

            _ = &mut shutdown_task => {
                info!("Shutdown signal received, stopping server...");
                break;
            }
        }
    }

    info!("Chat server shutdown complete");
    Ok(())
}
