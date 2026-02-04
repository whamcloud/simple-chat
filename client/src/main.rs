use anyhow::{Context, Result};
use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[command(name = "chat-client")]
#[command(about = "Simple chat client", long_about = None)]
struct Args {
    /// Server host
    #[arg(short = 'H', long, env = "CHAT_HOST", default_value = "127.0.0.1")]
    host: String,

    /// Server port
    #[arg(short, long, env = "CHAT_PORT", default_value = "8080")]
    port: u16,

    /// Username
    #[arg(short, long, env = "CHAT_USERNAME")]
    username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port);

    println!("Connecting to {}...", addr);
    let stream = TcpStream::connect(&addr)
        .await
        .context("Failed to connect to server")?;
    println!("Connected!");

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send JOIN command
    writer
        .write_all(format!("JOIN {}\n", args.username).as_bytes())
        .await?;

    // Read response
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    if line.trim() == "OK" {
        println!("Joined chat as '{}'", args.username);
    } else if line.starts_with("ERROR") {
        eprintln!("Failed to join: {}", line.trim());
        return Ok(());
    } else {
        eprintln!("Unexpected response: {}", line.trim());
        return Ok(());
    }

    // Spawn task to read from server
    let mut reader_clone = reader;
    tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match reader_clone.read_line(&mut line).await {
                Ok(0) => {
                    println!("\n[Server disconnected]");
                    std::process::exit(0);
                }
                Ok(_) => {
                    print!("\r{}", line);
                    print!(">> ");
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                }
                Err(_) => break,
            }
        }
    });

    // Read from stdin and send to server
    println!("\nCommands:");
    println!("  send <message>  - Send a message");
    println!("  leave           - Leave the chat\n");

    let stdin = tokio::io::stdin();
    let mut stdin_reader = BufReader::new(stdin);
    let mut input = String::new();

    loop {
        print!(">> ");
        use std::io::Write;
        std::io::stdout().flush()?;

        input.clear();
        let n = stdin_reader.read_line(&mut input).await?;
        if n == 0 {
            // EOF
            break;
        }

        let trimmed = input.trim();

        if trimmed == "leave" {
            writer.write_all(b"LEAVE\n").await?;
            println!("Goodbye!");
            break;
        } else if let Some(msg) = trimmed.strip_prefix("send ") {
            writer
                .write_all(format!("SEND {}\n", msg).as_bytes())
                .await?;
        } else if !trimmed.is_empty() {
            println!("Unknown command. Use 'send <message>' or 'leave'");
        }
    }

    Ok(())
}
