use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let _stream = TcpStream::connect("127.0.0.1:8080").await?;

    println!("connected to server");

    Ok(())
}