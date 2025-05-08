use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use http::Uri;
use structopt::StructOpt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_websockets::{ClientBuilder, Message};

#[derive(StructOpt)]
struct Cli {
    #[structopt(short, long, default_value = "127.0.0.1")]
    host: String,

    #[structopt(short, long, default_value = "2000")]
    port: String,

    #[structopt(short, long)]
    username: String,
}

#[tokio::main]
async fn main() -> Result<(), tokio_websockets::Error> {
    let args = Cli::from_args();
    let ws_url = format!("ws://{}:{}/", args.host, args.port);

    let (mut ws_stream, _) = ClientBuilder::from_uri(Uri::from_maybe_shared(ws_url).unwrap())
        .connect()
        .await?;

    let stdin = tokio::io::stdin();
    let mut stdin = BufReader::new(stdin).lines();

    // Send the join message immediately
    ws_stream
        .send(Message::text(format!("/join {}", args.username)))
        .await?;

    loop {
        tokio::select! {
            incoming = ws_stream.next() => {
                match incoming {
                    Some(Ok(msg)) => {
                        if let Some(text) = msg.as_text() {
                            println!("From server: {}", text);
                        }
                    },
                    Some(Err(err)) => return Err(err.into()),
                    None => return Ok(()),
                }
            }

            res = stdin.next_line() => {
                match res {
                    Ok(None) => return Ok(()),
                    Ok(Some(line)) => {
                        if line.starts_with("send ") {
                            let msg = line[5..].to_string();
                            ws_stream.send(Message::text(msg)).await?;
                        } else if line == "leave" {
                            ws_stream.send(Message::text("/leave".to_string())).await?;
                            return Ok(());
                        } else {
                            println!("Unknown command.");
                        }
                    }
                    Err(err) => return Err(err.into()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use tokio_websockets::Message;
    use futures::*;

    #[tokio::test]
    async fn test_handle_server_messages() {
        let messages = vec![
            Message::text("INFO: Welcome to the chat"),
            Message::text("ERROR: Username already taken"),
            Message::text("Hello from another user"),
        ];

        let ws_stream = stream::iter(messages);

        let mut results = vec![];

        ws_stream
            .for_each(|msg| {
                if let Some(text) = msg.as_text() {
                    if text.starts_with("ERROR:") {
                        results.push(format!("Error from server: {}", &text[7..]));
                    } else if text.starts_with("INFO:") {
                        results.push(format!("Info: {}", &text[6..]));
                    } else {
                        results.push(format!("From server: {}", text));
                    }
                }
                futures::future::ready(())
            })
            .await;

        assert_eq!(results[0], "Info: Welcome to the chat");
        assert_eq!(results[1], "Error from server: Username already taken");
        assert_eq!(results[2], "From server: Hello from another user");
    }
}