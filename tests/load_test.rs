use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::sleep;

const SERVER_ADDR: &str = "127.0.0.1:8080";
const CLIENT_COUNT: usize = 10000;

async fn start_server() -> Child {
    let _ = Command::new("cargo")
        .args(&["build", "--bin", "server"])
        .status();
    let child = Command::new("cargo")
        .args(&["run", "--bin", "server"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let start = Instant::now();
    loop {
        if TcpStream::connect(SERVER_ADDR).await.is_ok() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Server timeout");
        }
        sleep(Duration::from_millis(100)).await;
    }
    child
}

#[tokio::test]
async fn test_high_concurrency_load() {
    let mut server_process = start_server().await;

    let mut handles = vec![];

    println!("Starting load test with {} clients...", CLIENT_COUNT);

    for i in 0..CLIENT_COUNT {
        handles.push(tokio::spawn(async move {
            match TcpStream::connect(SERVER_ADDR).await {
                Ok(mut stream) => {
                    let username = format!("Bot{}", i);
                    if let Err(_) = stream.write_all(format!("{}\n", username).as_bytes()).await {
                        return false;
                    }
                    sleep(Duration::from_millis(500)).await;
                    true
                }
                Err(_) => false,
            }
        }));
    }

    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap_or(false) {
            success_count += 1;
        }
    }

    server_process.kill().expect("Failed to kill server");

    println!(
        "Load Test Result: {}/{} clients connected successfully.",
        success_count, CLIENT_COUNT
    );

    assert_eq!(
        success_count, CLIENT_COUNT,
        "Server dropped connections under load!"
    );
}
