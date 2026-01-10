use chatserver::{ClientMessage, ServerMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

type ClientMap = Arc<RwLock<HashMap<String, tokio::sync::mpsc::UnboundedSender<ServerMessage>>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port 8080");

    info!("Chat server listening on {}", addr);

    let clients: ClientMap = Arc::new(RwLock::new(HashMap::new()));

    loop {
        let (socket, peer_addr) = listener
            .accept()
            .await
            .expect("Failed to accept connection");

        info!("New connection from {}", peer_addr);

        let clients = Arc::clone(&clients);
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, clients).await {
                warn!("Client error: {}", e);
            }
        });
    }
}

async fn handle_client(
    socket: TcpStream,
    clients: ClientMap,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);

    // Read join message from client
    let mut line = String::new();
    let n = reader.read_line(&mut line).await?;

    if n == 0 {
        return Ok(());
    }

    let join_msg: ClientMessage = serde_json::from_str(&line)?;
    let username = match join_msg {
        ClientMessage::Join { username } => {
            if username.is_empty() {
                writer
                    .write_all(
                        ServerMessage::Error {
                            message: "Username cannot be empty".to_string(),
                        }
                        .to_bytes()?
                        .as_ref(),
                    )
                    .await?;
                return Ok(());
            }
            username
        }
        _ => {
            writer
                .write_all(
                    ServerMessage::Error {
                        message: "First message must be a join message".to_string(),
                    }
                    .to_bytes()?
                    .as_ref(),
                )
                .await?;
            return Ok(());
        }
    };

    // Check if username already exists
    {
        let map = clients.read().await;
        if map.contains_key(&username) {
            writer
                .write_all(
                    ServerMessage::Error {
                        message: format!("Username '{}' is already taken", username),
                    }
                    .to_bytes()?
                    .as_ref(),
                )
                .await?;
            return Ok(());
        }
    }

    info!("User '{}' joined the chat", username);

    // Create channel for this client
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Add client to map
    {
        let mut map = clients.write().await;
        map.insert(username.clone(), tx);
    }

    // Notify others that user joined
    broadcast_message(
        &clients,
        ServerMessage::UserJoined {
            username: username.clone(),
        },
        Some(&username),
    )
    .await;

    // Spawn task to handle incoming messages from this client
    let username_clone = username.clone();
    let clients_clone = Arc::clone(&clients);
    let receive_task = tokio::spawn(async move {
        let mut reader = reader;
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => match serde_json::from_str::<ClientMessage>(&line) {
                    Ok(ClientMessage::Send { content }) => {
                        if !content.trim().is_empty() {
                            broadcast_message(
                                &clients_clone,
                                ServerMessage::Chat {
                                    username: username_clone.clone(),
                                    content,
                                },
                                Some(&username_clone),
                            )
                            .await;
                        }
                    }
                    Ok(ClientMessage::Leave) => {
                        break;
                    }
                    Ok(ClientMessage::Join { .. }) => {
                        warn!("Unexpected join message from already-joined client");
                    }
                    Err(e) => {
                        error!("Failed to parse message: {}", e);
                    }
                },
                Err(e) => {
                    error!("Error reading from client: {}", e);
                    break;
                }
            }
        }
    });

    // Send messages to this client
    while let Some(msg) = rx.recv().await {
        if let Ok(bytes) = msg.to_bytes() {
            if writer.write_all(&bytes).await.is_err() {
                break;
            }
        }
    }

    receive_task.abort();

    // Remove client from map
    {
        let mut map = clients.write().await;
        map.remove(&username);
    }

    // Notify others that user left
    broadcast_message(
        &clients,
        ServerMessage::UserLeft {
            username: username.clone(),
        },
        None,
    )
    .await;

    info!("User '{}' disconnected", username);
    Ok(())
}

/// Broadcast a message to all connected clients, optionally excluding one
async fn broadcast_message(clients: &ClientMap, message: ServerMessage, exclude: Option<&str>) {
    let map = clients.read().await;
    for (username, tx) in map.iter() {
        info!("User '{}' sent '{:?}'", username, tx);
        if let Some(ex) = exclude {
            if username == ex {
                continue;
            }
        }
        // Ignore send errors (client may have disconnected)

        info!("Broadcasting message '{}'", message.clone());
        let _ = tx.send(message.clone());
    }
}
