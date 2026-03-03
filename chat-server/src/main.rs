use anyhow::Result;
use chat_server_lib::{ServerState, handle_connection};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

const SERVER_ADDR: &str = "0.0.0.0:8080";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind(SERVER_ADDR).await?;
    info!("Chat server listening on {}", SERVER_ADDR);

    let state = Arc::new(ServerState::new());

    loop {
        let (socket, _) = listener.accept().await?;
        let state = state.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, state).await {
                error!("Connection error: {:?}", e);
            }
        });
    }
}
