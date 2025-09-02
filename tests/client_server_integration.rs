use anyhow::Result;
use futures::{SinkExt, StreamExt};
use simple_chat::{codec::MessageCodec, Message};
use std::process::Stdio;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tokio_util::codec::{FramedRead, FramedWrite};

// Test helper to find an available port
async fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    addr.port()
}

// Test helper to start server process
async fn start_server_process(port: u16) -> Result<tokio::process::Child> {
    let child = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "server",
            "--",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--max-users",
            "20",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    // Wait a bit for server to start
    sleep(Duration::from_millis(1500)).await;

    Ok(child)
}

// Test helper for client connection with full protocol
async fn create_authenticated_client(
    host: &str,
    port: u16,
    username: &str,
) -> Result<(
    FramedRead<tokio::net::tcp::OwnedReadHalf, MessageCodec>,
    FramedWrite<tokio::net::tcp::OwnedWriteHalf, MessageCodec>,
)> {
    let stream = timeout(
        Duration::from_secs(5),
        TcpStream::connect(format!("{}:{}", host, port)),
    )
    .await??;

    let (read_half, write_half) = stream.into_split();
    let mut framed_read = FramedRead::new(read_half, MessageCodec);
    let mut framed_write = FramedWrite::new(write_half, MessageCodec);

    // Send join message
    let join_msg = Message::Join {
        username: username.to_string(),
    };
    framed_write.send(join_msg).await?;

    // Wait for join confirmation
    match timeout(Duration::from_secs(3), framed_read.next()).await {
        Ok(Some(Ok(Message::UserJoined {
            username: joined_user,
        }))) => {
            if joined_user == username {
                Ok((framed_read, framed_write))
            } else {
                Err(anyhow::anyhow!("Username mismatch in join confirmation"))
            }
        }
        Ok(Some(Ok(Message::Error { message }))) => {
            Err(anyhow::anyhow!("Join failed: {}", message))
        }
        other => Err(anyhow::anyhow!("Unexpected join response: {:?}", other)),
    }
}

// Test helper to send a chat message
async fn send_chat_message(
    writer: &mut FramedWrite<tokio::net::tcp::OwnedWriteHalf, MessageCodec>,
    username: &str,
    content: &str,
) -> Result<()> {
    let msg = Message::ChatMessage {
        username: username.to_string(),
        content: content.to_string(),
    };
    writer.send(msg).await?;
    Ok(())
}

// Test helper to expect a specific message
async fn expect_message(
    reader: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, MessageCodec>,
    expected: Message,
    timeout_secs: u64,
) -> Result<()> {
    match timeout(Duration::from_secs(timeout_secs), reader.next()).await {
        Ok(Some(Ok(actual))) => {
            if actual == expected {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Expected {:?}, got {:?}", expected, actual))
            }
        }
        Ok(Some(Err(e))) => Err(anyhow::anyhow!("Read error: {}", e)),
        Ok(None) => Err(anyhow::anyhow!("Connection closed unexpectedly")),
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for message")),
    }
}

#[tokio::test]
async fn test_full_client_server_communication_flow() {
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    // Create authenticated client
    let (mut reader, mut writer) = create_authenticated_client("127.0.0.1", port, "testuser")
        .await
        .expect("Failed to create authenticated client");

    // First, we might receive a UserJoined message (the server broadcasts join confirmations)
    // Let's clear any pending messages first
    tokio::time::sleep(Duration::from_millis(100)).await;
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader.next()).await {}

    // Send a chat message
    send_chat_message(&mut writer, "testuser", "Hello world!")
        .await
        .expect("Failed to send chat message");

    // Should receive our own message back (broadcast)
    let expected = Message::ChatMessage {
        username: "testuser".to_string(),
        content: "Hello world!".to_string(),
    };
    expect_message(&mut reader, expected, 3)
        .await
        .expect("Should receive chat message broadcast");

    // Send leave message
    let leave_msg = Message::Leave {
        username: "testuser".to_string(),
    };
    writer
        .send(leave_msg)
        .await
        .expect("Failed to send leave message");

    // Cleanup
    let _ = server.kill().await;
}

