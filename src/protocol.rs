use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientEvent {
    Join { username: String },
    Send { message: String },
    Leave,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerEvent {
    Chat { user: String, message: String },
    Error { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_join() {
        let cmd = ClientEvent::Join {
            username: "alice".to_string(),
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("alice"));
    }

    #[test]
    fn serialize_send() {
        let cmd = ClientEvent::Send {
            message: "hello".to_string(),
        };

        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("hello"));
    }

    #[test]
    fn deserialize_chat_event() {
        let json = r#"{"Chat":{"user":"alice","message":"hello"}}"#;

        let event: ServerEvent = serde_json::from_str(json).unwrap();

        match event {
            ServerEvent::Chat { user, message } => {
                assert_eq!(user, "alice");
                assert_eq!(message, "hello");
            }
            _ => panic!("wrong event type"),
        }
    }
}
