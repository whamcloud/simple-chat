use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;
use std::{env, thread};

const MSG_SIZE: usize = 1024;

fn main() {
    let (address, user_name) = match initialize_client() {
        Some(value) => value,
        None => return,
    };

    let client = start_client(address);

    let (tx, rx) = mpsc::channel::<String>();

    thread::spawn(move || read_server_message(client, rx));

    continue_chatting(tx, &user_name);
}

fn continue_chatting(tx: mpsc::Sender<String>, user: &str) {
    println!("Connected to chat server. Type 'leave' to leave the chat room.");
    loop {
        let mut buff = String::new();

        io::stdin()
            .read_line(&mut buff)
            .expect("reading from stdin failed");

        let msg = buff.trim();

        if msg == "leave" || tx.send(format!("{}:{}", user, msg)).is_err() {
            println!("Leave");
            break;
        }
    }
    println!("bye bye!");
}

fn start_client(address: String) -> TcpStream {
    let client = TcpStream::connect(address).expect("Stream failed to connect");
    client
        .set_nonblocking(true)
        .expect("Failed to initiate non-blocking");
    client
}

fn initialize_client() -> Option<(String, String)> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        println!("Usage: client <Server Address> <Server Port> <User Name>");
        return None;
    }
    let address = format!("{}:{}", args[1], args[2]);
    Some((address, args[3].clone()))
}

fn read_server_message(mut client: TcpStream, rx: mpsc::Receiver<String>) {
    loop {
        let mut buff = vec![0; MSG_SIZE];
        match client.read_exact(&mut buff) {
            Ok(_) => {
                let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                let msg = String::from_utf8(msg).expect("Invalid utf8 message");

                println!(">>> {}", msg);
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                println!("connection with server was severed");
                break;
            }
        }

        match rx.try_recv() {
            Ok(msg) => {
                let mut buff = msg.clone().into_bytes();
                buff.resize(MSG_SIZE, 0);
                client.write_all(&buff).expect("writing to socket failed");
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break,
        }

        thread::sleep(Duration::from_millis(100));
    }
}