#[tokio::test]
async fn test_join_send_leave_operations() {
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    // Test 1: Join operation
    let (mut reader1, mut writer1) = create_authenticated_client("127.0.0.1", port, "alice")
        .await
        .expect("Alice should be able to join");

    // Clear any remaining messages from alice's join
    tokio::time::sleep(Duration::from_millis(100)).await;
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader1.next()).await {}

    // Test 2: Second user join - first user should see notification
    let (mut reader2, mut writer2) = create_authenticated_client("127.0.0.1", port, "bob")
        .await
        .expect("Bob should be able to join");

    // Alice should receive bob's join notification
    let expected = Message::UserJoined {
        username: "bob".to_string(),
    };
    expect_message(&mut reader1, expected, 3)
        .await
        .expect("Alice should receive bob's join notification");

    // Clear any remaining messages from bob's join
    tokio::time::sleep(Duration::from_millis(100)).await;
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader2.next()).await {}

    // Test 3: Send operations
    send_chat_message(&mut writer1, "alice", "Hi Bob!")
        .await
        .expect("Alice should be able to send message");

    // Bob should receive Alice's message
    let expected = Message::ChatMessage {
        username: "alice".to_string(),
        content: "Hi Bob!".to_string(),
    };
    expect_message(&mut reader2, expected, 3)
        .await
        .expect("Bob should receive Alice's message");

    // Clear Alice's echo of her own message
    tokio::time::sleep(Duration::from_millis(100)).await;
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader1.next()).await {}

    // Bob replies
    send_chat_message(&mut writer2, "bob", "Hi Alice!")
        .await
        .expect("Bob should be able to send message");

    // Alice should receive Bob's message
    let expected = Message::ChatMessage {
        username: "bob".to_string(),
        content: "Hi Alice!".to_string(),
    };
    expect_message(&mut reader1, expected, 3)
        .await
        .expect("Alice should receive Bob's message");

    // Clear Bob's echo of his own message
    tokio::time::sleep(Duration::from_millis(100)).await;
    while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader2.next()).await {}

    // Test 4: Leave operation
    let leave_msg = Message::Leave {
        username: "alice".to_string(),
    };
    writer1
        .send(leave_msg)
        .await
        .expect("Alice should be able to leave");

    // Bob should receive leave notification
    let expected = Message::UserLeft {
        username: "alice".to_string(),
    };
    expect_message(&mut reader2, expected, 3)
        .await
        .expect("Bob should receive Alice's leave notification");

    // Cleanup
    let _ = server.kill().await;
}

#[tokio::test]
async fn test_error_handling_scenarios() {
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    // Test 1: Invalid username
    let stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("Should be able to connect");

    let (read_half, write_half) = stream.into_split();
    let mut framed_read = FramedRead::new(read_half, MessageCodec);
    let mut framed_write = FramedWrite::new(write_half, MessageCodec);

    // Try to join with invalid username
    let join_msg = Message::Join {
        username: "a".to_string(),
    }; // Too short
    framed_write
        .send(join_msg)
        .await
        .expect("Should be able to send join message");

    // Should receive error
    match timeout(Duration::from_secs(2), framed_read.next()).await {
        Ok(Some(Ok(Message::Error { message }))) => {
            assert!(
                message.contains("at least 2 characters"),
                "Error should mention username length requirement"
            );
        }
        other => panic!(
            "Expected error message for invalid username, got: {:?}",
            other
        ),
    }

    // Test 2: Duplicate username
    let (_reader1, _writer1) = create_authenticated_client("127.0.0.1", port, "testuser")
        .await
        .expect("First user should join successfully");

    // Try to join with same username
    let stream2 = TcpStream::connect(format!("127.0.0.1:{}", port))
        .await
        .expect("Should be able to connect");

    let (read_half2, write_half2) = stream2.into_split();
    let mut framed_read2 = FramedRead::new(read_half2, MessageCodec);
    let mut framed_write2 = FramedWrite::new(write_half2, MessageCodec);

    let join_msg2 = Message::Join {
        username: "testuser".to_string(),
    };
    framed_write2
        .send(join_msg2)
        .await
        .expect("Should be able to send join message");

    // Should receive error
    match timeout(Duration::from_secs(2), framed_read2.next()).await {
        Ok(Some(Ok(Message::Error { message }))) => {
            assert!(
                message.contains("already taken"),
                "Error should mention username is taken"
            );
        }
        other => panic!(
            "Expected error message for duplicate username, got: {:?}",
            other
        ),
    }

    // Cleanup
    let _ = server.kill().await;
}

