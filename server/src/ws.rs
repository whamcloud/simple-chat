use std::sync::Arc;

use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use proto::{ClientMsg, ServerMsg, decode_client, encode_server};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct WsParams {
    pub username: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if params.username.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "username required").into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, params.username, state))
}

async fn handle_socket(socket: WebSocket, username: String, state: AppState) {
    if !state.register(&username).await {
        tracing::info!("{username} rejected: username taken");
        let mut socket = socket;
        if let Ok(json) = encode_server(&ServerMsg::UsernameTaken {
            username: username.clone(),
        }) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
        return;
    }

    let mut rx = state.subscribe();

    let (mut sender, mut receiver) = socket.split();
    let welcome = ServerMsg::Welcome {
        username: username.clone(),
    };
    if let Ok(json) = encode_server(&welcome)
        && sender.send(Message::Text(json.into())).await.is_err()
    {
        state.deregister(&username).await;
        return;
    }

    tracing::info!("{username} joined");
    state.broadcast(Arc::new(ServerMsg::UserJoined {
        username: username.clone(),
    }));

    let uname = username.clone();
    let send_task = async {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let skip = match msg.as_ref() {
                        ServerMsg::Message { from, .. } => from == &uname,
                        ServerMsg::UserJoined { username: u } => u == &uname,
                        ServerMsg::UserLeft { username: u } => u == &uname,
                        _ => false,
                    };
                    if skip {
                        continue;
                    }
                    let Ok(json) = encode_server(&msg) else {
                        continue;
                    };
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!("{uname} lagged by {n} messages");
                }
                Err(RecvError::Closed) => break,
            }
        }
    };

    let uname = username.clone();
    let state_recv = state.clone();
    let recv_task = async {
        while let Some(Ok(frame)) = receiver.next().await {
            match frame {
                Message::Text(text) => match decode_client(text.as_str()) {
                    Ok(ClientMsg::Send { text: body }) => {
                        state_recv.broadcast(Arc::new(ServerMsg::Message {
                            from: uname.clone(),
                            text: body,
                        }));
                    }
                    Ok(ClientMsg::Leave) => break,
                    Err(e) => tracing::warn!("bad msg from {uname}: {e}"),
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
    };

    tokio::select! {
        () = send_task => {}
        () = recv_task => {}
    }

    tracing::info!("{username} left");
    state.deregister(&username).await;
    state.broadcast(Arc::new(ServerMsg::UserLeft { username }));
}
