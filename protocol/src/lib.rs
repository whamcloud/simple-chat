use serde::{Deserialize, Serialize};

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Join { username: String },
    Leave,
    Send { content: String },
}

/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ServerMessage {
    Welcome { message: String },
    Error { message: String },
    UserJoined { username: String },
    UserLeft { username: String },
    Chat { username: String, content: String },
}

/// Encode a message as a newline-delimited JSON string.
pub fn encode<T: Serialize>(msg: &T) -> Result<String, serde_json::Error> {
    let mut s = serde_json::to_string(msg)?;
    s.push('\n');
    Ok(s)
}

/// Decode a message from a JSON string.
pub fn decode<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, serde_json::Error> {
    serde_json::from_str(s.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_join() {
        let msg = ClientMessage::Join {
            username: "alice".to_string(),
        };
        let encoded = encode(&msg).unwrap();
        assert!(encoded.ends_with('\n'));
        let decoded: ClientMessage = decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_encode_decode_send() {
        let msg = ClientMessage::Send {
            content: "hello world".to_string(),
        };
        let encoded = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_encode_decode_leave() {
        let msg = ClientMessage::Leave;
        let encoded = encode(&msg).unwrap();
        let decoded: ClientMessage = decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_encode_decode_server_chat() {
        let msg = ServerMessage::Chat {
            username: "bob".to_string(),
            content: "hi there".to_string(),
        };
        let encoded = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_encode_decode_server_welcome() {
        let msg = ServerMessage::Welcome {
            message: "Welcome!".to_string(),
        };
        let encoded = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_encode_decode_server_error() {
        let msg = ServerMessage::Error {
            message: "username taken".to_string(),
        };
        let encoded = encode(&msg).unwrap();
        let decoded: ServerMessage = decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_decode_invalid_json() {
        let result: Result<ClientMessage, _> = decode("not json");
        assert!(result.is_err());
    }
}
