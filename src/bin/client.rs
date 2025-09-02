use anyhow::{anyhow, Context, Result};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use log::{error, info};
use simple_chat::Message;
use std::io::{self, Write};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::codec::{FramedRead, FramedWrite};

#[derive(Parser)]
#[command(author, version, about = "Simple Chat Client", long_about = None)]
struct ClientArgs {
    #[arg(short = 'H', long, env = "CHAT_HOST", default_value = "localhost")]
    host: String,

    #[arg(short, long, env = "CHAT_PORT", default_value_t = 8080)]
    port: u16,

    #[arg(short, long, env = "CHAT_USERNAME")]
    username: String,
}

#[derive(Debug, Clone)]
enum ClientCommand {
    Send(String),
    Leave,
    InvalidCommand(String),
}

impl ClientCommand {
    fn parse(input: &str) -> Self {
        let input = input.trim();

        if input.is_empty() {
            return ClientCommand::InvalidCommand("Empty command".to_string());
        }

        if input == "leave" {
            return ClientCommand::Leave;
        }

        if let Some(message) = input.strip_prefix("send ") {
            if message.trim().is_empty() {
                return ClientCommand::InvalidCommand("Empty message content".to_string());
            }
            return ClientCommand::Send(message.to_string());
        }

        if input == "send" {
            return ClientCommand::InvalidCommand("Empty message content".to_string());
        }

        ClientCommand::InvalidCommand(format!("Unknown command: {}", input))
    }
}

struct ChatClient {
    username: String,
    server_addr: String,
    framed_read:
        Option<FramedRead<tokio::net::tcp::OwnedReadHalf, simple_chat::codec::MessageCodec>>,
    framed_write:
        Option<FramedWrite<tokio::net::tcp::OwnedWriteHalf, simple_chat::codec::MessageCodec>>,
}

impl ChatClient {
    fn new(username: String, host: String, port: u16) -> Self {
        Self {
            username,
            server_addr: format!("{}:{}", host, port),
            framed_read: None,
            framed_write: None,
        }
    }

    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to server at {}", self.server_addr);

        let stream = timeout(
            Duration::from_secs(10),
            TcpStream::connect(&self.server_addr),
        )
        .await
        .context("Connection timeout")?
        .context("Failed to connect to server")?;

        let (read_half, write_half) = stream.into_split();

        self.framed_read = Some(FramedRead::new(read_half, simple_chat::codec::MessageCodec));
        self.framed_write = Some(FramedWrite::new(
            write_half,
            simple_chat::codec::MessageCodec,
        ));

        // Send join message
        let join_message = Message::Join {
            username: self.username.clone(),
        };

        if let Some(ref mut writer) = self.framed_write {
            writer
                .send(join_message)
                .await
                .context("Failed to send join message")?;
            info!("Sent join request for user '{}'", self.username);
        }

