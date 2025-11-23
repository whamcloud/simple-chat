use clap::Parser;
use simple_chat::Message;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server host
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(long, env = "PORT", default_value = "8080")]
    port: String,

    /// Username
    #[arg(long, env = "USERNAME")]
    username: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.username.is_empty() {
        eprintln!("Username is required. Use --username or set USERNAME environment variable.");
        std::process::exit(1);
    }

    let addr = format!("{}:{}", args.host, args.port);
    println!("Connecting to {}...", addr);

    let stream = match TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            std::process::exit(1);
        }
    };

    let (reader, mut writer) = stream.into_split();

    let join_msg = Message::Join {
        username: args.username.clone(),
    };
    let join_json = serde_json::to_string(&join_msg)?;
    writer.write_all(join_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: Message = serde_json::from_str(line.trim())?;
    match response {
        Message::Error { message } => {
            eprintln!("Error: {}", message);
            std::process::exit(1);
        }
        Message::System { message } => {
            println!("{}", message);
        }
        _ => {}
    }

    let mut reader_clone = reader;
    tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match reader_clone.read_line(&mut line).await {
                Ok(0) => {
                    println!("\nServer disconnected");
                    std::process::exit(0);
                }
                Ok(_) => {
                    if let Ok(msg) = serde_json::from_str::<Message>(line.trim()) {
                        match msg {
                            Message::Chat { username, message } => {
                                println!("{}: {}", username, message);
                            }
                            Message::System { message } => {
                                println!("[System] {}", message);
                            }
                            _ => {}
                        }
                    }
                }
                Err(_) => {
                    println!("\nConnection error");
                    std::process::exit(0);
                }
            }
        }
    });

    let stdin = tokio::io::stdin();
    let mut stdin_reader = BufReader::new(stdin);
    let mut input = String::new();

    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        input.clear();

        match stdin_reader.read_line(&mut input).await {
            Ok(0) => break,
            Ok(_) => {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }

                if input == "leave" {
                    let leave_msg = Message::Leave {
                        username: args.username.clone(),
                    };
                    let leave_json = serde_json::to_string(&leave_msg)?;
                    let _ = writer.write_all(leave_json.as_bytes()).await;
                    let _ = writer.write_all(b"\n").await;
                    let _ = writer.flush().await;
                    println!("Leaving chat...");
                    break;
                } else if let Some(msg) = input.strip_prefix("send ") {
                    let chat_msg = Message::Chat {
                        username: args.username.clone(),
                        message: msg.to_string(),
                    };
                    let chat_json = serde_json::to_string(&chat_msg)?;
                    if writer.write_all(chat_json.as_bytes()).await.is_err() {
                        break;
                    }
                    if writer.write_all(b"\n").await.is_err() {
                        break;
                    }
                    if writer.flush().await.is_err() {
                        break;
                    }
                } else {
                    println!("Unknown command. Use 'send <MSG>' or 'leave'");
                }
            }
            Err(_) => break,
        }
    }

    Ok(())
}
