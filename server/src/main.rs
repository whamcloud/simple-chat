use anyhow::Result;
use server::{ClientMessage, Room};
use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("CHAT_HOST").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let addr: SocketAddr = addr.parse()?;

    // Create socket with SO_REUSEADDR to allow immediate reuse of the address
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    socket.listen(128)?;

    // Convert to tokio TcpListener
    let listener = TcpListener::from_std(socket.into())?;
    println!("Chat server listening on {}", addr);

    let room = Room::new();

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        println!("New connection from {}", peer_addr);
        let room_handle = room.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, room_handle).await {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(stream: TcpStream, room: Room) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Read JOIN command
    line.clear();
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok(()); // Connection closed
    }

    let join_msg = ClientMessage::parse(&line).map_err(|e| anyhow::anyhow!(e))?;
    let username = match join_msg {
        ClientMessage::Join(username) => username,
        _ => {
            writer.write_all(b"ERROR Expected JOIN command\n").await?;
            return Ok(());
        }
    };

    // Create channel for this user
    let (tx, mut rx) = mpsc::channel(100);

    // Try to join room
    match room.join(username.clone(), tx).await {
        Ok(()) => {
            writer.write_all(b"OK\n").await?;
            println!("User '{}' joined", username);
        }
        Err(e) => {
            writer
                .write_all(format!("ERROR {}\n", e).as_bytes())
                .await?;
            return Ok(());
        }
    }

    // Spawn task to write messages to client
    let mut writer_clone = writer;
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let wire = msg.to_wire();
            if writer_clone.write_all(wire.as_bytes()).await.is_err() {
                break;
            }
        }
    });

    // Read messages from client
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            // Connection closed
            break;
        }

        match ClientMessage::parse(&line) {
            Ok(ClientMessage::Send(text)) => {
                room.broadcast(username.clone(), text)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
            }
            Ok(ClientMessage::Leave) => {
                break;
            }
            Ok(ClientMessage::Join(_)) => {
                // Ignore additional JOIN commands
            }
            Err(e) => {
                eprintln!("Parse error from {}: {}", username, e);
            }
        }
    }

    // Clean up
    room.leave(username.clone())
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    println!("User '{}' left", username);

    Ok(())
}
