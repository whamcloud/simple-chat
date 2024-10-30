use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc;
use std::{env, thread};

const MSG_SIZE: usize = 1024;

fn main() {
    let address = match decode_input() {
        Some(value) => value,
        None => return,
    };

    let server = start_server(address);

    let clients = vec![];
    let (tx, rx) = mpsc::channel::<(String, std::net::SocketAddr)>();

    wait_for_client(server, tx, clients, rx);
}

fn wait_for_client(
    server: TcpListener,
    tx: mpsc::Sender<(String, SocketAddr)>,
    mut clients: Vec<std::net::TcpStream>,
    rx: mpsc::Receiver<(String, SocketAddr)>,
) -> ! {
    loop {
        if let Ok((socket, addr)) = server.accept() {
            initiate_clients(addr, &tx, &mut clients, socket);
        }

        if let Ok((msg, sender_addr)) = rx.try_recv() {
            // Receive both the message and sender's address
            clients = clients
                .into_iter()
                .filter_map(|mut client| {
                    // Get the peer address to compare with the sender
                    if let Ok(peer_addr) = client.peer_addr() {
                        if peer_addr != sender_addr {
                            // Skip sending to the sender
                            let mut buff = msg.clone().into_bytes();
                            buff.resize(MSG_SIZE, 0);
                            client.write_all(&buff).map(|_| client).ok()
                        } else {
                            Some(client)
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
        }
    }
}

fn initiate_clients(
    addr: std::net::SocketAddr,
    tx: &mpsc::Sender<(String, SocketAddr)>,
    clients: &mut Vec<std::net::TcpStream>,
    socket: std::net::TcpStream,
) {
    println!("Client {} connected", addr);

    let tx = tx.clone();
    clients.push(socket.try_clone().expect("Failed to clone client"));

    thread::spawn(move || read_client_message(socket, addr, tx));
}

fn read_client_message(
    mut socket: std::net::TcpStream,
    addr: std::net::SocketAddr,
    tx: mpsc::Sender<(String, SocketAddr)>,
) {
    loop {
        let mut buff = vec![0; MSG_SIZE];

        match socket.read_exact(&mut buff) {
            Ok(_) => {
                let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                let msg = String::from_utf8(msg).expect("Invalid utf8 message");

                println!("{}: {:?}", addr, msg);
                tx.send((msg, addr)).expect("failed to send msg to rx");
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            Err(_) => {
                println!("Closing connection with: {}", addr);
                break;
            }
        }
    }
}

fn start_server(address: String) -> TcpListener {
    let server = TcpListener::bind(address).expect("Listener failed to bind");
    server
        .set_nonblocking(true)
        .expect("failed to initialize non-blocking");
    println!("Server started listening");
    println!();
    server
}

fn decode_input() -> Option<String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: client <Server Port>");
        return None;
    }
    let address = format!("127.0.0.1:{}", args[1]);
    Some(address)
}
