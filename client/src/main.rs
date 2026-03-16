use clap::Parser;
use protocol::{ClientMessage, ServerMessage};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Parser)]
#[command(name = "chat-client", about = "Simple async chat client")]
struct Args {
    /// Server host
    #[arg(long, env = "CHAT_HOST", default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(long, env = "CHAT_PORT", default_value = "8080")]
    port: u16,

    /// Username for the chat
    #[arg(long, env = "CHAT_USERNAME")]
    username: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port);

    let stream = TcpStream::connect(&addr).await?;
    println!("Connected to {addr}");

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send join message
    let join_msg = protocol::encode(&ClientMessage::Join {
        username: args.username.clone(),
    })?;
    writer.write_all(join_msg.as_bytes()).await?;

    // Read welcome/error response
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    match protocol::decode::<ServerMessage>(&line)? {
        ServerMessage::Welcome { message } => println!("{message}"),
        ServerMessage::Error { message } => {
            eprintln!("Error: {message}");
            return Ok(());
        }
        _ => {}
    }

    // Spawn task to read server messages
    let recv_task = tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("Server disconnected.");
                    break;
                }
                Ok(_) => {
                    if let Ok(msg) = protocol::decode::<ServerMessage>(&line) {
                        match msg {
                            ServerMessage::Chat { username, content } => {
                                println!("[{username}]: {content}");
                            }
                            ServerMessage::UserJoined { username } => {
                                println!("* {username} joined the chat");
                            }
                            ServerMessage::UserLeft { username } => {
                                println!("* {username} left the chat");
                            }
                            ServerMessage::Error { message } => {
                                eprintln!("Server error: {message}");
                            }
                            ServerMessage::Welcome { message } => {
                                println!("{message}");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Read error: {e}");
                    break;
                }
            }
        }
    });

    // Read user input from stdin
    let stdin = tokio::io::stdin();
    let mut stdin_reader = BufReader::new(stdin);
    let mut input = String::new();

    loop {
        input.clear();
        match stdin_reader.read_line(&mut input).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if trimmed == "leave" {
                    let leave_msg = protocol::encode(&ClientMessage::Leave)?;
                    writer.write_all(leave_msg.as_bytes()).await?;
                    println!("Disconnected from chat.");
                    break;
                } else if let Some(msg) = trimmed.strip_prefix("send ") {
                    if msg.is_empty() {
                        println!("Usage: send <message>");
                        continue;
                    }
                    let send_msg = protocol::encode(&ClientMessage::Send {
                        content: msg.to_string(),
                    })?;
                    writer.write_all(send_msg.as_bytes()).await?;
                } else {
                    println!("Unknown command. Available commands:");
                    println!("  send <MSG>  - Send a message to the chat");
                    println!("  leave       - Disconnect and exit");
                }
            }
            Err(e) => {
                eprintln!("Input error: {e}");
                break;
            }
        }
    }

    recv_task.abort();
    Ok(())
}
