use simple_chat::ChatServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
        + ":"
        + &std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let server = ChatServer::new();
    server.run(&addr).await?;

    Ok(())
}
