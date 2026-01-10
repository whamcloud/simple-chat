use chatserver::{ClientMessage, ServerMessage};
use clap::Parser;
use std::io::{self, BufRead, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// A simple async chat client
#[derive(Parser, Debug)]
#[command(name = "Chat Client")]
#[command(about = "Connect to a chat server and send/receive messages", long_about = None)]
struct Args {
    /// Username for the chat
    #[arg(short, long)]
    username: Option<String>,

    /// Server host address
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(short, long, default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Get username from environment variable or command line
    let username = match args.username {
        Some(u) => u,
        None => std::env::var("CHAT_USERNAME").unwrap_or_else(|_| {
            print!("Enter your username: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            input.trim().to_string()
        }),
    };

    if username.is_empty() {
        eprintln!("Username cannot be empty");
        return Err("Empty username".into());
    }

    let addr = format!("{}:{}", args.host, args.port);
    println!("Connecting to {}...", addr);

    let socket = TcpStream::connect(&addr).await?;
    println!("Connected to server!");

    let (reader, mut writer) = socket.into_split();
    let reader = BufReader::new(reader);

    // Send join message
    let join_msg = ClientMessage::Join {
        username: username.clone(),
    };
    let join_bytes = join_msg.to_bytes()?;
    writer.write_all(&join_bytes).await?;

    // Spawn task to receive messages
    let receive_task = tokio::spawn(async move {
        let mut reader = reader;
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("\nDisconnected from server");
                    break;
                }
                Ok(_) => match serde_json::from_str::<ServerMessage>(&line) {
                    Ok(msg) => {
                        println!("{}", msg);
                    }
                    Err(e) => {
                        eprintln!("Failed to parse message: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Error reading from server: {}", e);
                    break;
                }
            }
        }
    });

    // Command prompt for user input
    println!("\nCommands:");
    println!("  send <MSG>  - Send a message");
    println!("  leave       - Disconnect and exit\n");

    let stdin = io::stdin();
    let reader = stdin.lock();
    for line in reader.lines() {
        if let Ok(input) = line {
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            if input == "leave" {
                let leave_msg = ClientMessage::Leave;
                let leave_bytes = leave_msg.to_bytes()?;
                writer.write_all(&leave_bytes).await?;
                println!("Goodbye!");
                break;
            } else if input.starts_with("send ") {
                let content = input[5..].to_string();
                let send_msg = ClientMessage::Send { content };
                let send_bytes = send_msg.to_bytes()?;
                if writer.write_all(&send_bytes).await.is_err() {
                    eprintln!("Failed to send message - connection lost");
                    break;
                }
            } else {
                eprintln!("Unknown command: '{}'. Use 'send <MSG>' or 'leave'", input);
            }
        }
    }

    receive_task.abort();
    Ok(())
}
