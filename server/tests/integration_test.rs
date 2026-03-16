use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::sleep;

use server_lib::room::Room;

async fn start_server() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let room = std::sync::Arc::new(Room::new());
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let room = std::sync::Arc::clone(&room);
            tokio::spawn(handle_connection(stream, room));
        }
    });

    sleep(Duration::from_millis(50)).await;
    port
}

async fn handle_connection(stream: tokio::net::TcpStream, room: std::sync::Arc<Room>) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    let n = reader.read_line(&mut line).await.unwrap();
    if n == 0 {
        return;
    }

    let username = match protocol::decode::<protocol::ClientMessage>(&line).unwrap() {
        protocol::ClientMessage::Join { username } => username,
        _ => return,
    };

    let mut rx = match room.join(&username) {
        Ok(rx) => rx,
        Err(msg) => {
            let err = protocol::encode(&protocol::ServerMessage::Error { message: msg }).unwrap();
            writer.write_all(err.as_bytes()).await.unwrap();
            return;
        }
    };

    let welcome = protocol::encode(&protocol::ServerMessage::Welcome {
        message: format!("Welcome, {username}!"),
    })
    .unwrap();
    writer.write_all(welcome.as_bytes()).await.unwrap();

    let write_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let encoded = protocol::encode(&msg).unwrap();
            if writer.write_all(encoded.as_bytes()).await.is_err() {
                break;
            }
        }
    });

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        match protocol::decode::<protocol::ClientMessage>(&line) {
            Ok(protocol::ClientMessage::Send { content }) => {
                room.broadcast(&username, content);
            }
            Ok(protocol::ClientMessage::Leave) => break,
            _ => {}
        }
    }

    room.leave(&username);
    write_task.abort();
}

async fn connect_and_join(
    port: u16,
    username: &str,
) -> (
    tokio::net::tcp::OwnedWriteHalf,
    BufReader<tokio::net::tcp::OwnedReadHalf>,
) {
    let stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let join_msg = protocol::encode(&protocol::ClientMessage::Join {
        username: username.to_string(),
    })
    .unwrap();
    writer.write_all(join_msg.as_bytes()).await.unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let msg: protocol::ServerMessage = protocol::decode(&line).unwrap();
    match msg {
        protocol::ServerMessage::Welcome { .. } => {}
        protocol::ServerMessage::Error { message } => panic!("Join failed: {message}"),
        other => panic!("Unexpected message: {other:?}"),
    }

    (writer, reader)
}

#[tokio::test]
async fn test_single_user_join() {
    let port = start_server().await;
    let (_writer, _reader) = connect_and_join(port, "alice").await;
}

