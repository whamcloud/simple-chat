use clap::Parser;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use simple_chat::protocol::{ClientEvent, ServerEvent};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    host: String,

    #[arg(long)]
    port: u16,

    #[arg(long)]
    username: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let address = format!("{}:{}", args.host, args.port);

    let stream = TcpStream::connect(address).await?;

    println!("connected to server");

    let (reader, mut writer) = stream.into_split();

    let mut socket_reader = BufReader::new(reader).lines();

    // Send JOIN message
    let join = ClientEvent::Join {
        username: args.username.clone(),
    };

    let join_msg = serde_json::to_string(&join)?;
    writer.write_all(join_msg.as_bytes()).await?;
    writer.write_all(b"\n").await?;

    // Spawn task to read messages from server
    tokio::spawn(async move {
        while let Ok(Some(line)) = socket_reader.next_line().await {
            match serde_json::from_str::<ServerEvent>(&line) {
                Ok(ServerEvent::Chat { user, message }) => {
                    println!("{}: {}", user, message);
                }
                Ok(ServerEvent::Error { reason }) => {
                    println!("Error: {}", reason);
                }
                Err(e) => {
                    println!("Invalid server message: {}", e);
                }
            }
        }
    });

    let stdin = BufReader::new(tokio::io::stdin());
    let mut stdin_lines = stdin.lines();

    while let Some(line) = stdin_lines.next_line().await? {
        if let Some(stripped) = line.strip_prefix("send ") {
    let message = stripped.to_string();

            let cmd = ClientEvent::Send { message };

            let msg = serde_json::to_string(&cmd)?;

            writer.write_all(msg.as_bytes()).await?;
            writer.write_all(b"\n").await?;
        } else if line == "leave" {
            let cmd = ClientEvent::Leave;

            let msg = serde_json::to_string(&cmd)?;

            writer.write_all(msg.as_bytes()).await?;
            writer.write_all(b"\n").await?;

            break;
        } else {
            println!("Commands:");
            println!("send <message>");
            println!("leave");
        }
    }

    Ok(())
}
