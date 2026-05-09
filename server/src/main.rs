use axum::{Router, routing::get};
use clap::Parser;

use server::{state::AppState, ws::ws_handler};

#[derive(Parser)]
#[command(about = "simple chat server")]
struct Args {
    #[arg(long, env = "CHAT_HOST", default_value = "0.0.0.0")]
    host: String,

    #[arg(long, short, env = "CHAT_PORT", default_value_t = 3000u16)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=info".into()),
        )
        .init();

    let args = Args::parse();
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(AppState::new());

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", args.host, args.port)).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
