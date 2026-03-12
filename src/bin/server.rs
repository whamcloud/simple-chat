use tokio::{
    net::{TcpListener, TcpStream},
    io::{AsyncBufReadExt, BufReader},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    println!("chat server listening on 127.0.0.1:8080");

    loop {
        let (socket, addr) = listener.accept().await?;

        println!("client connected: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket).await {
                println!("connection error: {:?}", e);
            }
        });
    }
}

async fn handle_client(stream: TcpStream) -> anyhow::Result<()> {

    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        println!("received: {}", line);
    }

    Ok(())
}