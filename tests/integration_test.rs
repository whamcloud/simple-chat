use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::sleep;

const SERVER_ADDR: &str = "127.0.0.1:8080";

async fn start_server() -> Child {
    let status = Command::new("cargo")
        .args(&["build", "--bin", "server"])
        .status()
        .expect("Failed to build server");
    assert!(status.success(), "Server binary failed to build");

    let child = Command::new("cargo")
        .args(&["run", "--bin", "server"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server process");

    let start = Instant::now();
    loop {
        if TcpStream::connect(SERVER_ADDR).await.is_ok() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            panic!(
                "Server timed out: Could not connect to {} after 5s",
                SERVER_ADDR
            );
        }
        sleep(Duration::from_millis(100)).await;
    }

    child
}

#[tokio::test]
async fn test_server_client_interaction() {
    let mut server_process = start_server().await;

    let mut stream_a = TcpStream::connect(SERVER_ADDR).await.unwrap();
    let (read_a, mut write_a) = stream_a.split();
    let mut _reader_a = BufReader::new(read_a);

    let mut stream_b = TcpStream::connect(SERVER_ADDR).await.unwrap();
    let (read_b, mut write_b) = stream_b.split();
    let mut reader_b = BufReader::new(read_b);

    write_a.write_all(b"Alice\n").await.unwrap();
    write_b.write_all(b"Bob\n").await.unwrap();

    sleep(Duration::from_millis(100)).await;

    let msg = "Integration Test Message";
    write_a
        .write_all(format!("{}\n", msg).as_bytes())
        .await
        .unwrap();

    let mut line = String::new();
    let mut found = false;

    for _ in 0..5 {
        line.clear();
        let read_result =
            tokio::time::timeout(Duration::from_secs(1), reader_b.read_line(&mut line)).await;

        if let Ok(Ok(bytes)) = read_result {
            if bytes == 0 {
                break;
            }
            if line.contains(msg) {
                assert!(line.contains("Alice"), "Message MUST contain username");
                found = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(found, "Bob never received Alice's message!");
    server_process.kill().expect("Failed to kill server");
}
