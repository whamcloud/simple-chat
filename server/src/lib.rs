pub mod protocol;
pub mod room;

// Re-export for convenience
pub use protocol::{ClientMessage, ServerMessage};
pub use room::Room;
