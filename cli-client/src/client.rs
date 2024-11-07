//! This module contains types and functions to connect with the server.

use std::io;

use common::{extract_parts, messages};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
};

/// A struct to encapsulate functionalities related to connect and messaging with the server.
pub struct Client {
    /// Server host
    pub host: String,
    /// Server port
    pub port: String,
    /// Username representing human that uses this client
    pub username: String,
}

/// Enum to represent the commands that can be entered by the user
#[derive(PartialEq)]
enum ConsoleCommand {
    /// Command to leave the chat room
    Leave,
    /// Command to send a message to the chat room
    Send,
    InvalidCommand,
}

impl From<String> for ConsoleCommand {
    fn from(str: String) -> Self {
        match str.as_str() {
            "leave" => ConsoleCommand::Leave,
            "send" => ConsoleCommand::Send,
            _ => ConsoleCommand::InvalidCommand,
        }
    }
}

impl From<&str> for ConsoleCommand {
    fn from(command_str: &str) -> Self {
        match command_str.to_lowercase().as_str() {
            "leave" => ConsoleCommand::Leave,
            "send" => ConsoleCommand::Send,
            _ => ConsoleCommand::InvalidCommand,
        }
    }
}

impl Client {
    pub fn new(host: String, port: String, username: String) -> Self {
        Client {
            host,
            port,
            username,
        }
    }
    /// Starts the connection with the server and handles the communication between the client and the server.
    pub async fn start(&self) -> io::Result<()> {
        // Connect to the server
        let mut stream = TcpStream::connect(format!("{}:{}", self.host, self.port)).await?;

        // Disable Nagle's algorithm to send data immediately
        stream.set_nodelay(true).unwrap();
        let (reader, writer) = stream.split();

        // Create a buffered writer and reader for network communication
        let mut writer = BufWriter::new(writer);
        let mut reader = BufReader::new(reader);
        let mut line = String::new(); // Buffer to store received data

        // Join the default room using supplied username
        writer
            .write_all(format!("<{}> {}\n", messages::JOIN_USER, self.username).as_bytes())
            .await
            .expect("ERROR: Unable to write to server");
        writer.flush().await.expect("ERROR: Unable to flush writer");

        reader
            .read_line(&mut line)
            .await
            .expect("ERROR: Unable to read from server");

        let (command, _, _) = extract_parts(&line);
        if command == messages::DUPLICATE_USER {
            eprintln!(
                "ERROR: Username already in use. Please try again with a different username.\n"
            );
            return Ok(());
        }

        let mut input = String::new();

        // Create a buffered writer and reader for stdin/stdout communication
        let mut console_reader = BufReader::new(tokio::io::stdin());

        line.clear();
        loop {
            tokio::select! {
                _result = console_reader.read_line(&mut input) => {

                    // Handle sending here
                    input = input.trim().to_string();
                    if input.is_empty() || input == "\n" {
                        continue;
                    }

                    let user_input = input.split(" ").collect::<Vec<&str>>();

                    // Extract command from user input
                    let original_command = user_input[0].to_lowercase();
                    let command = ConsoleCommand::from(original_command.clone());

                    if command == ConsoleCommand::Leave {
                        writer.write_all(format!("<{}> {}\n", messages::LEAVE_USER, self.username).as_bytes()).await.expect("Unable to write to server");
                        writer.flush().await.expect("Unable to write to server");
                        return Ok(());
                    } else if command == ConsoleCommand::Send{
                        let usr_msg = &input[original_command.len() + 1..];
                        let usr_msg = format!("<{}> {} {}\n", messages::USER_MSG, self.username, usr_msg);
                        writer.write_all(usr_msg.as_bytes()).await.expect("Unable to write to server");
                        writer.flush().await.expect("Unable to write to server");
                    }

                    input.clear();
             }

                result = reader.read_line(&mut line) => {
                    if result.expect("ERROR: Unable to read from server") == 0 {
                        eprintln!("Server closed the connection.");
                        return Ok(());
                    }
                    let (command, username, data) = extract_parts(&line);
                    if command == messages::USER_MSG {
                        println!("{}> {}", username, data);
                    } else if command == messages::INVALID_CMD {
                        eprintln!("ERROR: Invalid command received from server");
                    }
                    line.clear();
                }
            }
        }
    }
}
