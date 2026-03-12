use simple_chat::protocol::{ClientEvent, ServerEvent};
use std::{collections::HashMap, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{RwLock, mpsc},
};

type ClientSender = mpsc::UnboundedSender<String>;
type UserMap = Arc<RwLock<HashMap<String, ClientSender>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    println!("chat server listening on 127.0.0.1:8080");

    let users: UserMap = Arc::new(RwLock::new(HashMap::new()));

    loop {
        let (socket, addr) = listener.accept().await?;

        println!("client connected: {}", addr);

        let users = users.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, users).await {
                println!("connection error: {:?}", e);
            }
        });
    }
}

async fn handle_client(stream: TcpStream, users: UserMap) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let (reader, mut writer) = stream.into_split();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    let mut username: Option<String> = None;

    while let Some(line) = lines.next_line().await? {
        let command: ClientEvent = match serde_json::from_str(&line) {
            Ok(cmd) => cmd,
            Err(_) => continue,
        };

        match command {
            ClientEvent::Join { username: name } => {
                let mut map = users.write().await;

                if map.contains_key(&name) {
                    let error = ServerEvent::Error {
                        reason: "username already taken".into(),
                    };

                    let _ = tx.send(serde_json::to_string(&error)?);
                } else {
                    map.insert(name.clone(), tx.clone());
                    username = Some(name);

                    println!("user joined");
                }
            }

            _ => {}
        }
    }

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            use tokio::io::AsyncWriteExt;

            if writer.write_all(msg.as_bytes()).await.is_err() {
                break;
            }

            let _ = writer.write_all(b"\n").await;
        }
    });
    Ok(())
}
