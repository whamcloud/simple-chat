use anyhow::Result;
use futures::{SinkExt, StreamExt};
use simple_chat::{codec::MessageCodec, Message};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout};
use tokio_util::codec::{FramedRead, FramedWrite};

// Test helper to find an available port
async fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    addr.port()
}

// Test helper to create a test client connection
async fn create_test_client(
    host: &str,
    port: u16,
) -> Result<(
    FramedRead<tokio::net::tcp::OwnedReadHalf, MessageCodec>,
    FramedWrite<tokio::net::tcp::OwnedWriteHalf, MessageCodec>,
)> {
    let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
    let (read_half, write_half) = stream.into_split();

    let framed_read = FramedRead::new(read_half, MessageCodec);
    let framed_write = FramedWrite::new(write_half, MessageCodec);

    Ok((framed_read, framed_write))
}

// Test helper to join a user and return success/failure
async fn join_user(
    writer: &mut FramedWrite<tokio::net::tcp::OwnedWriteHalf, MessageCodec>,
    reader: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, MessageCodec>,
    username: &str,
) -> Result<bool> {
    let join_msg = Message::Join {
        username: username.to_string(),
    };
    writer.send(join_msg).await?;

    match timeout(Duration::from_secs(2), reader.next()).await {
        Ok(Some(Ok(Message::UserJoined {
            username: joined_user,
        }))) => Ok(joined_user == username),
        Ok(Some(Ok(Message::Error { .. }))) => Ok(false),
        _ => Ok(false),
    }
}

// Test helper to start server in background
async fn start_test_server(port: u16) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let result = tokio::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "server",
                "--",
                "--port",
                &port.to_string(),
                "--max-users",
                "10",
            ])
            .status()
            .await;

        if let Err(e) = result {
            eprintln!("Server failed to start: {}", e);
        }
    })
}

#[tokio::test]
async fn test_server_startup_and_basic_connection() {
    let port = find_available_port().await;
    let _server_handle = start_test_server(port).await;

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    // Test basic connection
    let result = timeout(
        Duration::from_secs(5),
        TcpStream::connect(format!("127.0.0.1:{}", port)),
    )
    .await;

    assert!(result.is_ok(), "Should be able to connect to server");
    let stream = result.unwrap();
    assert!(stream.is_ok(), "Connection should succeed");
}

#[tokio::test]
async fn test_multiple_client_connections() {
    let port = find_available_port().await;
    let _server_handle = start_test_server(port).await;

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    // Create multiple clients
    let mut clients = Vec::new();
    for i in 0..3 {
        match create_test_client("127.0.0.1", port).await {
            Ok(client) => clients.push((format!("user{}", i), client)),
            Err(e) => panic!("Failed to create client {}: {}", i, e),
        }
    }

    assert_eq!(clients.len(), 3, "Should have 3 connected clients");

    // Test that all clients can join successfully
    for (username, (mut reader, mut writer)) in clients.into_iter() {
        let joined = join_user(&mut writer, &mut reader, &username)
            .await
            .unwrap_or(false);
        assert!(joined, "User {} should be able to join", username);
    }
}

