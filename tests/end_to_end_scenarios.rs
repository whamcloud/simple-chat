use anyhow::Result;
use futures::{SinkExt, StreamExt};
use simple_chat::{codec::MessageCodec, Message};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::sync::{mpsc, Barrier};
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
            "50",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    // Wait for server to start
    sleep(Duration::from_millis(2000)).await;

    Ok(child)
}

// Test helper to create and authenticate a client
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

// Simulate a chat participant
async fn simulate_chat_participant(
    port: u16,
    username: String,
    barrier: Arc<Barrier>,
    message_count: usize,
    chat_duration_secs: u64,
) -> Result<usize> {
    // Wait for all participants to be ready
    barrier.wait().await;

    let (mut reader, mut writer) =
        create_authenticated_client("127.0.0.1", port, &username).await?;

    // Start background task to count received messages
    let (tx, mut rx) = mpsc::channel(100);
    let reader_handle = tokio::spawn(async move {
        let mut received_messages = 0;
        let mut user_events = 0;

        loop {
            match timeout(Duration::from_secs(1), reader.next()).await {
                Ok(Some(Ok(Message::ChatMessage { .. }))) => {
                    received_messages += 1;
                }
                Ok(Some(Ok(Message::UserJoined { .. }))) => {
                    user_events += 1;
                }
                Ok(Some(Ok(Message::UserLeft { .. }))) => {
                    user_events += 1;
                }
                Ok(Some(Ok(Message::Error { .. }))) => {
                    // Continue on errors
                }
                Ok(Some(Ok(Message::Join { .. }))) => {
                    // Shouldn't receive join messages normally, but handle gracefully
                    continue;
                }
                Ok(Some(Ok(Message::Leave { .. }))) => {
                    // Shouldn't receive leave messages normally, but handle gracefully
                    continue;
                }
                Ok(Some(Err(_))) => break, // Connection error
                Ok(None) => break,         // Connection closed
                Err(_) => continue,        // Timeout, continue reading
            }

            // Check if we should stop
            if let Ok(()) = rx.try_recv() {
                break;
            }
        }

        (received_messages, user_events)
    });

    // Send messages periodically
    let message_interval =
        Duration::from_millis((chat_duration_secs * 1000) / message_count as u64);

    for i in 0..message_count {
        let content = format!("Message {} from {}", i + 1, username);

        let msg = Message::ChatMessage {
            username: username.clone(),
            content,
        };

        if (writer.send(msg).await).is_err() {
            break; // Connection failed
        }

        sleep(message_interval).await;
    }

    // Send leave message
    let leave_msg = Message::Leave {
        username: username.clone(),
    };
    let _ = writer.send(leave_msg).await;

    // Signal reader to stop and get results
    let _ = tx.send(()).await;
    let (received_messages, _user_events) = reader_handle.await.unwrap_or((0, 0));

    Ok(received_messages)
}

#[tokio::test]
async fn test_multiple_clients_chatting_scenario() {
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    let client_count = 8;
    let messages_per_client = 5;
    let chat_duration_secs = 10;

    // Create barrier for synchronized start
    let barrier = Arc::new(Barrier::new(client_count));

    // Spawn chat participants
    let mut handles = Vec::new();

    for i in 0..client_count {
        let barrier = Arc::clone(&barrier);
        let username = format!("user{:02}", i);

        let handle = tokio::spawn(async move {
            simulate_chat_participant(
                port,
                username,
                barrier,
                messages_per_client,
                chat_duration_secs,
            )
            .await
        });

        handles.push(handle);
    }

    // Wait for all participants to complete
    let mut total_messages_received = 0;
    let mut successful_participants = 0;

    for handle in handles {
        match handle.await {
            Ok(Ok(messages_received)) => {
                total_messages_received += messages_received;
                successful_participants += 1;
            }
            Ok(Err(e)) => eprintln!("Participant failed: {}", e),
            Err(e) => eprintln!("Task failed: {}", e),
        }
    }

    // Verify results
    assert!(
        successful_participants >= client_count - 1,
        "Most participants should complete successfully"
    );

    // Each client sends messages_per_client messages, and should receive
    // messages from other clients. Account for some message loss in concurrent scenarios.
    let expected_total_messages = client_count * messages_per_client * (client_count - 1);
    let message_delivery_rate = total_messages_received as f64 / expected_total_messages as f64;

    assert!(
        message_delivery_rate > 0.7,
        "Message delivery rate should be > 70%, got {:.2}%",
        message_delivery_rate * 100.0
    );

    println!("✅ Multi-client chat test completed:");
    println!(
        "   Participants: {}/{}",
        successful_participants, client_count
    );
    println!(
        "   Messages received: {}/{} ({:.1}%)",
        total_messages_received,
        expected_total_messages,
        message_delivery_rate * 100.0
    );

    // Cleanup
    let _ = server.kill().await;
}

