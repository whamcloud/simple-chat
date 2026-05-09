use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Send { text: String },
    Leave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Welcome { username: String },
    UsernameTaken { username: String },
    Message { from: String, text: String },
    UserJoined { username: String },
    UserLeft { username: String },
    Error { reason: String },
}

pub fn encode_client(msg: &ClientMsg) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

pub fn decode_client(s: &str) -> Result<ClientMsg, serde_json::Error> {
    serde_json::from_str(s)
}

pub fn encode_server(msg: &ServerMsg) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

pub fn decode_server(s: &str) -> Result<ServerMsg, serde_json::Error> {
    serde_json::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_send_roundtrip() {
        let msg = ClientMsg::Send {
            text: "hello world".into(),
        };
        let json = encode_client(&msg).unwrap();
        assert_eq!(decode_client(&json).unwrap(), msg);
    }

    #[test]
    fn client_leave_roundtrip() {
        let msg = ClientMsg::Leave;
        let json = encode_client(&msg).unwrap();
        assert_eq!(decode_client(&json).unwrap(), msg);
    }

    #[test]
    fn server_message_roundtrip() {
        let msg = ServerMsg::Message {
            from: "alice".into(),
            text: "hi".into(),
        };
        let json = encode_server(&msg).unwrap();
        assert_eq!(decode_server(&json).unwrap(), msg);
    }

    #[test]
    fn server_welcome_roundtrip() {
        let msg = ServerMsg::Welcome {
            username: "alice".into(),
        };
        let json = encode_server(&msg).unwrap();
        assert_eq!(decode_server(&json).unwrap(), msg);
    }

    #[test]
    fn server_username_taken_roundtrip() {
        let msg = ServerMsg::UsernameTaken {
            username: "bob".into(),
        };
        let json = encode_server(&msg).unwrap();
        assert_eq!(decode_server(&json).unwrap(), msg);
    }

    #[test]
    fn server_user_joined_roundtrip() {
        let msg = ServerMsg::UserJoined {
            username: "charlie".into(),
        };
        let json = encode_server(&msg).unwrap();
        assert_eq!(decode_server(&json).unwrap(), msg);
    }

    #[test]
    fn server_user_left_roundtrip() {
        let msg = ServerMsg::UserLeft {
            username: "dan".into(),
        };
        let json = encode_server(&msg).unwrap();
        assert_eq!(decode_server(&json).unwrap(), msg);
    }

    #[test]
    fn client_send_json_shape() {
        let json = encode_client(&ClientMsg::Send {
            text: "test".into(),
        })
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "send");
        assert_eq!(v["text"], "test");
    }

    #[test]
    fn client_leave_json_shape() {
        let json = encode_client(&ClientMsg::Leave).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "leave");
    }

    #[test]
    fn rejects_unknown_client_msg() {
        assert!(decode_client(r#"{"type":"unknown"}"#).is_err());
    }
}