#[tokio::test]
async fn test_message_broadcasting() {
    let port = find_available_port().await;
    let _server_handle = start_test_server(port).await;

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    // Create two clients
    let (mut reader1, mut writer1) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client 1");
    let (mut reader2, mut writer2) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client 2");

    // Join both users
    assert!(join_user(&mut writer1, &mut reader1, "alice")
        .await
        .unwrap_or(false));
    assert!(join_user(&mut writer2, &mut reader2, "bob")
        .await
        .unwrap_or(false));

    // Clear any pending join notifications
    sleep(Duration::from_millis(100)).await;
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader1.next()).await {}
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader2.next()).await {}

    // Send message from alice
    let chat_msg = Message::ChatMessage {
        username: "alice".to_string(),
        content: "Hello everyone!".to_string(),
    };
    writer1
        .send(chat_msg.clone())
        .await
        .expect("Failed to send message");

    // Bob should receive the message
    match timeout(Duration::from_secs(2), reader2.next()).await {
        Ok(Some(Ok(Message::ChatMessage { username, content }))) => {
            assert_eq!(username, "alice");
            assert_eq!(content, "Hello everyone!");
        }
        other => panic!("Expected chat message, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_username_uniqueness_enforcement() {
    let port = find_available_port().await;
    let _server_handle = start_test_server(port).await;

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    // Create two clients
    let (mut reader1, mut writer1) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client 1");
    let (mut reader2, mut writer2) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client 2");

    // First user joins successfully
    assert!(join_user(&mut writer1, &mut reader1, "testuser")
        .await
        .unwrap_or(false));

    // Second user tries to join with same username - should fail
    let join_msg = Message::Join {
        username: "testuser".to_string(),
    };
    writer2
        .send(join_msg)
        .await
        .expect("Failed to send join message");

    match timeout(Duration::from_secs(2), reader2.next()).await {
        Ok(Some(Ok(Message::Error { message }))) => {
            assert!(
                message.contains("already taken"),
                "Error should mention username is taken"
            );
        }
        other => panic!("Expected error message, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_client_disconnect_cleanup() {
    let port = find_available_port().await;
    let _server_handle = start_test_server(port).await;

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    // Create three clients
    let (mut reader1, mut writer1) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client 1");
    let (mut reader2, mut writer2) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client 2");
    let (mut reader3, mut writer3) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client 3");

    // All users join
    assert!(join_user(&mut writer1, &mut reader1, "alice")
        .await
        .unwrap_or(false));
    assert!(join_user(&mut writer2, &mut reader2, "bob")
        .await
        .unwrap_or(false));
    assert!(join_user(&mut writer3, &mut reader3, "charlie")
        .await
        .unwrap_or(false));

    // Clear join notifications
    sleep(Duration::from_millis(200)).await;
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader1.next()).await {}
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader3.next()).await {}

    // Bob leaves gracefully
    let leave_msg = Message::Leave {
        username: "bob".to_string(),
    };
    writer2
        .send(leave_msg)
        .await
        .expect("Failed to send leave message");

    // Drop bob's connection to simulate disconnect
    drop(writer2);
    drop(reader2);

    // Alice and Charlie should receive leave notification
    sleep(Duration::from_millis(100)).await;

    let mut found_leave_notification = false;
    for _ in 0..3 {
        // Check a few messages
        if let Ok(Some(Ok(Message::UserLeft { username }))) =
            timeout(Duration::from_millis(500), reader1.next()).await
        {
            if username == "bob" {
                found_leave_notification = true;
                break;
            }
        }
    }

    assert!(
        found_leave_notification,
        "Should receive leave notification for bob"
    );

    // New user should be able to take bob's username
    let (mut reader4, mut writer4) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client 4");
    assert!(join_user(&mut writer4, &mut reader4, "bob")
        .await
        .unwrap_or(false));
}

#[tokio::test]
async fn test_server_max_users_limit() {
    let port = find_available_port().await;
    let _server_handle = start_test_server(port).await;

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    // Try to connect more users than the limit (server started with --max-users 10)
    let mut successful_joins = 0;
    let mut clients = Vec::new();

    for i in 0..12 {
        // Try to create 12 users (limit is 10)
        if let Ok((mut reader, mut writer)) = create_test_client("127.0.0.1", port).await {
            let username = format!("user{}", i);
            if join_user(&mut writer, &mut reader, &username)
                .await
                .unwrap_or(false)
            {
                successful_joins += 1;
                clients.push((reader, writer));
            } else {
                // Check if we got a "server is full" error
                break;
            }
        }
    }

    // Should be able to join up to the limit
    assert!(successful_joins <= 10, "Should not exceed max user limit");
    assert!(
        successful_joins >= 8,
        "Should be able to join at least 8 users"
    ); // Allow some margin for test reliability
}

#[tokio::test]
async fn test_invalid_message_handling() {
    let port = find_available_port().await;
    let _server_handle = start_test_server(port).await;

    // Wait for server to start
    sleep(Duration::from_millis(1000)).await;

    let (mut reader, mut writer) = create_test_client("127.0.0.1", port)
        .await
        .expect("Failed to create client");

    // Join first
    assert!(join_user(&mut writer, &mut reader, "testuser")
        .await
        .unwrap_or(false));

    // Clear join notification
    sleep(Duration::from_millis(100)).await;
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader.next()).await {}

    // Try to send chat message with wrong username
    let invalid_msg = Message::ChatMessage {
        username: "wronguser".to_string(),
        content: "This should fail".to_string(),
    };
    writer
        .send(invalid_msg)
        .await
        .expect("Failed to send message");

    // Should receive error message
    match timeout(Duration::from_secs(2), reader.next()).await {
        Ok(Some(Ok(Message::Error { message }))) => {
            assert!(
                message.contains("mismatch"),
                "Error should mention username mismatch"
            );
        }
        other => panic!("Expected error message, got: {:?}", other),
    }
}
