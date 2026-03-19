use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};


pub async fn handle_client(
    mut socket: TcpStream,
    tx: broadcast::Sender<String>,
    mut rx: broadcast::Receiver<String>,
) {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    // Read the first line as the client's user code
    let client_user = match reader.read_line(&mut line).await {
        Ok(n) if n > 0 => {
            let id = line.trim().to_string();
            println!("Client {} connected to the server room", id);
            line.clear();
            id
        }
        _ => return,
    };

    loop {
        tokio::select! {
            // Receive from client and broadcast
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => break,
                    Ok(_) => {
                        let broadcast_msg = format!("{} > {}", client_user, line);
                        if let Err(e) = tx.send(broadcast_msg) {
                            eprintln!("Broadcast error: {}", e);
                        }
                        line.clear();
                    }
                    Err(_) => break,
                }
            }
            // Receive from broadcast and send to client
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if let Err(e) = writer.write_all(msg.as_bytes()).await {
                            eprintln!("Write error: {}", e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(_) => break,
                }
            }
        }
    }

    println!("Client {} left server room", client_user);
}
