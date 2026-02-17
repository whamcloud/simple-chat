// Chat client binary

use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

#[derive(Parser, Debug)]
#[command(name = "simple-chat-client")]
#[command(about = "Simple chat client", long_about = None)]
struct Args {
    /// Server host
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Username
    #[arg(short, long)]
    username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum Message {
    Join { username: String },
    Leave,
    Send { content: String },
    Broadcast { username: String, content: String },
    Error { message: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let url = format!("ws://{}:{}", args.host, args.port);

    let (ws_stream, _) = connect_async(&url).await?;
    println!("Connected to server at {}", url);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Send join message
    let join_msg = Message::Join {
        username: args.username.clone(),
    };
    ws_sender
        .send(WsMessage::Text(serde_json::to_string(&join_msg)?))
        .await?;

    // Spawn task to receive messages from server
    let recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(WsMessage::Text(text)) => {
                    if let Ok(msg) = serde_json::from_str::<Message>(&text) {
                        match msg {
                            Message::Broadcast { username, content } => {
                                println!("[{}]: {}", username, content);
                            }
                            Message::Error { message } => {
                                eprintln!("Error: {}", message);
                            }
                            _ => {}
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    println!("Connection closed by server");
                    break;
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Read commands from stdin
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    println!("Commands: send <message> | leave");
    loop {
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush()?;

        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "leave" {
            let leave_msg = Message::Leave;
            ws_sender
                .send(WsMessage::Text(serde_json::to_string(&leave_msg)?))
                .await?;
            break;
        } else if let Some(content) = trimmed.strip_prefix("send ") {
            let send_msg = Message::Send {
                content: content.to_string(),
            };
            ws_sender
                .send(WsMessage::Text(serde_json::to_string(&send_msg)?))
                .await?;
        } else {
            println!("Unknown command. Use 'send <message>' or 'leave'");
        }
    }

    recv_task.abort();
    println!("Disconnected");
    Ok(())
}
