//! Common definitions for the chat application.

pub const MAX_USERNAME_LEN: usize = 32;

pub const MAX_MESSAGE_LEN: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BroadcastMessage {
    pub sender_id: String,
    pub content: String,
    pub is_system: bool,
}

impl BroadcastMessage {
    pub fn user_msg(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            sender_id: sender.into(),
            content: content.into(),
            is_system: false,
        }
    }

    pub fn system_msg(content: impl Into<String>) -> Self {
        Self {
            sender_id: "SYSTEM".to_string(),
            content: content.into(),
            is_system: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = BroadcastMessage::user_msg("Alice", "Hello");
        assert_eq!(msg.sender_id, "Alice");
        assert_eq!(msg.content, "Hello");
        assert!(!msg.is_system);

        let sys = BroadcastMessage::system_msg("Alert");
        assert_eq!(sys.sender_id, "SYSTEM");
        assert!(sys.is_system);
    }
}
