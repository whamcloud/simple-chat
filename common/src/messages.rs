//! This module conntains messages used for communication between Client and the Server.

pub const JOIN_USER: u16 = 101;
pub const USER_JOINED: u16 = 102;
pub const LEAVE_USER: u16 = 103;
pub const USER_LEFT: u16 = 104;
pub const DUPLICATE_USER: u16 = 105;
pub const INVALID_CMD: u16 = 106;
pub const USER_MSG: u16 = 107;
pub const WELCOME_MSG: u16 = 108;
