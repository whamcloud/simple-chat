use server::{ClientMessage, Room, ServerMessage};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_room_integration() {
    let room = Room::new();

    // Create three users
    let (tx1, mut rx1) = mpsc::channel(10);
    let (tx2, mut rx2) = mpsc::channel(10);
    let (tx3, mut rx3) = mpsc::channel(10);

    // Join all users
    assert!(room.join("alice".to_string(), tx1).await.is_ok());
    assert!(room.join("bob".to_string(), tx2).await.is_ok());
    assert!(room.join("charlie".to_string(), tx3).await.is_ok());

    // Give time for notifications
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Clear join notifications
    while rx1.try_recv().is_ok() {}
    while rx2.try_recv().is_ok() {}
    while rx3.try_recv().is_ok() {}

    // Test broadcast
    room.broadcast("alice".to_string(), "Hello everyone!".to_string())
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify bob and charlie received the message
    assert!(matches!(rx2.try_recv(), Ok(ServerMessage::Message { from, .. }) if from == "alice"));
    assert!(matches!(rx3.try_recv(), Ok(ServerMessage::Message { from, .. }) if from == "alice"));

    // Verify alice did NOT receive her own message
    assert!(rx1.try_recv().is_err());
}

#[tokio::test]
async fn test_protocol_integration() {
    // Test full protocol round-trip
    let join = ClientMessage::Join("testuser".to_string());
    let wire = join.to_wire();
    let parsed = ClientMessage::parse(&wire).unwrap();
    assert_eq!(join, parsed);

    let msg = ServerMessage::Message {
        from: "alice".to_string(),
        text: "test".to_string(),
    };
    let wire = msg.to_wire();
    let parsed = ServerMessage::parse(&wire).unwrap();
    assert_eq!(msg, parsed);
}

#[tokio::test]
async fn test_multiple_users_scenario() {
    let room = Room::new();

    // Scenario: Users join, chat, and leave
    let (tx1, mut rx1) = mpsc::channel(10);
    let (tx2, mut rx2) = mpsc::channel(10);

    // Alice joins
    room.join("alice".to_string(), tx1).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Bob joins (alice should be notified)
    room.join("bob".to_string(), tx2).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Alice should have received bob's join notification
    assert!(matches!(
        rx1.try_recv(),
        Ok(ServerMessage::Joined(username)) if username == "bob"
    ));

    // Alice sends a message
    room.broadcast("alice".to_string(), "Hi Bob!".to_string())
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Bob receives alice's message
    assert!(matches!(
        rx2.try_recv(),
        Ok(ServerMessage::Message { from, text }) if from == "alice" && text == "Hi Bob!"
    ));

    // Alice leaves
    room.leave("alice".to_string()).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Bob should be notified of alice leaving
    assert!(matches!(
        rx2.try_recv(),
        Ok(ServerMessage::Left(username)) if username == "alice"
    ));
}

#[test]
fn test_client_message_validation() {
    // Test invalid commands
    assert!(ClientMessage::parse("").is_err());
    assert!(ClientMessage::parse("INVALID").is_err());
    assert!(ClientMessage::parse("JOIN").is_err());
    assert!(ClientMessage::parse("SEND").is_err());

    // Test valid commands
    assert!(ClientMessage::parse("JOIN alice").is_ok());
    assert!(ClientMessage::parse("SEND hello").is_ok());
    assert!(ClientMessage::parse("LEAVE").is_ok());
}

#[test]
fn test_server_message_validation() {
    // Test invalid messages
    assert!(ServerMessage::parse("").is_err());
    assert!(ServerMessage::parse("INVALID").is_err());

    // Test valid messages
    assert!(ServerMessage::parse("OK").is_ok());
    assert!(ServerMessage::parse("ERROR test").is_ok());
    assert!(ServerMessage::parse("MESSAGE alice: hello").is_ok());
    assert!(ServerMessage::parse("JOINED bob").is_ok());
    assert!(ServerMessage::parse("LEFT charlie").is_ok());
}
