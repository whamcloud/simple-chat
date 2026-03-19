use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "chat-client")]
#[command(about = "A simple chat client CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Connect to a server
    Connect {
        /// The IP address and port (e.g., 127.0.0.1:8080)
        address: String,
    },
    /// Send a message to the server
    Send {
        /// The message to send
        #[arg(num_args = 1..)]
        message: Vec<String>,
    },
    /// Leave the current server room
    Leave,
    /// Exit the application
    Exit,
}
