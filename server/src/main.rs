use server_lib::room::Room;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let host = std::env::var("CHAT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("CHAT_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{host}:{port}");

    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {addr}: {e}"));

    info!("Chat server listening on {addr}");

    let room = Arc::new(Room::new());

    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept connection: {e}");
                continue;
            }
        };

        info!("New connection from {peer_addr}");
        let room = Arc::clone(&room);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, room).await {
                error!("Connection error from {peer_addr}: {e}");
            }
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    room: Arc<Room>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // First message must be a Join
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok(());
    }

    let username = match protocol::decode::<protocol::ClientMessage>(&line)? {
        protocol::ClientMessage::Join { username } => username,
        _ => {
            let err = protocol::encode(&protocol::ServerMessage::Error {
                message: "First message must be a Join".to_string(),
            })?;
            writer.write_all(err.as_bytes()).await?;
            return Ok(());
        }
    };

    let mut rx = match room.join(&username) {
        Ok(rx) => rx,
        Err(msg) => {
            let err = protocol::encode(&protocol::ServerMessage::Error { message: msg })?;
            writer.write_all(err.as_bytes()).await?;
            return Ok(());
        }
    };

    let welcome = protocol::encode(&protocol::ServerMessage::Welcome {
        message: format!("Welcome, {username}!"),
    })?;
    writer.write_all(welcome.as_bytes()).await?;

    info!("{username} joined the chat");

    let write_username = username.clone();
    let write_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let encoded = match protocol::encode(&msg) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if writer.write_all(encoded.as_bytes()).await.is_err() {
                break;
            }
        }
        write_username
    });

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await;

        match n {
            Ok(0) | Err(_) => {
                // Client disconnected
                break;
            }
            Ok(_) => {}
        }

        match protocol::decode::<protocol::ClientMessage>(&line) {
            Ok(protocol::ClientMessage::Send { content }) => {
                room.broadcast(&username, content);
            }
            Ok(protocol::ClientMessage::Leave) => {
                break;
            }
            Ok(protocol::ClientMessage::Join { .. }) => {
                // Ignore duplicate join
            }
            Err(e) => {
                error!("Failed to decode message from {username}: {e}");
            }
        }
    }

    room.leave(&username);
    info!("{username} left the chat");
    write_task.abort();

    Ok(())
}
