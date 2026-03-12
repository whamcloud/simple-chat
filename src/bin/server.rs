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

async fn handle_client(stream: TcpStream, _users: UserMap) -> anyhow::Result<()> {
    let (_tx, mut rx) = mpsc::unbounded_channel::<String>();
    let (reader, mut writer) = stream.into_split();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        println!("received: {}", line);
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