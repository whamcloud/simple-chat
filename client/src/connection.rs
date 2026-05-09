use std::{
    io::{IsTerminal, Write},
    sync::{Arc, Mutex},
};

use anyhow::Result;
use futures_util::StreamExt;
use proto::{ServerMsg, decode_server};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, Message, error::ProtocolError},
};

use crate::repl;

pub async fn run(url: &str) -> Result<()> {
    let ws_stream = match connect_async(url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            eprintln!("{}", format_connect_error(&e));
            return Ok(());
        }
    };
    let (mut write, mut read) = ws_stream.split();

    let input_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let recv_buf = Arc::clone(&input_buf);

    let is_tty = std::io::stdin().is_terminal();
    let stdin_task: std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>>>> = if is_tty {
        Box::pin(repl::run_repl(&mut write, input_buf))
    } else {
        Box::pin(repl::run_pipe(&mut write))
    };

    let recv_task = async move {
        while let Some(frame) = read.next().await {
            match frame {
                Ok(Message::Text(text)) => print_server_msg(text.as_str(), &recv_buf),
                Ok(Message::Close(_)) | Err(WsError::ConnectionClosed) => break,
                Err(WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake)) => break,
                Ok(_) => {}
                Err(e) => return Err(anyhow::anyhow!(e)),
            }
        }
        Ok::<(), anyhow::Error>(())
    };

    tokio::select! {
        res = stdin_task => res?,
        res = recv_task  => res?,
    }
    Ok(())
}

fn format_connect_error(e: &WsError) -> String {
    match e {
        WsError::Io(io_err) if io_err.kind() == std::io::ErrorKind::ConnectionRefused => {
            "error: connection refused — is the server running?".to_owned()
        }
        WsError::Http(response) => {
            format!(
                "error: server rejected connection (HTTP {})",
                response.status()
            )
        }
        _ => format!("error: {e}"),
    }
}

fn format_server_msg(msg: &ServerMsg) -> String {
    match msg {
        ServerMsg::Message { from, text } => format!("{from}: {text}\r\n"),
        ServerMsg::Welcome { username } => {
            format!("connected as {username}. commands: send <MSG>, leave\r\n")
        }
        ServerMsg::UsernameTaken { username } => {
            format!("error: username '{username}' is taken\r\n")
        }
        ServerMsg::UserJoined { username } => format!("*** {username} joined\r\n"),
        ServerMsg::UserLeft { username } => format!("*** {username} left\r\n"),
        ServerMsg::Error { reason } => format!("server error: {reason}\r\n"),
    }
}

fn print_server_msg(text: &str, input_buf: &Arc<Mutex<String>>) {
    let current = input_buf.lock().unwrap();

    // Clear current input line, print message, then reprint the buffer.
    print!("\r\x1b[2K");
    match decode_server(text) {
        Ok(msg) => print!("{}", format_server_msg(&msg)),
        Err(e) => tracing::warn!("parse error: {e}"),
    }
    // Reprint whatever the user had typed so far.
    print!("{}", *current);
    std::io::stdout().flush().ok();
}

#[cfg(test)]
mod tests {
    use proto::ServerMsg;

    use super::format_server_msg;

    #[test]
    fn formats_message() {
        let msg = ServerMsg::Message {
            from: "alice".into(),
            text: "hi there".into(),
        };
        let out = format_server_msg(&msg);
        assert!(out.contains("alice"));
        assert!(out.contains("hi there"));
    }

    #[test]
    fn formats_welcome() {
        let msg = ServerMsg::Welcome {
            username: "bob".into(),
        };
        let out = format_server_msg(&msg);
        assert!(out.contains("bob"));
    }

    #[test]
    fn formats_username_taken() {
        let msg = ServerMsg::UsernameTaken {
            username: "bob".into(),
        };
        let out = format_server_msg(&msg);
        assert!(out.contains("bob"));
        assert!(out.contains("taken"));
    }

    #[test]
    fn formats_user_joined() {
        let msg = ServerMsg::UserJoined {
            username: "carol".into(),
        };
        let out = format_server_msg(&msg);
        assert!(out.contains("carol"));
        assert!(out.contains("joined"));
    }

    #[test]
    fn formats_user_left() {
        let msg = ServerMsg::UserLeft {
            username: "carol".into(),
        };
        let out = format_server_msg(&msg);
        assert!(out.contains("carol"));
        assert!(out.contains("left"));
    }

    #[test]
    fn formats_error() {
        let msg = ServerMsg::Error {
            reason: "internal failure".into(),
        };
        let out = format_server_msg(&msg);
        assert!(out.contains("internal failure"));
    }
}
