mod handler;

use tokio::net::TcpListener;
use tokio::sync::broadcast;
use handler::handle_client;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    
    let (tx, _rx) = broadcast::channel::<String>(10);
    
    println!("Server running on {}", addr);

    loop {
        let (socket, _addr) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let rx = tx.subscribe();

        tokio::spawn(async move {
            handle_client(socket, tx, rx).await;
        });
    }
}