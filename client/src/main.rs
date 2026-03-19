mod cli;
mod handler;

use clap::Parser;
use rand::distr::{Alphanumeric, SampleString};
use tokio::io::{AsyncBufReadExt, BufReader};
use std::io::{self, Write};
use cli::{Cli, Commands};
use handler::ChatHandler;

#[tokio::main]
async fn main() {
    let client_user = Alphanumeric.sample_string(&mut rand::rng(), 16);
    println!("Chat Client Started.");
    println!("User ID: {}", client_user);
    println!("Type 'help' for available commands.");

    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdin_line = String::new();
    let mut handler = ChatHandler::new();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        stdin_line.clear();
        if stdin.read_line(&mut stdin_line).await.unwrap() == 0 {
            break;
        }

        let input = stdin_line.trim();
        if input.is_empty() {
            continue;
        }

        let args = std::iter::once("chat-client").chain(input.split_whitespace());
        
        let cli = match Cli::try_parse_from(args) {
            Ok(cli) => cli,
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };

        match cli.command {
            Commands::Connect { address } => {
                if let Err(e) = handler.connect(&address, &client_user).await {
                    println!("Connection failed: {}", e);
                }
            }
            Commands::Send { message } => {
                let full_message = message.join(" ");
                let _ = handler.send(&full_message).await;
            }
            Commands::Leave => {
                handler.leave();
            }
            Commands::Exit => {
                println!("Exiting...");
                break;
            }
        }
    }
}