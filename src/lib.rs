use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatError {
    InvalidUsername(String),
    SerializationError(String),
    NetworkError(String),
}

impl fmt::Display for ChatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChatError::InvalidUsername(msg) => write!(f, "Invalid username: {}", msg),
            ChatError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            ChatError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for ChatError {}

impl From<std::io::Error> for ChatError {
    fn from(error: std::io::Error) -> Self {
        ChatError::NetworkError(error.to_string())
    }
}

pub type ChatResult<T> = Result<T, ChatError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    Join { username: String },
    Leave { username: String },
    ChatMessage { username: String, content: String },
    UserJoined { username: String },
    UserLeft { username: String },
    Error { message: String },
}

impl Message {
    pub fn validate(&self) -> ChatResult<()> {
        match self {
            Message::Join { username }
            | Message::Leave { username }
            | Message::UserJoined { username }
            | Message::UserLeft { username } => validate_username(username),
            Message::ChatMessage { username, content } => {
                validate_username(username)?;
                if content.trim().is_empty() {
                    return Err(ChatError::InvalidUsername(
                        "Message content cannot be empty".to_string(),
                    ));
                }
                if content.len() > 1000 {
                    return Err(ChatError::InvalidUsername(
                        "Message content too long (max 1000 characters)".to_string(),
                    ));
                }
                Ok(())
            }
            Message::Error { .. } => Ok(()),
        }
    }
}

pub fn validate_username(username: &str) -> ChatResult<()> {
    if username.trim().is_empty() {
        return Err(ChatError::InvalidUsername(
            "Username cannot be empty".to_string(),
        ));
    }

    if username.len() < 2 {
        return Err(ChatError::InvalidUsername(
            "Username must be at least 2 characters long".to_string(),
        ));
    }

    if username.len() > 20 {
        return Err(ChatError::InvalidUsername(
            "Username must be at most 20 characters long".to_string(),
        ));
    }

    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ChatError::InvalidUsername(
            "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
        ));
    }

    Ok(())
}

pub mod codec {
    use super::{ChatError, Message};
    use bytes::{BufMut, BytesMut};
    use tokio_util::codec::{Decoder, Encoder};

    pub struct MessageCodec;

    impl Decoder for MessageCodec {
        type Item = Message;
        type Error = ChatError;

        fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
            if src.is_empty() {
                return Ok(None);
            }

            if let Some(newline_pos) = src.iter().position(|&b| b == b'\n') {
                let line = src.split_to(newline_pos + 1);
                let line = &line[..line.len() - 1];
                let message: Message = serde_json::from_slice(line)
                    .map_err(|e| ChatError::SerializationError(e.to_string()))?;
                message.validate()?;
                Ok(Some(message))
            } else {
                Ok(None)
            }
        }
    }

    impl Encoder<Message> for MessageCodec {
        type Error = ChatError;

        fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
            item.validate()?;
            let json = serde_json::to_vec(&item)
                .map_err(|e| ChatError::SerializationError(e.to_string()))?;
            dst.reserve(json.len() + 1);
            dst.put_slice(&json);
            dst.put_u8(b'\n');
            Ok(())
        }
    }
}

pub mod stream_utils {
    use super::codec::MessageCodec;
    use super::{ChatError, ChatResult, Message};
    use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
    use tokio_util::codec::{FramedRead, FramedWrite};

    pub async fn send_message<W>(writer: &mut W, message: &Message) -> ChatResult<()>
    where
        W: AsyncWrite + Unpin,
    {
        message.validate()?;
        let json = serde_json::to_string(message)
            .map_err(|e| ChatError::SerializationError(e.to_string()))?;
        let data = format!("{}\n", json);
        writer
            .write_all(data.as_bytes())
            .await
            .map_err(|e| ChatError::NetworkError(e.to_string()))?;
        writer
            .flush()
            .await
            .map_err(|e| ChatError::NetworkError(e.to_string()))?;
        Ok(())
    }

    pub async fn receive_message<R>(reader: &mut BufReader<R>) -> ChatResult<Message>
    where
        R: AsyncRead + Unpin,
    {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| ChatError::NetworkError(e.to_string()))?;

        if line.is_empty() {
            return Err(ChatError::NetworkError("Connection closed".to_string()));
        }

