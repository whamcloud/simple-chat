// Chat server binary

use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

mod websocket;

use websocket::config::Config;
use websocket::handlers::websocket::handle_connection;
use websocket::repository::ChatRepository;
use websocket::service::ChatService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let addr = format!("{}:{}", config.host, config.port);

    let listener = TcpListener::bind(&addr).await?;
    println!("Server listening on: {}", addr);

    let repository = ChatRepository::new();
    let service = ChatService::new(repository);

    while let Ok((stream, _)) = listener.accept().await {
        let service = service.clone();
        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws) => {
                    if let Err(e) = handle_connection(ws, service).await {
                        eprintln!("Connection error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket handshake error: {}", e);
                }
            }
        });
    }

    Ok(())
}
