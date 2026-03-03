use chat_server_lib::{ServerState, handle_connection};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn test_full_chat_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let state = Arc::new(ServerState::new());

    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let s = state_clone.clone();
            tokio::spawn(async move {
                handle_connection(socket, s).await.ok();
            });
        }
    });

    let mut client_a = TcpStream::connect(addr).await.unwrap();
    let mut buf_a = [0u8; 1024];

    let n = client_a.read(&mut buf_a).await.unwrap();
    assert!(String::from_utf8_lossy(&buf_a[..n]).contains("Please enter your username"));

    client_a.write_all(b"Alice\n").await.unwrap();

    let n = client_a.read(&mut buf_a).await.unwrap();
    assert!(String::from_utf8_lossy(&buf_a[..n]).contains("Welcome Alice"));

    let mut client_b = TcpStream::connect(addr).await.unwrap();
    let mut buf_b = [0u8; 1024];
    let _ = client_b.read(&mut buf_b).await.unwrap(); // Prompt
    client_b.write_all(b"Bob\n").await.unwrap(); // Username
    let _ = client_b.read(&mut buf_b).await.unwrap(); // Welcome

    let n = client_a.read(&mut buf_a).await.unwrap();
    let msg = String::from_utf8_lossy(&buf_a[..n]);
    assert!(msg.contains("Bob has joined"));

    client_b.write_all(b"send Hello World\n").await.unwrap();

    let n = client_a.read(&mut buf_a).await.unwrap();
    let msg = String::from_utf8_lossy(&buf_a[..n]);
    assert!(msg.contains("Bob: Hello World"));
}
