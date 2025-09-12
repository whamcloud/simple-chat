use std::env;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

use chrono::TimeZone;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, BufReader, stdin};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use url::Url;

mod shared_types;
use shared_types::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <host> <port> <username>", args[0]);
        return Ok(());
    }

    let host = &args[1];
    let port = &args[2];
    let username = &args[3];

    let url = Url::parse(&format!("ws://{}:{}/", host, port))?;
    let (ws_stream, _) = connect_async(url).await?;

    let (write_half, read_half) = ws_stream.split();
    let shared_write = Arc::new(Mutex::new(write_half));
    let shared_write_for_read_task = shared_write.clone();

    // Send join message
    let join_msg = ClientMessage::Join {
        username: username.to_string(),
    };
    let json = serde_json::to_string(&join_msg)?;
    {
        let mut write_guard = shared_write.lock().await;
        write_guard.send(WsMessage::Text(json)).await?;
    }

    // Spawn task to handle incoming messages
    let read_handle = tokio::spawn(async move {
        let mut read = read_half;

        while let Some(msg) = read.next().await {
            match msg {
                Ok(WsMessage::Text(text)) => {
                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(ServerMessage::Welcome { username: _ }) => {
                            println!("✅ Welcome to the chat!");
                            println!("➡️  Type 'send <message>' to chat or 'leave' to exit.");
                        }
                        Ok(ServerMessage::Joined { username }) => {
                            println!("📥 {} joined the chat", username);
                        }
                        Ok(ServerMessage::Left { username }) => {
                            println!("📤 {} left the chat", username);
                        }
                        Ok(ServerMessage::NewMessage(msg)) => {
                            let ts = chrono::Local
                                .timestamp_opt((msg.timestamp / 1000) as i64, 0)
                                .single()
                                .unwrap_or_else(|| chrono::Local::now())
                                .format("%H:%M:%S");
                            println!("[{}] [{}] {}", ts, msg.sender, msg.content);

                            // Send ACK
                            let ack = ClientMessage::AckReceived {
                                message_id: msg.id.clone(),
                                intended_recipient: msg.sender.clone(),
                            };
                            if let Ok(ack_json) = serde_json::to_string(&ack) {
                                let mut write_guard = shared_write_for_read_task.lock().await;
                                let _ = write_guard.send(WsMessage::Text(ack_json)).await;
                            }
                        }
                        Ok(ServerMessage::DeliveryReceipt {
                            message_id,
                            received_by,
                        }) => {
                            println!("✅ Message {} delivered to {}", message_id, received_by);
                        }
                        Ok(ServerMessage::Error { message }) => {
                            eprintln!("Error: {}", message);
                        }
                        Err(_) => {}
                    }
                }
                Ok(WsMessage::Close(_)) | Err(_) => {
                    println!("Disconnected from server.");
                    break;
                }
                _ => {}
            }
        }
    });

    // Handle user input
    let stdin = stdin();
    let mut lines = BufReader::new(stdin).lines();

    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();

        match lines.next_line().await {
            Ok(Some(line)) => {
                let line = line.trim();
                if line == "leave" {
                    let leave_msg = ClientMessage::Leave;
                    let json = serde_json::to_string(&leave_msg)?;
                    {
                        let mut write_guard = shared_write.lock().await;
                        let _ = write_guard.send(WsMessage::Text(json)).await;
                    }
                    println!("Goodbye!");
                    break;
                } else if line.starts_with("send ") {
                    let content = line[5..].to_string();
                    let msg = ClientMessage::SendMessage { content };
                    let json = serde_json::to_string(&msg)?;
                    {
                        let mut write_guard = shared_write.lock().await;
                        let _ = write_guard.send(WsMessage::Text(json)).await;
                    }
                } else {
                    println!("Unknown command. Use 'send <message>' or 'leave'");
                }
            }
            Err(_) => break,
            _ => break,
        }
    }

    let _ = read_handle.await;
    Ok(())
}
