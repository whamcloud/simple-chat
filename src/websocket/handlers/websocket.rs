// WebSocket connection and message handlers

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::WebSocketStream;

use crate::websocket::error::{Result, WebSocketError};
use crate::websocket::models::client::Client;
use crate::websocket::models::message::Message;
use crate::websocket::service::ChatService;

pub type WebSocket = WebSocketStream<TcpStream>;

/// Handle a new WebSocket connection
pub async fn handle_connection(ws: WebSocket, service: ChatService) -> Result<()> {
    let (mut ws_sender, mut ws_receiver) = ws.split();

    // Wait for join message
    let username = match ws_receiver.next().await {
        Some(Ok(WsMessage::Text(text))) => match Message::from_json(&text) {
            Ok(Message::Join { username }) => username,
            Ok(_) => {
                let err_msg = Message::Error {
                    message: "First message must be a Join message".to_string(),
                };
                let _ = ws_sender
                    .send(WsMessage::Text(err_msg.to_json().unwrap()))
                    .await;
                return Err(WebSocketError::InvalidMessage(
                    "Expected Join message".to_string(),
                ));
            }
            Err(e) => {
                let err_msg = Message::Error {
                    message: format!("Invalid message format: {}", e),
                };
                let _ = ws_sender
                    .send(WsMessage::Text(err_msg.to_json().unwrap()))
                    .await;
                return Err(WebSocketError::SerializationError(e));
            }
        },
        Some(Ok(_)) => {
            return Err(WebSocketError::InvalidMessage(
                "Expected text message".to_string(),
            ));
        }
        Some(Err(e)) => {
            return Err(WebSocketError::Internal(e.to_string()));
        }
        None => {
            return Err(WebSocketError::ConnectionClosed);
        }
    };

    // Create channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel();
    let client = Client::new(username.clone(), tx);

    // Try to add the client
    if let Err(e) = service.handle_join(username.clone(), client).await {
        let err_msg = Message::Error {
            message: format!("Failed to join: {}", e),
        };
        let _ = ws_sender
            .send(WsMessage::Text(err_msg.to_json().unwrap()))
            .await;
        return Err(e);
    }

    let username_clone = username.clone();
    let service_clone = service.clone();

    // Spawn task to forward messages from channel to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = msg.to_json() {
                if ws_sender.send(WsMessage::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages from WebSocket
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(WsMessage::Text(text)) => {
                    match Message::from_json(&text) {
                        Ok(Message::Send { content }) => {
                            service_clone.handle_send(&username_clone, content).await;
                        }
                        Ok(Message::Leave) => {
                            service_clone.handle_leave(&username_clone).await;
                            break;
                        }
                        Ok(_) => {
                            // Ignore other message types
                        }
                        Err(_) => {
                            // Ignore invalid messages
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    break;
                }
                Err(_) => {
                    break;
                }
                _ => {}
            }
        }
        service_clone.handle_leave(&username_clone).await;
    });

    // Wait for either task to complete
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }

    service.handle_leave(&username).await;
    Ok(())
}
