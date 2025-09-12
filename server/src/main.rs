use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, broadcast};
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

mod shared_types;
use shared_types::*;
use tracing_subscriber::EnvFilter;

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    WsMessage,
>;
type SharedUsers = Arc<Mutex<HashMap<String, Arc<Mutex<WsSink>>>>>;

struct ChatServer {
    users: SharedUsers,
    tx: broadcast::Sender<ServerMessage>,
    user_store: sled::Db,
}

impl ChatServer {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, _) = broadcast::channel(100);
        let user_store = sled::open("./chat_data")?;

        let mut cleaned = 0;
        for item in user_store.scan_prefix(b"user:") {
            let (key, _) = item?;
            user_store.remove(key)?;
            cleaned += 1;
        }
        user_store.flush()?;

        let users: SharedUsers = Arc::new(Mutex::new(HashMap::new()));

        let tx_clone = tx.clone();
        let users_clone = users.clone();

        tokio::spawn(async move {
            let mut rx = tx_clone.subscribe();
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        let sender = match &msg {
                            ServerMessage::NewMessage(chat_msg) => Some(chat_msg.sender.clone()),
                            ServerMessage::Joined { username } => Some(username.clone()),
                            ServerMessage::Left { username } => Some(username.clone()),
                            _ => None,
                        };

                        let sinks: Vec<(String, Arc<Mutex<WsSink>>)> = {
                            let users_guard = users_clone.lock().await;
                            users_guard
                                .iter()
                                .filter(|(username, _)| {
                                    sender.as_ref().map_or(true, |s| s != *username)
                                })
                                .map(|(username, sink)| (username.clone(), sink.clone()))
                                .collect()
                        };

                        for (username, sink) in sinks {
                            let json = match serde_json::to_string(&msg) {
                                Ok(json) => json,
                                Err(_) => continue,
                            };

                            let _ = sink.lock().await.send(WsMessage::Text(json)).await;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            users,
            tx,
            user_store,
        })
    }

    pub async fn handle_connection(
        users: SharedUsers,
        tx: broadcast::Sender<ServerMessage>,
        user_store: sled::Db,
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    ) {
        let (write_half, mut read_half) = ws_stream.split();
        let mut current_user: Option<String> = None;
        let write_arc = Arc::new(Mutex::new(write_half));

        while let Some(msg) = read_half.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Join { username }) => {
                        let key = format!("user:{}", username);
                        if user_store.contains_key(&key).unwrap_or(false) {
                            let error = ServerMessage::Error {
                                message: "Username already taken".to_string(),
                            };
                            let json = serde_json::to_string(&error).unwrap();
                            let _ = write_arc.lock().await.send(WsMessage::Text(json)).await;
                            break;
                        }

                        user_store.insert(key.clone(), b"1").unwrap();
                        user_store.flush().unwrap();

                        current_user = Some(username.clone());

                        {
                            let mut users_guard = users.lock().await;
                            users_guard.insert(username.clone(), write_arc.clone());
                        }

                        let welcome = ServerMessage::Welcome {
                            username: username.clone(),
                        };
                        let _ = tx.send(welcome);

                        let join_msg = ServerMessage::Joined {
                            username: username.clone(),
                        };
                        let _ = tx.send(join_msg);
                    }
                    Ok(ClientMessage::SendMessage { content }) => {
                        if let Some(ref sender) = current_user {
                            let msg = ChatMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                sender: sender.clone(),
                                content,
                                timestamp: SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u64,
                                delivered_to: vec![],
                            };

                            let server_msg = ServerMessage::NewMessage(msg);
                            let _ = tx.send(server_msg);
                        }
                    }
                    Ok(ClientMessage::AckReceived {
                        message_id,
                        intended_recipient,
                    }) => {
                        if let Some(ref username) = current_user {
                            let receipt = ServerMessage::DeliveryReceipt {
                                message_id,
                                received_by: username.clone(),
                            };

                            {
                                let users_guard = users.lock().await;
                                if let Some(recipient_sink) = users_guard.get(&intended_recipient) {
                                    let json = match serde_json::to_string(&receipt) {
                                        Ok(json) => json,
                                        Err(_) => return,
                                    };

                                    let _ = recipient_sink
                                        .lock()
                                        .await
                                        .send(WsMessage::Text(json))
                                        .await;
                                }
                            }
                        }
                    }
                    Ok(ClientMessage::Leave) => {
                        if let Some(ref username) = current_user {
                            let key = format!("user:{}", username);
                            user_store.remove(key).unwrap();
                            user_store.flush().unwrap();

                            {
                                let mut users_guard = users.lock().await;
                                users_guard.remove(username);
                            }

                            let leave_msg = ServerMessage::Left {
                                username: username.clone(),
                            };
                            let _ = tx.send(leave_msg);
                        }
                        break;
                    }
                    Err(_) => {}
                },
                Ok(WsMessage::Close(_)) | Err(_) => {
                    if let Some(ref username) = current_user {
                        let key = format!("user:{}", username);
                        user_store.remove(key).unwrap();
                        {
                            let mut users_guard = users.lock().await;
                            users_guard.remove(username);
                        }
                        let leave_msg = ServerMessage::Left {
                            username: username.clone(),
                        };
                        let _ = tx.send(leave_msg);
                    }
                    break;
                }
                _ => {}
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let server = Arc::new(Mutex::new(ChatServer::new().await?));
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let server_clone = server.clone();

                tokio::spawn(async move {
                    match accept_async(stream).await {
                        Ok(ws_stream) => {
                            let (users, tx, user_store) = {
                                let server = server_clone.lock().await;
                                (
                                    server.users.clone(),
                                    server.tx.clone(),
                                    server.user_store.clone(),
                                )
                            };

                            ChatServer::handle_connection(users, tx, user_store, ws_stream).await;
                        }
                        Err(_) => {}
                    }
                });
            }
            Err(_) => continue,
        }
    }
}