        Ok(())
    }

    async fn run(&mut self) -> Result<()> {
        self.connect().await?;

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<ClientCommand>(100);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
        let mut main_shutdown_rx = shutdown_rx;

        // Spawn task for reading server messages
        let mut reader = self.framed_read.take().unwrap();
        let shutdown_tx_clone = shutdown_tx.clone();
        let server_task = tokio::spawn(async move {
            loop {
                match reader.next().await {
                    Some(Ok(message)) => {
                        Self::display_message(&message);

                        // Check for server disconnection messages
                        if let Message::Error { message: error_msg } = &message {
                            if error_msg.contains("Server error")
                                || error_msg.contains("Connection")
                            {
                                error!("Server error received: {}", error_msg);
                                let _ = shutdown_tx_clone.send(()).await;
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!("Error reading from server: {}", e);
                        let _ = shutdown_tx_clone.send(()).await;
                        break;
                    }
                    None => {
                        info!("Server disconnected");
                        let _ = shutdown_tx_clone.send(()).await;
                        break;
                    }
                }
            }
        });

        // Spawn task for reading user input
        let cmd_tx_clone = cmd_tx.clone();
        let shutdown_tx_clone = shutdown_tx.clone();
        let input_task =
            tokio::spawn(
                async move { Self::handle_user_input(cmd_tx_clone, shutdown_tx_clone).await },
            );

        // Main command processing loop
        let mut writer = self.framed_write.take().unwrap();

        println!("🚀 Connected to chat server!");
        println!("💡 Commands:");
        println!("   send <message> - Send a message");
        println!("   leave          - Leave the chat");
        println!();

        loop {
            tokio::select! {
                command = cmd_rx.recv() => {
                    match command {
                        Some(ClientCommand::Send(content)) => {
                            let message = Message::ChatMessage {
                                username: self.username.clone(),
                                content,
                            };

                            if let Err(e) = writer.send(message).await {
                                error!("Failed to send message: {}", e);
                                break;
                            }
                        },
                        Some(ClientCommand::Leave) => {
                            info!("User requested to leave");
                            let leave_message = Message::Leave {
                                username: self.username.clone(),
                            };
                            let _ = writer.send(leave_message).await;
                            break;
                        },
                        Some(ClientCommand::InvalidCommand(error)) => {
                            println!("❌ Error: {}", error);
                            println!("💡 Use 'send <message>' to send a message or 'leave' to exit");
                        },
                        None => {
                            info!("Command channel closed");
                            break;
                        }
                    }
                },
                _ = main_shutdown_rx.recv() => {
                    info!("Main loop received shutdown signal");
                    break;
                }
            }
        }

        // Cleanup
        server_task.abort();
        input_task.abort();

        Ok(())
    }

    fn display_message(message: &Message) {
        match message {
            Message::ChatMessage { username, content } => {
                println!("[{}]: {}", username, content);
            }
            Message::UserJoined { username } => {
                println!("🟢 {} joined the chat", username);
            }
            Message::UserLeft { username } => {
                println!("🔴 {} left the chat", username);
            }
            Message::Error { message } => {
                println!("❌ Server error: {}", message);
            }
            _ => {
                // Handle other message types if needed
                info!("Received message: {:?}", message);
            }
        }

        // Flush stdout to ensure immediate display
        io::stdout().flush().unwrap_or(());
    }

    async fn handle_user_input(cmd_tx: mpsc::Sender<ClientCommand>, shutdown_tx: mpsc::Sender<()>) {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            print!("> ");
            io::stdout().flush().unwrap_or(());

            line.clear();

            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF reached (Ctrl+D)
                    info!("EOF received, shutting down");
                    let _ = shutdown_tx.send(()).await;
                    break;
                }
                Ok(_) => {
                    let command = ClientCommand::parse(&line);

                    // Check for leave command to trigger shutdown
                    if matches!(command, ClientCommand::Leave) {
                        let _ = cmd_tx.send(command).await;
                        let _ = shutdown_tx.send(()).await;
                        break;
                    }

                    if cmd_tx.send(command).await.is_err() {
                        error!("Failed to send command");
                        break;
                    }
                }
                Err(e) => {
                    error!("Error reading input: {}", e);
                    let _ = shutdown_tx.send(()).await;
                    break;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = ClientArgs::parse();

    // Validate username
    if let Err(e) = simple_chat::validate_username(&args.username) {
        return Err(anyhow!("Invalid username: {}", e));
    }

    info!("Starting chat client for user '{}'", args.username);
    info!("Connecting to server at {}:{}", args.host, args.port);

    let mut client = ChatClient::new(args.username.clone(), args.host, args.port);

    match client.run().await {
        Ok(_) => {
            println!("👋 Goodbye, {}!", args.username);
            Ok(())
        }
        Err(e) => {
            error!("Client error: {}", e);
            println!("❌ Connection error: {}", e);
            println!("💡 Please check that the server is running and try again.");
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing_send() {
        let cmd = ClientCommand::parse("send Hello world!");
        match cmd {
            ClientCommand::Send(msg) => assert_eq!(msg, "Hello world!"),
            _ => panic!("Expected Send command"),
        }
    }

    #[test]
    fn test_command_parsing_send_with_extra_spaces() {
        let cmd = ClientCommand::parse("  send   Hello world!  ");
        match cmd {
            ClientCommand::Send(msg) => assert_eq!(msg, "  Hello world!"),
            _ => panic!("Expected Send command"),
        }
    }

    #[test]
    fn test_command_parsing_leave() {
        let cmd = ClientCommand::parse("leave");
        matches!(cmd, ClientCommand::Leave);

        let cmd = ClientCommand::parse("  leave  ");
        matches!(cmd, ClientCommand::Leave);
    }

    #[test]
    fn test_command_parsing_invalid_empty() {
        let cmd = ClientCommand::parse("");
        match cmd {
            ClientCommand::InvalidCommand(msg) => assert_eq!(msg, "Empty command"),
            _ => panic!("Expected InvalidCommand"),
        }

        let cmd = ClientCommand::parse("   ");
        match cmd {
            ClientCommand::InvalidCommand(msg) => assert_eq!(msg, "Empty command"),
            _ => panic!("Expected InvalidCommand"),
        }
    }

    #[test]
    fn test_command_parsing_invalid_unknown() {
        let cmd = ClientCommand::parse("unknown command");
        match cmd {
            ClientCommand::InvalidCommand(msg) => {
                assert_eq!(msg, "Unknown command: unknown command")
            }
            _ => panic!("Expected InvalidCommand"),
        }
    }

    #[test]
    fn test_command_parsing_send_empty_message() {
        let cmd = ClientCommand::parse("send ");
        match cmd {
            ClientCommand::InvalidCommand(msg) => assert_eq!(msg, "Empty message content"),
            _ => panic!("Expected InvalidCommand"),
        }

        let cmd = ClientCommand::parse("send   ");
        match cmd {
            ClientCommand::InvalidCommand(msg) => assert_eq!(msg, "Empty message content"),
            _ => panic!("Expected InvalidCommand"),
        }
    }

    #[test]
    fn test_message_display_formatting() {
        // Test that display_message doesn't panic
        // Since it prints to stdout, we can't easily test the output
        // but we can ensure it doesn't crash

        let messages = vec![
            Message::ChatMessage {
                username: "alice".to_string(),
                content: "Hello world!".to_string(),
            },
            Message::UserJoined {
                username: "bob".to_string(),
            },
            Message::UserLeft {
                username: "charlie".to_string(),
            },
            Message::Error {
                message: "Test error".to_string(),
            },
        ];

        for message in messages {
            ChatClient::display_message(&message);
        }
    }

    #[test]
    fn test_client_creation() {
        let client = ChatClient::new("testuser".to_string(), "localhost".to_string(), 8080);

        assert_eq!(client.username, "testuser");
        assert_eq!(client.server_addr, "localhost:8080");
        assert!(client.framed_read.is_none());
        assert!(client.framed_write.is_none());
    }
}
