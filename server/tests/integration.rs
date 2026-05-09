use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use proto::{ClientMsg, ServerMsg, decode_server, encode_client};
use tokio_tungstenite::{connect_async, tungstenite::Message};

async fn spawn_server() -> u16 {
    use axum::{Router, routing::get};
    use server::{state::AppState, ws::ws_handler};

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(AppState::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    port
}

async fn connect(
    port: u16,
    username: &str,
) -> (
    impl futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    impl futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
) {
    let url = format!("ws://127.0.0.1:{port}/ws?username={username}");
    let (ws, _) = connect_async(&url).await.unwrap();
    ws.split()
}

async fn recv_msg(
    rx: &mut (
             impl futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
             + Unpin
         ),
) -> ServerMsg {
    let frame = rx.next().await.unwrap().unwrap();
    match frame {
        Message::Text(t) => decode_server(t.as_str()).unwrap(),
        other => panic!("expected text frame, got {other:?}"),
    }
}

async fn send_msg(
    tx: &mut (impl futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    msg: &ClientMsg,
) {
    let json = encode_client(msg).unwrap();
    tx.send(Message::Text(json.into())).await.unwrap();
}

#[tokio::test]
async fn empty_username_is_rejected() {
    let port = spawn_server().await;
    let url = format!("ws://127.0.0.1:{port}/ws?username=");
    let result = connect_async(&url).await;
    assert!(result.is_err(), "empty username should be rejected");
    assert!(matches!(
        result.unwrap_err(),
        tokio_tungstenite::tungstenite::Error::Http(_)
    ));
}

#[tokio::test]
async fn missing_username_param_is_rejected() {
    let port = spawn_server().await;
    let url = format!("ws://127.0.0.1:{port}/ws");
    let result = connect_async(&url).await;
    assert!(result.is_err(), "missing username param should be rejected");
    assert!(matches!(
        result.unwrap_err(),
        tokio_tungstenite::tungstenite::Error::Http(_)
    ));
}

#[tokio::test]
async fn connect_sends_welcome() {
    let port = spawn_server().await;
    let (_, mut rx) = connect(port, "alice").await;
    let msg = recv_msg(&mut rx).await;
    assert_eq!(
        msg,
        ServerMsg::Welcome {
            username: "alice".into()
        }
    );
}

#[tokio::test]
async fn duplicate_username_rejected() {
    let port = spawn_server().await;
    let (_tx1, mut rx1) = connect(port, "alice").await;
    assert!(matches!(
        recv_msg(&mut rx1).await,
        ServerMsg::Welcome { .. }
    ));

    let (_tx2, mut rx2) = connect(port, "alice").await;
    let msg = recv_msg(&mut rx2).await;
    assert_eq!(
        msg,
        ServerMsg::UsernameTaken {
            username: "alice".into()
        }
    );
}

#[tokio::test]
async fn message_broadcast_to_others() {
    let port = spawn_server().await;

    let (mut alice_tx, mut alice_rx) = connect(port, "alice").await;
    assert!(matches!(
        recv_msg(&mut alice_rx).await,
        ServerMsg::Welcome { .. }
    ));

    let (_bob_tx, mut bob_rx) = connect(port, "bob").await;
    assert!(matches!(
        recv_msg(&mut bob_rx).await,
        ServerMsg::Welcome { .. }
    ));
    // alice gets UserJoined for bob
    let _ = recv_msg(&mut alice_rx).await;

    send_msg(
        &mut alice_tx,
        &ClientMsg::Send {
            text: "hello".into(),
        },
    )
    .await;

    // bob gets the message
    let msg = recv_msg(&mut bob_rx).await;
    assert_eq!(
        msg,
        ServerMsg::Message {
            from: "alice".into(),
            text: "hello".into()
        }
    );

    // alice does NOT get her own message within 200ms
    let alice_received =
        tokio::time::timeout(Duration::from_millis(200), recv_msg(&mut alice_rx)).await;
    assert!(
        alice_received.is_err(),
        "alice should not receive her own message"
    );
}

#[tokio::test]
async fn leave_removes_user() {
    let port = spawn_server().await;

    let (mut alice_tx, mut alice_rx) = connect(port, "alice").await;
    assert!(matches!(
        recv_msg(&mut alice_rx).await,
        ServerMsg::Welcome { .. }
    ));

    let (_bob_tx, mut bob_rx) = connect(port, "bob").await;
    assert!(matches!(
        recv_msg(&mut bob_rx).await,
        ServerMsg::Welcome { .. }
    ));
    // alice sees bob join
    let _ = recv_msg(&mut alice_rx).await;

    send_msg(&mut alice_tx, &ClientMsg::Leave).await;

    // bob sees alice leave
    let msg = recv_msg(&mut bob_rx).await;
    assert_eq!(
        msg,
        ServerMsg::UserLeft {
            username: "alice".into()
        }
    );

    // re-connect as alice succeeds
    let (_, mut rx3) = connect(port, "alice").await;
    assert!(matches!(
        recv_msg(&mut rx3).await,
        ServerMsg::Welcome { .. }
    ));
}

#[tokio::test]
async fn disconnect_cleans_up_state() {
    let port = spawn_server().await;

    let (alice_tx, mut alice_rx) = connect(port, "alice").await;
    assert!(matches!(
        recv_msg(&mut alice_rx).await,
        ServerMsg::Welcome { .. }
    ));

    let (_bob_tx, mut bob_rx) = connect(port, "bob").await;
    assert!(matches!(
        recv_msg(&mut bob_rx).await,
        ServerMsg::Welcome { .. }
    ));
    // alice sees bob join
    let _ = recv_msg(&mut alice_rx).await;

    // drop alice's connection without sending Leave
    drop(alice_tx);
    drop(alice_rx);

    // bob sees alice leave (may take a moment)
    let msg = tokio::time::timeout(Duration::from_secs(2), recv_msg(&mut bob_rx))
        .await
        .expect("timed out waiting for UserLeft");
    assert_eq!(
        msg,
        ServerMsg::UserLeft {
            username: "alice".into()
        }
    );
}
