use anyhow::Result;
use clap::Parser;

mod connection;
mod repl;

#[derive(Parser)]
#[command(about = "simple chat client")]
struct Args {
    #[arg(long, env = "CHAT_HOST", default_value = "127.0.0.1")]
    host: String,

    #[arg(long, short, env = "CHAT_PORT", default_value_t = 3000u16)]
    port: u16,

    #[arg(long, short = 'u', env = "CHAT_USERNAME")]
    username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "client=warn".into()),
        )
        .init();

    let args = Args::parse();
    let url = format!(
        "ws://{}:{}/ws?username={}",
        args.host, args.port, args.username
    );
    connection::run(&url).await
}