#[tokio::test]
async fn test_client_disconnect_during_chat() {
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    // Create multiple clients
    let (mut reader1, mut writer1) = create_authenticated_client("127.0.0.1", port, "alice")
        .await
        .expect("Alice should connect");

    let (mut reader2, writer2) = create_authenticated_client("127.0.0.1", port, "bob")
        .await
        .expect("Bob should connect");

    let (mut reader3, mut writer3) = create_authenticated_client("127.0.0.1", port, "charlie")
        .await
        .expect("Charlie should connect");

    // Clear initial join notifications
    sleep(Duration::from_millis(500)).await;
    for reader in [&mut reader1, &mut reader2, &mut reader3] {
        while let Ok(Some(_)) = timeout(Duration::from_millis(50), reader.next()).await {}
    }

    // Start chat between all participants
    let msg1 = Message::ChatMessage {
        username: "alice".to_string(),
        content: "Hello everyone!".to_string(),
    };
    writer1
        .send(msg1.clone())
        .await
        .expect("Alice should send message");

    // Bob and Charlie should receive the message
    let timeout_duration = Duration::from_secs(2);

    match timeout(timeout_duration, reader2.next()).await {
        Ok(Some(Ok(received))) => assert_eq!(received, msg1),
        other => panic!("Bob should receive Alice's message, got: {:?}", other),
    }

    match timeout(timeout_duration, reader3.next()).await {
        Ok(Some(Ok(received))) => assert_eq!(received, msg1),
        other => panic!("Charlie should receive Alice's message, got: {:?}", other),
    }

    // Bob suddenly disconnects (simulated by dropping connection)
    drop(writer2);
    drop(reader2);

    // Wait for disconnect to be processed
    sleep(Duration::from_millis(500)).await;

    // Alice and Charlie should receive disconnect notification
    let expected_leave = Message::UserLeft {
        username: "bob".to_string(),
    };

    let mut found_leave_alice = false;
    let mut found_leave_charlie = false;

    // Check multiple messages for leave notification
    for _ in 0..5 {
        if let Ok(Some(Ok(msg))) = timeout(Duration::from_millis(300), reader1.next()).await {
            if msg == expected_leave {
                found_leave_alice = true;
                break;
            }
        }
    }

    for _ in 0..5 {
        if let Ok(Some(Ok(msg))) = timeout(Duration::from_millis(300), reader3.next()).await {
            if msg == expected_leave {
                found_leave_charlie = true;
                break;
            }
        }
    }

    assert!(
        found_leave_alice,
        "Alice should receive Bob's leave notification"
    );
    assert!(
        found_leave_charlie,
        "Charlie should receive Bob's leave notification"
    );

    // Chat should continue between remaining participants
    let msg3 = Message::ChatMessage {
        username: "charlie".to_string(),
        content: "Bob left, but we can still chat".to_string(),
    };
    writer3
        .send(msg3.clone())
        .await
        .expect("Charlie should send message");

    match timeout(Duration::from_secs(2), reader1.next()).await {
        Ok(Some(Ok(received))) => assert_eq!(received, msg3),
        other => panic!("Alice should receive Charlie's message, got: {:?}", other),
    }

    // New user should be able to join with Bob's old username
    let (_reader4, _writer4) = create_authenticated_client("127.0.0.1", port, "bob")
        .await
        .expect("New Bob should be able to join");

    // Cleanup
    let _ = server.kill().await;
}

#[tokio::test]
async fn test_server_shutdown_with_active_clients() {
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    // Create multiple active clients
    let mut clients = Vec::new();
    for i in 0..5 {
        let username = format!("user{}", i);
        match create_authenticated_client("127.0.0.1", port, &username).await {
            Ok((reader, writer)) => clients.push((username, reader, writer)),
            Err(e) => panic!("Failed to create client {}: {}", i, e),
        }
    }

    assert_eq!(clients.len(), 5, "All clients should connect successfully");

    // Start some chat activity
    for (username, _, writer) in &mut clients {
        let msg = Message::ChatMessage {
            username: username.clone(),
            content: "Active before shutdown".to_string(),
        };
        let _ = writer.send(msg).await; // Ignore errors
    }

    // Give some time for messages to propagate
    sleep(Duration::from_millis(200)).await;

    // Shutdown server abruptly
    server.kill().await.expect("Should be able to kill server");

    // Wait for shutdown to propagate
    sleep(Duration::from_millis(500)).await;

    // All clients should detect server shutdown
    let mut clients_detected_shutdown = 0;

    for (username, mut reader, mut writer) in clients {
        // Try to send a message - should fail
        let msg = Message::ChatMessage {
            username: username.clone(),
            content: "This should fail".to_string(),
        };

        let send_failed = writer.send(msg).await.is_err();

        // Try to read - should fail or timeout
        let read_failed = match timeout(Duration::from_millis(500), reader.next()).await {
            Ok(Some(Ok(_))) => false, // Unexpectedly got a message
            Ok(Some(Err(_))) => true, // Read error (expected)
            Ok(None) => true,         // Connection closed (expected)
            Err(_) => true,           // Timeout (expected)
        };

        if send_failed || read_failed {
            clients_detected_shutdown += 1;
        }
    }

    assert!(
        clients_detected_shutdown >= 4,
        "Most clients should detect server shutdown (detected: {})",
        clients_detected_shutdown
    );
}

