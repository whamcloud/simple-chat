use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    terminal,
};
use futures_util::{SinkExt, StreamExt};
use proto::{ClientMsg, encode_client};
use tokio::io::AsyncBufReadExt;
use tokio_tungstenite::tungstenite::Message;

struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

pub async fn run_repl<W>(write: &mut W, input_buf: Arc<Mutex<String>>) -> Result<()>
where
    W: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    terminal::enable_raw_mode()?;
    let _guard = RawModeGuard;
    repl_loop(write, &input_buf).await
}

/// Non-TTY fallback: read lines from stdin and send them as commands.
pub async fn run_pipe<W>(write: &mut W) -> Result<()>
where
    W: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_owned();
        if line.is_empty() {
            continue;
        }
        match parse_command(&line) {
            Some(msg @ ClientMsg::Leave) => {
                write
                    .send(Message::Text(encode_client(&msg)?.into()))
                    .await
                    .ok();
                write.close().await.ok();
                return Ok(());
            }
            Some(msg) => {
                write
                    .send(Message::Text(encode_client(&msg)?.into()))
                    .await?;
            }
            None => {}
        }
    }
    // stdin closed — send leave
    let leave = encode_client(&ClientMsg::Leave)?;
    write.send(Message::Text(leave.into())).await.ok();
    write.close().await.ok();
    Ok(())
}

async fn repl_loop<W>(write: &mut W, input_buf: &Arc<Mutex<String>>) -> Result<()>
where
    W: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let mut reader = EventStream::new();

    while let Some(Ok(event)) = reader.next().await {
        let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event
        else {
            continue;
        };

        match code {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                println!();
                break;
            }
            KeyCode::Enter => {
                let line = {
                    let mut buf = input_buf.lock().unwrap();
                    let line = buf.clone();
                    buf.clear();
                    line
                };
                // Move to new line
                print!("\r\n");
                std::io::stdout().flush()?;

                let line = line.trim().to_owned();
                if line.is_empty() {
                    continue;
                }
                match parse_command(&line) {
                    Some(msg @ ClientMsg::Leave) => {
                        write
                            .send(Message::Text(encode_client(&msg)?.into()))
                            .await
                            .ok();
                        write.close().await.ok();
                        break;
                    }
                    Some(msg) => {
                        write
                            .send(Message::Text(encode_client(&msg)?.into()))
                            .await?;
                    }
                    None => {
                        println!("unknown command. use: `send <MSG>` or `leave`\r");
                        std::io::stdout().flush()?;
                    }
                }
            }
            KeyCode::Backspace => {
                let removed = input_buf.lock().unwrap().pop().is_some();
                if removed {
                    // erase character: backspace + space + backspace
                    print!("\x08 \x08");
                    std::io::stdout().flush()?;
                }
            }
            KeyCode::Char(c) => {
                input_buf.lock().unwrap().push(c);
                print!("{c}");
                std::io::stdout().flush()?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_command(line: &str) -> Option<ClientMsg> {
    if line == "leave" {
        Some(ClientMsg::Leave)
    } else {
        line.strip_prefix("send ").map(|text| ClientMsg::Send {
            text: text.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use proto::{ClientMsg, decode_client, encode_client};

    use super::parse_command;

    #[test]
    fn send_encodes_correctly() {
        let msg = ClientMsg::Send {
            text: "hello world".into(),
        };
        let json = encode_client(&msg).unwrap();
        assert_eq!(decode_client(&json).unwrap(), msg);
    }

    #[test]
    fn leave_encodes_correctly() {
        let json = encode_client(&ClientMsg::Leave).unwrap();
        assert_eq!(decode_client(&json).unwrap(), ClientMsg::Leave);
    }

    #[test]
    fn unknown_type_is_rejected() {
        assert!(decode_client(r#"{"type":"badcommand"}"#).is_err());
    }

    #[test]
    fn parse_leave() {
        assert_eq!(parse_command("leave"), Some(ClientMsg::Leave));
    }

    #[test]
    fn parse_send_simple() {
        assert_eq!(
            parse_command("send hello"),
            Some(ClientMsg::Send {
                text: "hello".into()
            })
        );
    }

    #[test]
    fn parse_send_preserves_inner_spaces() {
        assert_eq!(
            parse_command("send hello world"),
            Some(ClientMsg::Send {
                text: "hello world".into()
            })
        );
    }

    #[test]
    fn parse_send_prefix_only_returns_none() {
        assert_eq!(parse_command("send"), None);
    }

    #[test]
    fn parse_unknown_returns_none() {
        assert_eq!(parse_command("foo"), None);
    }

    #[test]
    fn parse_empty_returns_none() {
        assert_eq!(parse_command(""), None);
    }
}
