use server::{protocol::ServerMessage, Room};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::time::{timeout, Duration};

/// Stress test: many concurrent users
#[tokio::test]
async fn test_many_concurrent_users() {
    let room = Room::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn server task
    let room_clone = room.clone();
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let room = room_clone.clone();
                tokio::spawn(async move {
                    let (reader, mut writer) = stream.into_split();
                    let mut reader = BufReader::new(reader);
                    let mut line = String::new();

                    // Read JOIN command
                    if reader.read_line(&mut line).await.is_err() {
                        return;
                    }

                    if let Some(username) = line.trim().strip_prefix("JOIN ") {
                        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
                        match room.join(username.to_string(), tx).await {
                            Ok(_) => {
                                let _ = writer.write_all(b"OK\n").await;

                                // Handle incoming messages
                                let room_clone = room.clone();
                                let username = username.to_string();
                                tokio::spawn(async move {
                                    loop {
                                        line.clear();
                                        if reader.read_line(&mut line).await.is_err() {
                                            break;
                                        }
                                        if line.trim() == "LEAVE" {
                                            let _ = room_clone.leave(username.clone()).await;
                                            break;
                                        }
                                        if let Some(msg) = line.trim().strip_prefix("SEND ") {
                                            let _ = room_clone
                                                .broadcast(username.clone(), msg.to_string())
                                                .await;
                                        }
                                    }
                                });

                                // Send messages to client
                                while let Some(msg) = rx.recv().await {
                                    let formatted = msg.to_wire();
                                    if writer.write_all(formatted.as_bytes()).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = writer.write_all(format!("ERROR {}\n", e).as_bytes()).await;
                            }
                        }
                    }
                });
            }
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    const NUM_CLIENTS: usize = 100;
    let mut handles = Vec::new();

    // Connect many clients concurrently
    for i in 0..NUM_CLIENTS {
        let handle = tokio::spawn(async move {
            let stream = timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(addr))
                .await
                .expect("Connection timeout")
                .expect("Failed to connect");

            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            // Join
            writer
                .write_all(format!("JOIN user{}\n", i).as_bytes())
                .await
                .unwrap();

            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            assert_eq!(line.trim(), "OK");

            // Send a message
            writer
                .write_all(format!("SEND Hello from user{}!\n", i).as_bytes())
                .await
                .unwrap();

            // Read some messages (but not all, to avoid deadlock)
            for _ in 0..5 {
                line.clear();
                if timeout(Duration::from_millis(100), reader.read_line(&mut line))
                    .await
                    .is_ok()
                {
                    // Received a message
                }
            }

            // Leave
            writer.write_all(b"LEAVE\n").await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all clients to complete
    for handle in handles {
        timeout(Duration::from_secs(30), handle)
            .await
            .expect("Client task timeout")
            .expect("Client task failed");
    }
}

/// Stress test: high message throughput
#[tokio::test]
async fn test_high_message_throughput() {
    let room = Room::new();
    const NUM_CLIENTS: usize = 10;
    const MESSAGES_PER_CLIENT: usize = 100;

    let mut handles = Vec::new();

    // Create clients
    for i in 0..NUM_CLIENTS {
        let room = room.clone();
        let handle = tokio::spawn(async move {
            let (tx, mut rx) = tokio::sync::mpsc::channel(1000);
            let username = format!("user{}", i);

            // Join
            room.join(username.clone(), tx).await.unwrap();

            // Send many messages rapidly
            for j in 0..MESSAGES_PER_CLIENT {
                let _ = room
                    .broadcast(username.clone(), format!("Message {} from {}", j, username))
                    .await;
            }

            // Receive messages (non-blocking, just drain what's there)
            let mut received = 0;
            while timeout(Duration::from_millis(10), rx.recv()).await.is_ok() {
                received += 1;
            }

            // Leave
            let _ = room.leave(username).await;

            received
        });
        handles.push(handle);
    }

    // Wait for all tasks
    let start = std::time::Instant::now();
    for handle in handles {
        handle.await.unwrap();
    }
    let elapsed = start.elapsed();

    // Should complete in reasonable time (< 5 seconds for 1000 messages)
    assert!(
        elapsed < Duration::from_secs(5),
        "High throughput test took too long: {:?}",
        elapsed
    );

    println!(
        "✅ Processed {} messages from {} clients in {:?}",
        NUM_CLIENTS * MESSAGES_PER_CLIENT,
        NUM_CLIENTS,
        elapsed
    );
}

/// Stress test: rapid join/leave cycles
#[tokio::test]
async fn test_rapid_join_leave() {
    let room = Room::new();
    const NUM_CYCLES: usize = 100;

    let mut handles = Vec::new();

    for i in 0..NUM_CYCLES {
        let room = room.clone();
        let handle = tokio::spawn(async move {
            let (tx, _rx) = tokio::sync::mpsc::channel(10);
            // Use unique usernames to avoid conflicts
            let username = format!("rapiduser{}", i);

            // Join
            let result = room.join(username.clone(), tx).await;

            // If we joined successfully, leave immediately
            if result.is_ok() {
                let _ = room.leave(username).await;
            }
        });
        handles.push(handle);
    }

    // All should complete quickly
    let start = std::time::Instant::now();
    for handle in handles {
        timeout(Duration::from_secs(5), handle)
            .await
            .expect("Join/leave cycle timeout")
            .unwrap();
    }
    let elapsed = start.elapsed();

    println!(
        "✅ Completed {} join/leave cycles in {:?}",
        NUM_CYCLES, elapsed
    );
}

/// Stress test: message ordering and delivery
#[tokio::test]
async fn test_message_ordering() {
    let room = Room::new();

    // Create sender
    let (sender_tx, _sender_rx) = tokio::sync::mpsc::channel(10);
    room.join("sender".to_string(), sender_tx).await.unwrap();

    // Create receiver
    let (receiver_tx, mut receiver_rx) = tokio::sync::mpsc::channel(1000);
    room.join("receiver".to_string(), receiver_tx)
        .await
        .unwrap();

    // Send messages in order
    const NUM_MESSAGES: usize = 100;
    for i in 0..NUM_MESSAGES {
        let _ = room
            .broadcast("sender".to_string(), format!("Message {}", i))
            .await;
    }

    // Receive and verify ordering
    let mut received = Vec::new();
    for _ in 0..NUM_MESSAGES {
        if let Ok(Some(msg)) = timeout(Duration::from_secs(1), receiver_rx.recv()).await {
            received.push(msg);
        }
    }

    // Should receive all messages
    assert_eq!(received.len(), NUM_MESSAGES);

    // Messages should be in order
    for (i, msg) in received.iter().enumerate() {
        match msg {
            ServerMessage::Message { from, text } => {
                assert_eq!(from, "sender");
                assert_eq!(text, &format!("Message {}", i));
            }
            _ => panic!("Unexpected message type: {:?}", msg),
        }
    }

    println!("✅ All {} messages received in order", NUM_MESSAGES);
}
