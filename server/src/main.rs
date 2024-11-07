use std::process::exit;

use clap::{command, Parser};
use server::SimpleChatServer;
use tracing::subscriber;
use tracing_subscriber::EnvFilter;

mod server;

/// Struct to represent command line args
#[derive(Clone, Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: Option<String>,

    #[arg(short, long, default_value = "8090")]
    port: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let server =
        SimpleChatServer::new(format!("{}:{}", args.ip.unwrap(), args.port.unwrap()).to_string());

    let subscriber = tracing_subscriber::fmt()
        .with_thread_ids(true)
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_env_filter(EnvFilter::new("info"))
        .finish();

    // Sets this subscriber as the global default for the duration of the entire program.
    subscriber::set_global_default(subscriber).expect("Error in setting logging mechanism");

    match server.start().await {
        Ok(_) => {
            tracing::info!("Server started successfully");
        }
        Err(e) => {
            tracing::error!("Error starting server: {}", e);
            exit(1);
        }
    }
}