        let line = line.trim();
        let message: Message =
            serde_json::from_str(line).map_err(|e| ChatError::SerializationError(e.to_string()))?;
        message.validate()?;
        Ok(message)
    }

    pub fn create_framed_read<R>(reader: R) -> FramedRead<R, MessageCodec>
    where
        R: AsyncRead,
    {
        FramedRead::new(reader, MessageCodec)
    }

    pub fn create_framed_write<W>(writer: W) -> FramedWrite<W, MessageCodec>
    where
        W: AsyncWrite,
    {
        FramedWrite::new(writer, MessageCodec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let message = Message::ChatMessage {
            username: "alice".to_string(),
            content: "Hello, world!".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_all_message_variants_serialization() {
        let messages = vec![
            Message::Join {
                username: "alice".to_string(),
            },
            Message::Leave {
                username: "bob".to_string(),
            },
            Message::ChatMessage {
                username: "charlie".to_string(),
                content: "Hello!".to_string(),
            },
            Message::UserJoined {
                username: "dave".to_string(),
            },
            Message::UserLeft {
                username: "eve".to_string(),
            },
            Message::Error {
                message: "Something went wrong".to_string(),
            },
        ];

        for message in messages {
            let json = serde_json::to_string(&message).unwrap();
            let deserialized: Message = serde_json::from_str(&json).unwrap();
            assert_eq!(message, deserialized);
        }
    }

    #[test]
    fn test_username_validation_valid() {
        assert!(validate_username("alice").is_ok());
        assert!(validate_username("bob123").is_ok());
        assert!(validate_username("charlie_doe").is_ok());
        assert!(validate_username("dave-smith").is_ok());
        assert!(validate_username("a1").is_ok());
        assert!(validate_username("abcdefghij1234567890").is_ok()); // 20 chars
    }

    #[test]
    fn test_username_validation_invalid() {
        assert!(validate_username("").is_err());
        assert!(validate_username(" ").is_err());
        assert!(validate_username("a").is_err());
        assert!(validate_username("abcdefghij12345678901").is_err()); // 21 chars
        assert!(validate_username("alice@domain").is_err());
        assert!(validate_username("bob!").is_err());
        assert!(validate_username("charlie#").is_err());
        assert!(validate_username("dave.smith").is_err());
        assert!(validate_username("eve space").is_err());
    }

    #[test]
    fn test_message_validation_valid() {
        let valid_messages = vec![
            Message::Join {
                username: "alice".to_string(),
            },
            Message::Leave {
                username: "bob".to_string(),
            },
            Message::ChatMessage {
                username: "charlie".to_string(),
                content: "Hello, world!".to_string(),
            },
            Message::UserJoined {
                username: "dave".to_string(),
            },
            Message::UserLeft {
                username: "eve".to_string(),
            },
            Message::Error {
                message: "Any error message".to_string(),
            },
        ];

        for message in valid_messages {
            assert!(
                message.validate().is_ok(),
                "Message should be valid: {:?}",
                message
            );
        }
    }

    #[test]
    fn test_message_validation_invalid_username() {
        let invalid_messages = vec![
            Message::Join {
                username: "a".to_string(),
            },
            Message::Leave {
                username: "".to_string(),
            },
            Message::ChatMessage {
                username: "alice@domain".to_string(),
                content: "Hello!".to_string(),
            },
            Message::UserJoined {
                username: "abcdefghij12345678901".to_string(),
            },
            Message::UserLeft {
                username: "bob!".to_string(),
            },
        ];

        for message in invalid_messages {
            assert!(
                message.validate().is_err(),
                "Message should be invalid: {:?}",
                message
            );
        }
    }

    #[test]
    fn test_message_validation_invalid_content() {
        let message = Message::ChatMessage {
            username: "alice".to_string(),
            content: "".to_string(),
        };
        assert!(message.validate().is_err());

        let message = Message::ChatMessage {
            username: "alice".to_string(),
            content: "   ".to_string(),
        };
        assert!(message.validate().is_err());

        let long_content = "a".repeat(1001);
        let message = Message::ChatMessage {
            username: "alice".to_string(),
            content: long_content,
        };
        assert!(message.validate().is_err());
    }

    #[tokio::test]
    async fn test_codec_encode_decode() {
        use crate::codec::MessageCodec;
        use bytes::BytesMut;
        use tokio_util::codec::{Decoder, Encoder};

        let mut codec = MessageCodec;
        let message = Message::ChatMessage {
            username: "alice".to_string(),
            content: "Hello, world!".to_string(),
        };

        let mut buffer = BytesMut::new();
        codec.encode(message.clone(), &mut buffer).unwrap();

        let decoded = codec.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(message, decoded);
    }

    #[tokio::test]
    async fn test_codec_invalid_message() {
        use crate::codec::MessageCodec;
        use bytes::BytesMut;
        use tokio_util::codec::Encoder;

        let mut codec = MessageCodec;
        let invalid_message = Message::Join {
            username: "a".to_string(),
        }; // Too short

        let mut buffer = BytesMut::new();
        let result = codec.encode(invalid_message, &mut buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        let error = ChatError::InvalidUsername("Test error".to_string());
        assert_eq!(format!("{}", error), "Invalid username: Test error");

        let error = ChatError::SerializationError("JSON error".to_string());
        assert_eq!(format!("{}", error), "Serialization error: JSON error");

        let error = ChatError::NetworkError("Connection failed".to_string());
        assert_eq!(format!("{}", error), "Network error: Connection failed");
    }

    #[tokio::test]
    async fn test_stream_utils_send_receive() {
        use crate::stream_utils::{receive_message, send_message};
        use tokio::io::BufReader;

        let message = Message::ChatMessage {
            username: "alice".to_string(),
            content: "Hello!".to_string(),
        };

        // Create an in-memory buffer to simulate a stream
        let mut buffer = Vec::new();

        // Send message
        send_message(&mut buffer, &message).await.unwrap();

        // Create a reader from the buffer
        let cursor = std::io::Cursor::new(buffer);
        let mut reader = BufReader::new(cursor);

        // Receive message
        let received = receive_message(&mut reader).await.unwrap();
        assert_eq!(message, received);
    }
}
