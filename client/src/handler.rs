use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::io::{self, Write};
use tokio::task::JoinHandle;

pub struct ChatHandler {
    pub writer: Option<OwnedWriteHalf>,
    pub task: Option<JoinHandle<()>>,
}

impl ChatHandler {
    pub fn new() -> Self {
        Self {
            writer: None,
            task: None,
        }
    }

    pub async fn connect(&mut self, address: &str, client_user: &str) -> io::Result<()> {
        if self.writer.is_some() {
            println!("Already connected. Use 'leave' first.");
            return Ok(());
        }

        let mut stream = TcpStream::connect(address).await?;
        println!("Connected to {}", address);

        // Send client ID immediately
        stream.write_all(format!("{}\n", client_user).as_bytes()).await?;

        let (reader, writer) = stream.into_split();
        self.writer = Some(writer);

        // Spawn task to handle incoming messages
        let handle = tokio::spawn(async move {
            let mut server_reader = BufReader::new(reader);
            let mut server_line = String::new();
            loop {
                server_line.clear();
                match server_reader.read_line(&mut server_line).await {
                    Ok(0) => {
                        println!("\nDisconnected from server.");
                        break;
                    }
                    Ok(_) => {
                        print!("\r{}", server_line);
                        print!("> ");
                        io::stdout().flush().unwrap();
                    }
                    Err(_) => break,
                }
            }
        });
        self.task = Some(handle);
        Ok(())
    }

    pub async fn send(&mut self, message: &str) -> io::Result<()> {
        if let Some(ref mut w) = self.writer {
            if let Err(e) = w.write_all(format!("{}\n", message).as_bytes()).await {
                println!("Failed to send message: {}", e);
                self.leave();
                return Err(e);
            }
            Ok(())
        } else {
            println!("Not connected. Use 'connect <address>' first.");
            Ok(())
        }
    }

    pub fn leave(&mut self) {
        if self.writer.is_some() {
            println!("Left server room");
            self.writer = None;
            if let Some(h) = self.task.take() {
                h.abort();
            }
        } else {
            println!("Not connected.");
        }
    }
}
