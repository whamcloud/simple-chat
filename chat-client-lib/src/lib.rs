use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LinesCodec};

pub async fn run_client(addr: String, username: String) -> Result<()> {
    println!("Connecting to {}...", addr);
    let stream = TcpStream::connect(&addr)
        .await
        .context("Failed to connect to chat server")?;

    let mut lines = Framed::new(stream, LinesCodec::new());

    if let Some(Ok(msg)) = lines.next().await {
        println!("{}", msg);
    }

    lines.send(&username).await?;

    match lines.next().await {
        Some(Ok(msg)) => {
            println!("{}", msg);
            if msg.starts_with("Invalid") || msg.starts_with("Username") {
                return Ok(());
            }
        }
        _ => return Ok(()),
    }

    println!("Session started. Type 'send <MSG>' to chat, or 'leave' to quit.");

    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut line = String::new();
        loop {
            line.clear();
            if stdin.read_line(&mut line).is_ok() {
                if input_tx.send(line.trim().to_string()).is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    });

    loop {
        tokio::select! {
            msg = lines.next() => {
                match msg {
                    Some(Ok(text)) => println!("{}", text),
                    Some(Err(e)) => {
                        eprintln!("Network error: {}", e);
                        break;
                    }
                    None => {
                        println!("Server disconnected.");
                        break;
                    }
                }
            }

            input = input_rx.recv() => {
                match input {
                    Some(cmd) => {
                        if cmd == "leave" {
                            lines.send("leave").await?;
                            break;
                        } else if cmd.starts_with("send ") {
                             lines.send(&cmd).await?;
                        } else {
                             println!("Unknown command. Use 'send <MSG>' or 'leave'.");
                        }
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}
