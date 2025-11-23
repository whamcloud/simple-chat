use simple_chat::Message;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

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
async fn test_multiple_users_can_join_and_chat() {
    let server = simple_chat::ChatServer::new();
    let addr = "127.0.0.1:0";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    tokio::spawn(async move {
        server.run(&local_addr.to_string()).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (mut reader1, mut writer1) = connect_and_join(&local_addr.to_string(), "alice")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (mut reader2, _writer2) = connect_and_join(&local_addr.to_string(), "bob")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // send message
    let chat_msg = Message::Chat {
        username: "alice".to_string(),
        message: "Hello Bob!".to_string(),
    };
    let chat_json = serde_json::to_string(&chat_msg).unwrap();
    writer1.write_all(chat_json.as_bytes()).await.unwrap();
    writer1.write_all(b"\n").await.unwrap();
    writer1.flush().await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut line = String::new();
    let mut found_chat = false;
    for _ in 0..5 {
        let result = timeout(Duration::from_secs(1), reader2.read_line(&mut line)).await;
        if result.is_err() {
            break;
        }
        let received: Message = serde_json::from_str(line.trim()).unwrap();
        match received {
            Message::Chat { username, message } => {
                assert_eq!(username, "alice");
                assert_eq!(message, "Hello Bob!");
                found_chat = true;
                break;
            }
            Message::System { .. } => {
                // Skip system messages
                line.clear();
                continue;
            }
            _ => panic!("Unexpected message type"),
        }
    }
    assert!(found_chat, "Bob should receive chat message from Alice");

    // alice shouldn't get her own message
    let mut line2 = String::new();
    let result2 = timeout(Duration::from_millis(200), reader1.read_line(&mut line2)).await;
    if result2.is_ok() {
        // If she received something, verify it's not her own chat message
        let received: Message = serde_json::from_str(line2.trim()).unwrap();
        match received {
            Message::Chat { username, .. } => {
                assert_ne!(
                    username, "alice",
                    "Alice should not receive her own chat message"
                );
            }
            Message::System { .. } => {
                // System messages are fine (like "bob joined")
            }
            _ => {}
        }
    }
    // If timeout occurred, that's also fine - means no message received
}

#[tokio::test]
async fn test_user_leaves_chat() {
    let server = simple_chat::ChatServer::new();
    let addr = "127.0.0.1:0";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    tokio::spawn(async move {
        server.run(&local_addr.to_string()).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let (_reader1, mut writer1) = connect_and_join(&local_addr.to_string(), "alice")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (mut reader2, _writer2) = connect_and_join(&local_addr.to_string(), "bob")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Alice leaves
    let leave_msg = Message::Leave {
        username: "alice".to_string(),
    };
    let leave_json = serde_json::to_string(&leave_msg).unwrap();
    writer1.write_all(leave_json.as_bytes()).await.unwrap();
    writer1.write_all(b"\n").await.unwrap();
    writer1.flush().await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut line = String::new();
    let mut found_leave = false;
    for _ in 0..5 {
        let result = timeout(Duration::from_secs(1), reader2.read_line(&mut line)).await;
        if result.is_err() {
            break;
        }
        let received: Message = serde_json::from_str(line.trim()).unwrap();
        match received {
            Message::System { message } => {
                if message.contains("left") && message.contains("alice") {
                    found_leave = true;
                    break;
                }
                // Skip other system messages (like join notifications)
            }
            _ => {
                // Skip non-system messages
            }
        }
        line.clear();
    }
    assert!(
        found_leave,
        "Bob should receive leave notification about Alice"
    );
}
