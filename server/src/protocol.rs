/// Protocol messages that clients send to the server
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientMessage {
    Join(String),
    Send(String),
    Leave,
}

/// Protocol messages that server sends to clients
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerMessage {
    Ok,
    Error(String),
    Message { from: String, text: String },
    Joined(String),
    Left(String),
}

impl ClientMessage {
    /// Parse a client message from a line of text
    pub fn parse(line: &str) -> Result<Self, String> {
        // Trim only leading/trailing whitespace, preserve internal structure
        let line = line.trim();

        if let Some(rest) = line.strip_prefix("JOIN") {
            let username = rest.trim();
            if username.is_empty() {
                return Err("Username cannot be empty".to_string());
            }
            Ok(ClientMessage::Join(username.to_string()))
        } else if let Some(rest) = line.strip_prefix("SEND") {
            let message = rest.trim();
            if message.is_empty() {
                return Err("Message cannot be empty".to_string());
            }
            Ok(ClientMessage::Send(message.to_string()))
        } else if line == "LEAVE" {
            Ok(ClientMessage::Leave)
        } else {
            Err(format!("Unknown command: {}", line))
        }
    }

    /// Serialize to wire format
    pub fn to_wire(&self) -> String {
        match self {
            ClientMessage::Join(username) => format!("JOIN {}\n", username),
            ClientMessage::Send(text) => format!("SEND {}\n", text),
            ClientMessage::Leave => "LEAVE\n".to_string(),
        }
    }
}

impl ServerMessage {
    /// Parse a server message from a line of text
    pub fn parse(line: &str) -> Result<Self, String> {
        let line = line.trim();

        if line == "OK" {
            Ok(ServerMessage::Ok)
        } else if let Some(reason) = line.strip_prefix("ERROR ") {
            Ok(ServerMessage::Error(reason.to_string()))
        } else if let Some(rest) = line.strip_prefix("MESSAGE ") {
            if let Some(colon_pos) = rest.find(':') {
                let from = rest[..colon_pos].trim().to_string();
                let text = rest[colon_pos + 1..].trim().to_string();
                Ok(ServerMessage::Message { from, text })
            } else {
                Err("Invalid MESSAGE format".to_string())
            }
        } else if let Some(username) = line.strip_prefix("JOINED ") {
            Ok(ServerMessage::Joined(username.trim().to_string()))
        } else if let Some(username) = line.strip_prefix("LEFT ") {
            Ok(ServerMessage::Left(username.trim().to_string()))
        } else {
            Err(format!("Unknown server message: {}", line))
        }
    }

