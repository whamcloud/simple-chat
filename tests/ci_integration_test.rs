use simple_chat::{ChatServer, Message};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

async fn connect_and_join(
    addr: &str,
    username: &str,
) -> Result<
    (
        BufReader<tokio::net::tcp::OwnedReadHalf>,
        tokio::net::tcp::OwnedWriteHalf,
    ),
    Box<dyn std::error::Error>,
> {
    let stream = TcpStream::connect(addr).await?;
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let join_msg = Message::Join {
        username: username.to_string(),
    };
    let join_json = serde_json::to_string(&join_msg)?;
    let mut writer_clone = writer;
    writer_clone.write_all(join_json.as_bytes()).await?;
    writer_clone.write_all(b"\n").await?;
    writer_clone.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let response: Message = serde_json::from_str(line.trim())?;

    if let Message::Error { .. } = response {
        return Err("Failed to join".into());
    }

    Ok((reader, writer_clone))
}

#[tokio::test]
async fn test_server_and_client_integration() {
    let server = ChatServer::new();
    let addr = "127.0.0.1:0";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    tokio::spawn(async move {
        server.run(&local_addr.to_string()).await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;
    let (_reader, mut writer) = match timeout(
        Duration::from_secs(5),
        connect_and_join(&local_addr.to_string(), "testuser"),
    )
    .await
    {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => panic!("Failed to connect: {}", e),
        Err(_) => panic!("Timeout connecting to server"),
    };

    let chat_msg = Message::Chat {
        username: "testuser".to_string(),
        message: "Hello from test".to_string(),
    };
    let chat_json = serde_json::to_string(&chat_msg).unwrap();
    writer.write_all(chat_json.as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    writer.flush().await.unwrap();
    let leave_msg = Message::Leave {
        username: "testuser".to_string(),
    };
    let leave_json = serde_json::to_string(&leave_msg).unwrap();
    writer.write_all(leave_json.as_bytes()).await.unwrap();
    writer.write_all(b"\n").await.unwrap();
    writer.flush().await.unwrap();
}
