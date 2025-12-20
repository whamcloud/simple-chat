use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

type InMemoryUserDb = Arc<Mutex<HashSet<String>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server running on 127.0.0.1:8080");

    let (tx, _rx) = broadcast::channel::<(String, String)>(100);
    let state: InMemoryUserDb = InMemoryUserDb::new(Mutex::new(HashSet::new()));

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let state = state.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            let username = match reader.read_line(&mut line).await {
                Ok(n) if n > 0 => {
                    let name = line.trim().to_string();
                    line.clear();
                    name
                }
                _ => return,
            };

            let is_taken = {
                let mut users = state.lock().unwrap();
                if users.contains(&username) {
                    true
                } else {
                    users.insert(username.clone());
                    false
                }
            };

            if is_taken {
                let _ = writer.write_all(b"Error: Username taken\n").await;
                return;
            }

            println!("User '{}' joined from {:?}", username, addr);
            let _ = writer.write_all(b"Welcome to the chat!\n").await;

            let _ = tx.send((
                username.clone(),
                format!("{} has joined the chat", username),
            ));

            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        match result {
                            Ok(0) => break,
                            Ok(_) => {
                                let msg = line.trim();
                                if !msg.is_empty() {
                                    let _ = tx.send((username.clone(), format!("{}: {}", username, msg)));
                                }
                                line.clear();
                            }
                            Err(_) => break,
                        }
                    }
                    result = rx.recv() => {
                        match result {
                            Ok((sender_name, msg)) => {
                                if sender_name != username {
                                    if writer.write_all(msg.as_bytes()).await.is_err() { break; }
                                    let _ = writer.write_all(b"\n").await;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            }

            {
                let mut users = state.lock().unwrap();
                users.remove(&username);
            }
            let _ = tx.send((username.clone(), format!("{} has left the chat", username)));
            println!("User '{}' disconnected", username);
        });
    }
}