    /// Serialize to wire format
    pub fn to_wire(&self) -> String {
        match self {
            ServerMessage::Ok => "OK\n".to_string(),
            ServerMessage::Error(reason) => format!("ERROR {}\n", reason),
            ServerMessage::Message { from, text } => format!("MESSAGE {}: {}\n", from, text),
            ServerMessage::Joined(username) => format!("JOINED {}\n", username),
            ServerMessage::Left(username) => format!("LEFT {}\n", username),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Client Message Parsing Tests
    #[test]
    fn test_parse_join() {
        let msg = ClientMessage::parse("JOIN alice").unwrap();
        assert_eq!(msg, ClientMessage::Join("alice".to_string()));
    }

    #[test]
    fn test_parse_join_with_whitespace() {
        let msg = ClientMessage::parse("  JOIN   bob  ").unwrap();
        assert_eq!(msg, ClientMessage::Join("bob".to_string()));
    }

    #[test]
    fn test_parse_join_empty_username() {
        let result = ClientMessage::parse("JOIN ");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Username cannot be empty");
    }

    #[test]
    fn test_parse_send() {
        let msg = ClientMessage::parse("SEND hello world").unwrap();
        assert_eq!(msg, ClientMessage::Send("hello world".to_string()));
    }

    #[test]
    fn test_parse_send_empty() {
        let result = ClientMessage::parse("SEND ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_leave() {
        let msg = ClientMessage::parse("LEAVE").unwrap();
        assert_eq!(msg, ClientMessage::Leave);
    }

    #[test]
    fn test_parse_unknown_command() {
        let result = ClientMessage::parse("INVALID");
        assert!(result.is_err());
    }

    // Client Message Serialization Tests
    #[test]
    fn test_join_to_wire() {
        let msg = ClientMessage::Join("alice".to_string());
        assert_eq!(msg.to_wire(), "JOIN alice\n");
    }

    #[test]
    fn test_send_to_wire() {
        let msg = ClientMessage::Send("hello world".to_string());
        assert_eq!(msg.to_wire(), "SEND hello world\n");
    }

    #[test]
    fn test_leave_to_wire() {
        let msg = ClientMessage::Leave;
        assert_eq!(msg.to_wire(), "LEAVE\n");
    }

    // Server Message Parsing Tests
    #[test]
    fn test_parse_ok() {
        let msg = ServerMessage::parse("OK").unwrap();
        assert_eq!(msg, ServerMessage::Ok);
    }

    #[test]
    fn test_parse_error() {
        let msg = ServerMessage::parse("ERROR username already taken").unwrap();
        assert_eq!(
            msg,
            ServerMessage::Error("username already taken".to_string())
        );
    }

    #[test]
    fn test_parse_message() {
        let msg = ServerMessage::parse("MESSAGE alice: hello world").unwrap();
        assert_eq!(
            msg,
            ServerMessage::Message {
                from: "alice".to_string(),
                text: "hello world".to_string()
            }
        );
    }

    #[test]
    fn test_parse_joined() {
        let msg = ServerMessage::parse("JOINED bob").unwrap();
        assert_eq!(msg, ServerMessage::Joined("bob".to_string()));
    }

    #[test]
    fn test_parse_left() {
        let msg = ServerMessage::parse("LEFT charlie").unwrap();
        assert_eq!(msg, ServerMessage::Left("charlie".to_string()));
    }

    // Server Message Serialization Tests
    #[test]
    fn test_ok_to_wire() {
        let msg = ServerMessage::Ok;
        assert_eq!(msg.to_wire(), "OK\n");
    }

    #[test]
    fn test_error_to_wire() {
        let msg = ServerMessage::Error("invalid command".to_string());
        assert_eq!(msg.to_wire(), "ERROR invalid command\n");
    }

    #[test]
    fn test_message_to_wire() {
        let msg = ServerMessage::Message {
            from: "alice".to_string(),
            text: "hello".to_string(),
        };
        assert_eq!(msg.to_wire(), "MESSAGE alice: hello\n");
    }

    #[test]
    fn test_joined_to_wire() {
        let msg = ServerMessage::Joined("bob".to_string());
        assert_eq!(msg.to_wire(), "JOINED bob\n");
    }

    #[test]
    fn test_left_to_wire() {
        let msg = ServerMessage::Left("charlie".to_string());
        assert_eq!(msg.to_wire(), "LEFT charlie\n");
    }

    // Round-trip tests
    #[test]
    fn test_client_message_roundtrip() {
        let messages = vec![
            ClientMessage::Join("alice".to_string()),
            ClientMessage::Send("test message".to_string()),
            ClientMessage::Leave,
        ];

        for msg in messages {
            let wire = msg.to_wire();
            let parsed = ClientMessage::parse(wire.trim()).unwrap();
            assert_eq!(msg, parsed);
        }
    }

    #[test]
    fn test_server_message_roundtrip() {
        let messages = vec![
            ServerMessage::Ok,
            ServerMessage::Error("test error".to_string()),
            ServerMessage::Message {
                from: "alice".to_string(),
                text: "hello".to_string(),
            },
            ServerMessage::Joined("bob".to_string()),
            ServerMessage::Left("charlie".to_string()),
        ];

        for msg in messages {
            let wire = msg.to_wire();
            let parsed = ServerMessage::parse(wire.trim()).unwrap();
            assert_eq!(msg, parsed);
        }
    }
}