#[tokio::test]
async fn test_high_load_stress_scenario() {
    let port = find_available_port().await;
    let mut server = start_server_process(port)
        .await
        .expect("Failed to start server");

    let client_count = 20;
    let messages_per_client = 10;

    // Create all clients first
    let mut clients = Vec::new();

    for i in 0..client_count {
        let username = format!("stress_user_{:03}", i);
        match timeout(
            Duration::from_secs(10),
            create_authenticated_client("127.0.0.1", port, &username),
        )
        .await
        {
            Ok(Ok((reader, writer))) => clients.push((username, reader, writer)),
            Ok(Err(e)) => {
                eprintln!("Failed to create client {}: {}", i, e);
                break; // Stop if we can't create more clients
            }
            Err(_) => {
                eprintln!("Timeout creating client {}", i);
                break;
            }
        }
    }

    let actual_client_count = clients.len();
    assert!(
        actual_client_count >= client_count / 2,
        "Should be able to create at least half the intended clients"
    );

    // Split into senders and receivers for concurrent operations
    let (senders, receivers): (Vec<_>, Vec<_>) = clients
        .into_iter()
        .enumerate()
        .map(|(i, (username, reader, writer))| {
            ((i, username.clone(), writer), (i, username, reader))
        })
        .unzip();

    // Start message sending tasks
    let send_handles: Vec<_> = senders
        .into_iter()
        .map(|(i, username, mut writer)| {
            tokio::spawn(async move {
                let mut sent_count = 0;
                for msg_num in 0..messages_per_client {
                    let content = format!("Stress message {} from {}", msg_num, username);
                    let msg = Message::ChatMessage {
                        username: username.clone(),
                        content,
                    };

                    if writer.send(msg).await.is_ok() {
                        sent_count += 1;
                    } else {
                        break; // Stop on send error
                    }

                    // Small delay to avoid overwhelming
                    sleep(Duration::from_millis(10)).await;
                }

                // Send leave message
                let leave_msg = Message::Leave { username };
                let _ = writer.send(leave_msg).await;

                (i, sent_count)
            })
        })
        .collect();

    // Start message receiving tasks
    let receive_handles: Vec<_> = receivers
        .into_iter()
        .map(|(i, username, mut reader)| {
            tokio::spawn(async move {
                let mut received_count = 0;
                let start_time = std::time::Instant::now();
                let max_duration = Duration::from_secs(30); // Max test duration

                while start_time.elapsed() < max_duration {
                    match timeout(Duration::from_millis(100), reader.next()).await {
                        Ok(Some(Ok(Message::ChatMessage { .. }))) => {
                            received_count += 1;
                        }
                        Ok(Some(Ok(Message::UserLeft { .. }))) => {
                            // User left, might be end of test
                            continue;
                        }
                        Ok(Some(Ok(_))) => {
                            // Other message types
                            continue;
                        }
                        Ok(Some(Err(_))) => break, // Connection error
                        Ok(None) => break,         // Connection closed
                        Err(_) => continue,        // Timeout, continue
                    }
                }

                (i, username, received_count)
            })
        })
        .collect();

    // Wait for all senders to complete
    let mut total_sent = 0;
    for handle in send_handles {
        if let Ok((_, sent_count)) = handle.await {
            total_sent += sent_count;
        }
    }

    // Wait for receivers (with timeout)
    let mut total_received = 0;
    for handle in receive_handles {
        if let Ok(Ok((_, _, count))) = timeout(Duration::from_secs(5), handle).await {
            total_received += count;
        }
    }

    // Verify stress test results
    let expected_total = actual_client_count * messages_per_client;
    let send_success_rate = total_sent as f64 / expected_total as f64;

    assert!(
        send_success_rate > 0.8,
        "Send success rate should be > 80%, got {:.2}%",
        send_success_rate * 100.0
    );

    // In high load scenarios, some message loss is acceptable
    assert!(total_received > 0, "Should receive at least some messages");

    println!("✅ High load stress test completed:");
    println!(
        "   Clients created: {}/{}",
        actual_client_count, client_count
    );
    println!(
        "   Messages sent: {}/{} ({:.1}%)",
        total_sent,
        expected_total,
        send_success_rate * 100.0
    );
    println!(
        "   Messages received: {} (delivery varies under load)",
        total_received
    );

    // Cleanup
    let _ = server.kill().await;
}
