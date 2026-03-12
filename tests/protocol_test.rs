use simple_chat::protocol::{ClientEvent, ServerEvent};

#[test]
fn test_join_serialization() {
    let cmd = ClientEvent::Join {
        username: "alice".to_string(),
    };

    let json = serde_json::to_string(&cmd).unwrap();

    assert!(json.contains("alice"));
}

#[test]
fn test_send_serialization() {
    let cmd = ClientEvent::Send {
        message: "hello".to_string(),
    };

    let json = serde_json::to_string(&cmd).unwrap();

    assert!(json.contains("hello"));
}

#[test]
fn test_leave_serialization() {
    let cmd = ClientEvent::Leave;

    let json = serde_json::to_string(&cmd).unwrap();

    assert!(json.contains("Leave"));
}

#[test]
fn test_chat_event_deserialization() {
    let json = r#"{"Chat":{"user":"alice","message":"hello"}}"#;

    let event: ServerEvent = serde_json::from_str(json).unwrap();

    match event {
        ServerEvent::Chat { user, message } => {
            assert_eq!(user, "alice");
            assert_eq!(message, "hello");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_error_event() {
    let event = ServerEvent::Error {
        reason: "username already taken".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();

    assert!(json.contains("username already taken"));
}
