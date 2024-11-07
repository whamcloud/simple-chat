//! This module contains types and functions to handle messages from the client.

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use common::{extract_parts, messages};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
};
use tracing::{event, span, Level, Span};

/// The server instance
pub struct SimpleChatServer {
    /// The address the server is listening on
    pub address: Arc<String>,
    /// The users connected to the server
    pub users: Arc<Mutex<HashMap<String, Sender<String>>>>,
}

impl SimpleChatServer {
    /// Create a new server instance
    pub fn new(address: String) -> SimpleChatServer {
        let address = Arc::new(address);

        SimpleChatServer {
            address,
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start the server
    pub async fn start(&self) -> io::Result<()> {
        let address = self.address.clone();
        let listener = TcpListener::bind(address.to_string()).await?;

        tracing::info!("Starting server on {}", self.address);

        while let Ok((stream, client_address)) = listener.accept().await {
            let users = self.users.clone();

            tokio::spawn({
                async move {
                    let _ = Self::handle_connection(stream, client_address, users).await;
                }
            });
        }

        Ok(())
    }

    // Handles joining of a new user. Returns true/false based on user joining success.
    async fn handle_join_command<T>(
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
        writer: &mut T,
        line: &mut String,
        my_username: &mut String,
        data: &str,
        tx: Sender<String>,
        parent_span: &Span,
    ) -> bool
    where
        T: AsyncWrite + std::marker::Unpin,
    {
        let span = span!(parent: parent_span, Level::INFO, "handle_join_command");
        let _guard = span.enter();

        if users.lock().await.contains_key(data) {
            let _ = writer
                .write_all(format!("<{}>\n", messages::DUPLICATE_USER).as_bytes())
                .await;
            let _ = writer.flush().await;

            event!(Level::WARN, "Duplicate username received: {}", data);
            line.clear(); // Clear the buffer for safety.
            return false;
        }
        users.lock().await.insert(data.to_string(), tx.clone());
        *my_username = data.to_string();
        event!(Level::INFO, "User {} has joined the chat", data);

        // Send ACK for user joining.
        let _ = writer
            .write_all(format!("<{}> {}\n", messages::USER_JOINED, &data).as_bytes())
            .await;
        let _ = writer.flush().await;

        line.clear(); // Clear the buffer for safety.
        drop(_guard);
        true
    }

    // Handle leaving of the existing user
    async fn handle_leave_command(
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
        rx: &mut Receiver<String>,
        data: &str,
        parent_span: &Span,
    ) {
        let span = span!(parent: parent_span, Level::INFO, "handle_leave_command");
        let _guard = span.enter();

        users.lock().await.remove(data);

        rx.close();
        event!(Level::INFO, "User {:?} has left the chat", &data);
    }

    // Handle user messages
    async fn handle_user_messages<T>(
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
        my_username: &str,
        writer: &mut T,
        line: &mut String,
        data: &str,
        sender_username: &str,
        parent_span: &Span,
    ) where
        T: AsyncWrite + std::marker::Unpin,
    {
        let span = span!(parent: parent_span, Level::INFO, "handle_user_messages");
        let _guard = span.enter();

        // User should not be able to send message if not joined.
        if my_username.is_empty() {
            event!(
                Level::WARN,
                "Connection is trying to send message without joining the chat"
            );

            let _ = writer
                .write_all("Please join the chat first.\n".as_bytes())
                .await;
            let _ = writer.flush().await;
            line.clear(); // Clear the buffer for safety.
            return;
        }

        event!(
            Level::DEBUG,
            "Sending message {:?} to all users from {}",
            &line,
            &sender_username
        );

        for (username, sender) in users.lock().await.iter() {
            if username != my_username {
                let usr_msg = format!("<{}> {} {}\n", messages::USER_MSG, sender_username, data);
                let _ = sender.send(usr_msg).await;
            }
        }
    }

    // Primary function to handle the connection. Called for each new user connection. It manages the user joining, leaving and messages.
    async fn handle_connection(
        mut stream: TcpStream,
        client_address: SocketAddr,
        users: Arc<Mutex<HashMap<String, Sender<String>>>>,
    ) -> io::Result<()> {
        let handle_connection_span =
            span!(Level::INFO, "handle_connection", client_address = %client_address);
        let _guard = handle_connection_span.enter();

        event!(Level::INFO, "Connected");
        stream.set_nodelay(true)?;

        let (reader, writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);

        // Send welcome message to the client
        // let _ = writer
        //     .write_all(format!("{} <server>\n", messages::WELCOME_MSG).as_bytes())
        //     .await;
        // let _ = writer.flush().await;

        let (tx, mut rx) = channel::<String>(1000);

        let mut line = String::new();
        let mut my_username = String::new();

        /*
           IMP:
           reader.read_line() method expects \n at the end of the message to mark it as line. Without it, it will wait indefinitely.
        */
        event!(Level::TRACE, "Starting loop");
        loop {
            tokio::select! {
                result = reader.read_line(&mut line) => {
                    match result {
                        Ok(n) => {
                            event!(Level::TRACE, "Line received {:?}", &line);
                            if n == 0 {
                                event!(Level::TRACE, "Connection closed");
                                break Ok(());
                            }

                            if line.trim() == "\n" {
                                continue;
                            }

                            let (command, username, data) = extract_parts(&line);
                            event!(Level::TRACE, "Command: {:?}, Username: {:?}, Data: {:?}", &command, &username, &data);

                            // Handle joining of the user
                            if command == messages::JOIN_USER {
                                Self::handle_join_command(users.clone(), &mut writer, &mut line, &mut my_username, &data, tx.clone(), &handle_connection_span).await;
                            }
                            // Handle leaving of the user
                            else if command == messages::LEAVE_USER {
                                Self::handle_leave_command(users.clone(), &mut rx, &data, &handle_connection_span).await;
                            }
                            // Handle user messages
                            else if command ==  messages::USER_MSG {
                                Self::handle_user_messages(users.clone(), &my_username, &mut writer, &mut line, &data, &username, &handle_connection_span).await;
                            } else {
                                event!(Level::WARN, "Invalid command received: {:?}", &line);
                                let _ = writer.write_all(format!("{}", messages::INVALID_CMD).as_bytes()).await;
                                let _ = writer.flush().await;
                            }

                            line.clear(); // Clear the buffer for safety.

                        }
                        Err(e) => {
                            event!(Level::ERROR, "ERROR: Failed to read from socket; error={:?}", e);
                            break Ok(());
                        }
                    }
                }
                result = rx.recv() => {
                    match result {
                        Some(msg) => {
                            // Received message from other user(s). Send it to this user if this user is not the sender.
                            let _ = writer.write_all(msg.as_bytes()).await;
                            let _ = writer.flush().await;
                        }
                        None => {
                            event!(Level::ERROR, "Channel closed");
                            break Ok(());
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_handle_leave_command() {
        /*********** Preparation **********/
        let data: &str = "testuser";
        let (tx, mut rx) = channel::<String>(1);
        let mut users_map = HashMap::new();
        users_map.insert(data.to_string(), tx);
        let span = span!(Level::INFO, "test_handle_leave_command");
        let users = Arc::new(Mutex::new(users_map));

        /*********** Call **********/
        SimpleChatServer::handle_leave_command(users.clone(), &mut rx, data, &span).await;

        /*********** Assertion **********/
        assert!(rx.is_closed());
        assert_eq!(users.lock().await.len(), 0);
    }

    // Tests the scenario where a new user joins the chat. The username of a new user doesn't exists.
    #[tokio::test]
    async fn test_handle_join_command_user_not_exists() {
        /*********** Preparation **********/
        let existing_username: &str = "testuser";
        let (tx, _rx) = channel::<String>(1);

        let (client, mut _server) = io::duplex(64);
        let mut buf_writer = BufWriter::new(client);
        let mut line = String::from("test line from user");
        let mut my_username = String::new();
        let users = Arc::new(Mutex::new(HashMap::new()));

        // This is really not important as we are not using it in the test.
        let span = span!(Level::INFO, "test_handle_join_command_user_not_exists");

        /*********** Call **********/
        SimpleChatServer::handle_join_command(
            users.clone(),
            &mut buf_writer,
            &mut line,
            &mut my_username,
            existing_username,
            tx,
            &span,
        )
        .await;

        /*********** Assertion **********/
        assert!(users.lock().await.contains_key(existing_username));
        assert_eq!(my_username, existing_username);
        assert!(line.is_empty());
    }

    // Tests the scenario where a new user joins the chat. The username of a new user already exists.
    #[tokio::test]
    async fn test_handle_join_command_user_exists() {
        /*********** Preparation **********/
        let existing_username: &str = "testuser";
        let (client, server) = io::duplex(64);

        let mut buf_writer = BufWriter::new(client);
        let mut buf_reader = BufReader::new(server);

        let mut line = String::from("test line from user");
        let mut my_username = String::new();

        // Add user already
        let mut users_map: HashMap<String, Sender<String>> = HashMap::new();
        let (tx, _rx) = channel::<String>(1);
        users_map.insert(existing_username.to_string(), tx.clone());

        let users = Arc::new(Mutex::new(users_map));

        // This is really not important as we are not using it in the test.
        let span = span!(Level::INFO, "test_handle_join_command_user_exists");

        /*********** Call **********/
        SimpleChatServer::handle_join_command(
            users.clone(),
            &mut buf_writer,
            &mut line,
            &mut my_username,
            existing_username,
            tx,
            &span,
        )
        .await;

        /*********** Assertion **********/
        assert!(my_username.is_empty());
        let mut output = String::new();
        let _ = buf_reader.read_line(&mut output).await;
        assert_eq!(output, format!("<{}>\n", messages::DUPLICATE_USER));
        assert!(line.is_empty());
    }

    // Tests the scenario where a connection is trying to send message without joining the chat.
    #[tokio::test]
    async fn test_handle_user_msgs_without_joining() {
        /*********** Preparation **********/
        let (client, server) = io::duplex(64);

        let mut buf_writer = BufWriter::new(client);
        let mut buf_reader = BufReader::new(server);

        let mut line = String::from("test line from user");

        // We send blank username to simulate the scenario where user has not joined the chat.
        let mut my_username = String::new();
        let mut output = String::new();

        let users = Arc::new(Mutex::new(HashMap::new()));

        // This is really not important as we are not using it in the test.
        let span = span!(Level::INFO, "test_handle_join_command_user_exists");

        /*********** Call **********/
        SimpleChatServer::handle_user_messages(
            users.clone(),
            &mut my_username,
            &mut buf_writer,
            &mut line,
            "",
            "",
            &span,
        )
        .await;

        /*********** Assertion **********/
        let _ = buf_reader.read_line(&mut output).await;
        assert_eq!(output, "Please join the chat first.\n");
        assert!(line.is_empty());
    }

    // Tests the scenario where a connection is trying to send message without joining the chat.
    #[tokio::test]
    async fn test_handle_user_msgs_after_joining() {
        /*********** Preparation **********/
        let (client, _) = io::duplex(64);

        let mut buf_writer = BufWriter::new(client);

        let mut line = String::from("test line from user");

        let mut my_username = String::from("user1");
        let sender_username = "user1";

        // Insert users to simulate the scenario where users have already joined the chat.
        let (tx1, mut rx1) = channel::<String>(5);
        let (tx2, mut rx2) = channel::<String>(5);
        let (tx3, mut rx3) = channel::<String>(5);
        let mut users_map: HashMap<String, Sender<String>> = HashMap::new();
        users_map.insert("user1".to_string(), tx1);
        users_map.insert("user2".to_string(), tx2);
        users_map.insert("user3".to_string(), tx3);

        let user_msg = "Hello All";

        let users = Arc::new(Mutex::new(users_map));

        // This is really not important as we are not using it in the test.
        let span = span!(Level::INFO, "test_handle_join_command_user_exists");

        /*********** Call **********/
        SimpleChatServer::handle_user_messages(
            users.clone(),
            &mut my_username,
            &mut buf_writer,
            &mut line,
            user_msg,
            sender_username,
            &span,
        )
        .await;

        /*********** Assertion **********/
        // All users except the sender user1 should receive the message.
        // Check for user2
        assert_eq!(
            rx2.recv().await.unwrap(),
            format!(
                "<{}> {} {}\n",
                messages::USER_MSG,
                sender_username,
                user_msg
            )
        );

        // Check for user3
        assert_eq!(
            rx3.recv().await.unwrap(),
            format!(
                "<{}> {} {}\n",
                messages::USER_MSG,
                sender_username,
                user_msg
            )
        );

        // users1 should not receive message from herself
        assert!(rx1.try_recv().is_err());
    }
}