#[tokio::test]
async fn test_duplicate_username_rejected() {
    let port = start_server().await;
    let (_w1, _r1) = connect_and_join(port, "alice").await;

    let stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let join_msg = protocol::encode(&protocol::ClientMessage::Join {
        username: "alice".to_string(),
    })
    .unwrap();
    writer.write_all(join_msg.as_bytes()).await.unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let msg: protocol::ServerMessage = protocol::decode(&line).unwrap();
    match msg {
        protocol::ServerMessage::Error { message } => {
            assert!(message.contains("already taken"));
        }
        other => panic!("Expected error, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_message_broadcast() {
    let port = start_server().await;
    let (mut w_alice, _r_alice) = connect_and_join(port, "alice").await;

    sleep(Duration::from_millis(50)).await;

    let (_w_bob, mut r_bob) = connect_and_join(port, "bob").await;

    sleep(Duration::from_millis(50)).await;

    let send_msg = protocol::encode(&protocol::ClientMessage::Send {
        content: "hello bob".to_string(),
    })
    .unwrap();
    w_alice.write_all(send_msg.as_bytes()).await.unwrap();

    let mut line = String::new();
    r_bob.read_line(&mut line).await.unwrap();
    let msg: protocol::ServerMessage = protocol::decode(&line).unwrap();
    match msg {
        protocol::ServerMessage::Chat { username, content } => {
            assert_eq!(username, "alice");
            assert_eq!(content, "hello bob");
        }
        other => panic!("Expected Chat, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_leave_notification() {
    let port = start_server().await;
    let (mut w_alice, _r_alice) = connect_and_join(port, "alice").await;

    sleep(Duration::from_millis(50)).await;

    let (_w_bob, mut r_bob) = connect_and_join(port, "bob").await;

    sleep(Duration::from_millis(50)).await;

    let leave_msg = protocol::encode(&protocol::ClientMessage::Leave).unwrap();
    w_alice.write_all(leave_msg.as_bytes()).await.unwrap();

    let mut line = String::new();
    r_bob.read_line(&mut line).await.unwrap();
    let msg: protocol::ServerMessage = protocol::decode(&line).unwrap();
    match msg {
        protocol::ServerMessage::UserLeft { username } => {
            assert_eq!(username, "alice");
        }
        other => panic!("Expected UserLeft, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_join_notification() {
    let port = start_server().await;
    let (_w_alice, mut r_alice) = connect_and_join(port, "alice").await;

    sleep(Duration::from_millis(50)).await;

    let (_w_bob, _r_bob) = connect_and_join(port, "bob").await;

    let mut line = String::new();
    r_alice.read_line(&mut line).await.unwrap();
    let msg: protocol::ServerMessage = protocol::decode(&line).unwrap();
    match msg {
        protocol::ServerMessage::UserJoined { username } => {
            assert_eq!(username, "bob");
        }
        other => panic!("Expected UserJoined, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_sender_does_not_receive_own_message() {
    let port = start_server().await;
    let (mut w_alice, mut r_alice) = connect_and_join(port, "alice").await;

    sleep(Duration::from_millis(50)).await;

    let (_w_bob, mut r_bob) = connect_and_join(port, "bob").await;

    // Drain alice's join notification for bob
    let mut line = String::new();
    r_alice.read_line(&mut line).await.unwrap();

    sleep(Duration::from_millis(50)).await;

    let send_msg = protocol::encode(&protocol::ClientMessage::Send {
        content: "test".to_string(),
    })
    .unwrap();
    w_alice.write_all(send_msg.as_bytes()).await.unwrap();

    let mut bob_line = String::new();
    r_bob.read_line(&mut bob_line).await.unwrap();
    let bob_msg: protocol::ServerMessage = protocol::decode(&bob_line).unwrap();
    assert!(matches!(bob_msg, protocol::ServerMessage::Chat { .. }));

    let result = tokio::time::timeout(Duration::from_millis(200), async {
        let mut alice_line = String::new();
        r_alice.read_line(&mut alice_line).await
    })
    .await;

    assert!(result.is_err(), "Alice should not receive her own message");
}

#[tokio::test]
async fn test_multiple_users_chat() {
    let port = start_server().await;
    let (mut w_alice, mut r_alice) = connect_and_join(port, "alice").await;

    sleep(Duration::from_millis(50)).await;

    let (mut w_bob, mut r_bob) = connect_and_join(port, "bob").await;

    // Drain alice's join notification
    let mut line = String::new();
    r_alice.read_line(&mut line).await.unwrap();

    sleep(Duration::from_millis(50)).await;

    let (_w_charlie, mut r_charlie) = connect_and_join(port, "charlie").await;

    // Drain join notifications
    let mut a_line = String::new();
    r_alice.read_line(&mut a_line).await.unwrap();
    let mut b_line = String::new();
    r_bob.read_line(&mut b_line).await.unwrap();

    sleep(Duration::from_millis(50)).await;

    // Alice sends a message
    let send_msg = protocol::encode(&protocol::ClientMessage::Send {
        content: "hi everyone".to_string(),
    })
    .unwrap();
    w_alice.write_all(send_msg.as_bytes()).await.unwrap();

    // Both bob and charlie receive it
    let mut bob_line = String::new();
    r_bob.read_line(&mut bob_line).await.unwrap();
    let bob_msg: protocol::ServerMessage = protocol::decode(&bob_line).unwrap();
    match bob_msg {
        protocol::ServerMessage::Chat { username, content } => {
            assert_eq!(username, "alice");
            assert_eq!(content, "hi everyone");
        }
        other => panic!("Expected Chat for bob, got: {other:?}"),
    }

    let mut charlie_line = String::new();
    r_charlie.read_line(&mut charlie_line).await.unwrap();
    let charlie_msg: protocol::ServerMessage = protocol::decode(&charlie_line).unwrap();
    match charlie_msg {
        protocol::ServerMessage::Chat { username, content } => {
            assert_eq!(username, "alice");
            assert_eq!(content, "hi everyone");
        }
        other => panic!("Expected Chat for charlie, got: {other:?}"),
    }

    // Bob replies
    let reply = protocol::encode(&protocol::ClientMessage::Send {
        content: "hey alice".to_string(),
    })
    .unwrap();
    w_bob.write_all(reply.as_bytes()).await.unwrap();

    let mut a2 = String::new();
    r_alice.read_line(&mut a2).await.unwrap();
    let a2_msg: protocol::ServerMessage = protocol::decode(&a2).unwrap();
    match a2_msg {
        protocol::ServerMessage::Chat { username, content } => {
            assert_eq!(username, "bob");
            assert_eq!(content, "hey alice");
        }
        other => panic!("Expected Chat for alice, got: {other:?}"),
    }

    let mut c2 = String::new();
    r_charlie.read_line(&mut c2).await.unwrap();
    let c2_msg: protocol::ServerMessage = protocol::decode(&c2).unwrap();
    match c2_msg {
        protocol::ServerMessage::Chat { username, content } => {
            assert_eq!(username, "bob");
            assert_eq!(content, "hey alice");
        }
        other => panic!("Expected Chat for charlie, got: {other:?}"),
    }
}
