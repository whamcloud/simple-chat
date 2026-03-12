use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    println!("Chat server running on 8080");

    loop {
        let (_socket, addr) = listener.accept().await?;
        println!("client connected: {}", addr);
    }
}