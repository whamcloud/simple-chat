use chrono::Local;
use colored::*;
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

const MOVE_CURSOR_UP: &str = "\x1b[1A";
const CLEAR_LINE: &str = "\x1b[2K";
const CARRIAGE_RETURN: &str = "\r";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("{} client <host> <port> <username>", "Usage:".red().bold());
        return Ok(());
    }
    let (host, port, username) = (&args[1], &args[2], &args[3]);
    let addr = format!("{}:{}", host, port);

    print!("\x1b[2J\x1b[1;1H");
    println!("   {} {}", "Connected to:".green(), addr.yellow());
    println!("   {} {}\n", "    Username:".green(), username.yellow());
    println!(
        "{}\n",
        "Type 'send <msg>' to chat, 'leave' to quit."
            .dimmed()
            .italic()
    );

    let mut stream = TcpStream::connect(&addr).await?;
    stream
        .write_all(format!("{}\n", username).as_bytes())
        .await?;

    let (reader, mut writer) = stream.into_split();

    tokio::spawn(async move {
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("\n{}", "[Server closed connection]".red().bold());
                    std::process::exit(0);
                }
                Ok(_) => {
                    let text = line.trim_end();

                    let time = Local::now().format("%H:%M").to_string();
                    let timestamp = format!("[{}]", time).dimmed();

                    print!("{}{}", CARRIAGE_RETURN, CLEAR_LINE);

                    if let Some((name, msg)) = text.split_once(": ") {
                        println!("{} {}: {}", timestamp, name.bold().cyan(), msg);
                    } else {
                        println!("{} {}", timestamp, text.italic());
                    }

                    print!("{}", "> ".green().bold());
                    use std::io::Write;
                    let _ = std::io::stdout().flush();

                    line.clear();
                }
                Err(_) => {
                    std::process::exit(1);
                }
            }
        }
    });

    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        stdout
            .write_all(format!("{}", "> ".green().bold()).as_bytes())
            .await?;
        stdout.flush().await?;

        line.clear();
        let bytes_read = stdin.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let input = line.trim();

        if input == "leave" {
            println!("{}", "Leaving chat...".yellow());
            break;
        } else if let Some(msg) = input.strip_prefix("send ") {
            writer.write_all(format!("{}\n", msg).as_bytes()).await?;

            let time = Local::now().format("%H:%M").to_string();
            let timestamp = format!("[{}]", time).dimmed();

            print!("{}{}{}", MOVE_CURSOR_UP, CLEAR_LINE, CARRIAGE_RETURN);

            println!("{} {}: {}", timestamp, "You".bold().green(), msg);
        } else {
            println!("{}", "Invalid command. Use 'send <msg>' or 'leave'".red());
        }
    }

    Ok(())
}