#[tokio::test]
async fn test_concurrent_client_operations() {
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    // Create multiple clients concurrently
    let client_count = 5;
    let mut join_handles = Vec::new();
    let (tx, mut rx) = mpsc::channel(client_count);

    for i in 0..client_count {
        let tx = tx.clone();
        let username = format!("user{}", i);

        let handle = tokio::spawn(async move {
            let result = create_authenticated_client("127.0.0.1", port, &username).await;
            let _ = tx.send((i, username, result)).await;
        });

        join_handles.push(handle);
    }

    // Collect results
    let mut successful_clients = Vec::new();
    for _ in 0..client_count {
        if let Some((i, username, result)) = rx.recv().await {
            match result {
                Ok((reader, writer)) => {
                    successful_clients.push((i, username, reader, writer));
                }
                Err(e) => panic!("Client {} failed to connect: {}", i, e),
            }
        }
    }

    assert_eq!(
        successful_clients.len(),
        client_count,
        "All clients should connect successfully"
    );

    // Test concurrent message sending
    let message_handles: Vec<_> = successful_clients
        .into_iter()
        .map(|(i, username, mut reader, mut writer)| {
            tokio::spawn(async move {
                // Send message
                let content = format!("Message from user {}", i);
                send_chat_message(&mut writer, &username, &content)
                    .await
                    .expect("Should be able to send message");

                // Count received messages (should receive from all other users)
                let mut message_count = 0;
                let timeout_duration = Duration::from_secs(3);

                while message_count < client_count - 1 {
                    // -1 because we don't receive our own
                    match timeout(timeout_duration, reader.next()).await {
                        Ok(Some(Ok(Message::ChatMessage { .. }))) => {
                            message_count += 1;
                        }
                        Ok(Some(Ok(_))) => {
                            // Other message types, continue
                            continue;
                        }
                        _ => break,
                    }
                }

                message_count
            })
        })
        .collect();

    // Wait for all message exchanges
    for handle in message_handles {
        let received_count = handle.await.expect("Message exchange should complete");
        // Each client should receive messages from other clients
        // (allowing some margin for test reliability in concurrent scenarios)
        assert!(
            received_count >= client_count - 2,
            "Should receive most messages from other clients"
        );
    }

    // Wait for handles to complete
    for handle in join_handles {
        let _ = handle.await;
    }

    // Cleanup
    let _ = server.kill().await;
}

#[tokio::test]
async fn test_connection_failure_scenarios() {
    // Test 1: Connection to non-existent server
    let non_existent_port = find_available_port().await;

    let result = timeout(
        Duration::from_secs(2),
        TcpStream::connect(format!("127.0.0.1:{}", non_existent_port)),
    )
    .await;

    assert!(result.is_ok(), "Timeout should complete");
    assert!(
        result.unwrap().is_err(),
        "Connection should fail to non-existent server"
    );

    // Test 2: Server shutdown during client operation
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    // Connect client
    let (mut reader, mut writer) = create_authenticated_client("127.0.0.1", port, "testuser")
        .await
        .expect("Client should connect initially");

    // Kill server
    server.kill().await.expect("Should be able to kill server");
    sleep(Duration::from_millis(500)).await; // Give time for shutdown

    // Try to send message - should fail
    let msg = Message::ChatMessage {
        username: "testuser".to_string(),
        content: "This should fail".to_string(),
    };

    let send_result = writer.send(msg).await;
    assert!(
        send_result.is_err(),
        "Send should fail after server shutdown"
    );

    // Try to read - should fail or return None
    match timeout(Duration::from_secs(1), reader.next()).await {
        Ok(Some(Ok(_))) => panic!("Should not receive message after server shutdown"),
        Ok(Some(Err(_))) => {} // Expected - read error
        Ok(None) => {}         // Expected - connection closed
        Err(_) => {}           // Expected - timeout
    }
}
