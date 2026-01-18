use anyhow::Result;
use chat_client_lib::run_client;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    host: String,

    port: u16,

    username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port);

    run_client(addr, args.username).await
}
