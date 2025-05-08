
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast::{channel, Sender}, Mutex};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};
use std::sync::Arc;
use actix_web::{web, App, HttpServer, Responder};
use serde::{Serialize, Deserialize};
extern crate actix_web;

//mod webutils;

use chat_app::webutils::{statsall, index, indexPost, User, ServerState};

async fn handle_connection(
    addr: SocketAddr,
    mut ws_stream: WebSocketStream<TcpStream>,
    state: Arc<ServerState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Send a welcome message
    ws_stream.send(Message::text("Welcome to chat! Type '/join <username>' to join.".to_string())).await?;

    let mut bcast_rx = state.bcast_tx.subscribe();
    let mut username: Option<String> = None;

    loop {
        tokio::select! {
            // Handle incoming messages from the client
            incoming = ws_stream.next() => {
                match incoming {
                    Some(Ok(msg)) => {
                        if let Some(text) = msg.as_text() {
                            if text.starts_with("/join ") {
                                // Handle user joining
                                let new_username = text[6..].trim().to_string();
                                let mut users = state.users.lock().await;

                                //if users.values().any(|name| &name.0 == &new_username) {
                                if users.values().any(|name| &name.username == &new_username) {
                                    ws_stream.send(Message::text("Username already taken.".to_string())).await?;
                                } else {
                                    if users.contains_key(&addr) == true {
                                        /*if let Some(tuple_ref) = users.get(&addr) {
                                            if let Some(fe) = tuple_ref.as_ref().map(|t| &t.0)
                                              && let Some(se) = tuple_ref.as_ref().map(|t| &t.1) {
                                                users.insert(addr, (fe,se+1));
                                            }
                                        }*/
                                        /*if let Some(fv) = users.get(&addr).cloned().map(|t| &t.0){
                                            let sname = fv;
                                            if let Some(sv) = users.get(&addr).cloned().map(|t| &t.1){
                                                let count = sv;
                                                users.insert(addr, (sname.clone(),count+1));
                                            }
                                        }*/

                                        if let Some(U) = users.get(&addr).cloned() {
                                            //users.insert(addr, (sname.clone(), count + 1));
                                            //users.insert(addr, User(sname.clone(), addr, count+1));
                                            users.insert(addr, User{username: U.username, addr: addr, lifetime_cnt: U.lifetime_cnt+1});
                                        }
                                        /*users.entry(addr).
                                             .and_modify(|entry| entry.1+=1)
                                             .or_insert("default_name".to_string(),1);*/
                                        //users.insert(addr, ((users.get(&addr)).as_ref().0,(users.get(&addr)).as_ref().1+1));
                                    }
                                    else {
                                        users.insert(addr, User{username:new_username.clone(), addr: addr, lifetime_cnt: 0});
                                        }
                                    ws_stream.send(Message::text(format!("Joined as {}", new_username))).await?;
                                    state.bcast_tx.send(format!("{} has joined the chat.", new_username))?;
                                    username = Some(new_username);
                                }
                            } else if text == "/leave" {
                                // Handle user leaving
                                if let Some(name) = username.take() {
                                    let mut users = state.users.lock().await;
                                    users.remove(&addr);
                                    state.bcast_tx.send(format!("{} has left the chat.", name))?;
                                    return Ok(());
                                }
                            } else if let Some(_) = username {
                                // Broadcast regular messages
                                state.broadcast_message(&addr, text.into()).await;
                            } else {
                                ws_stream.send(Message::text("Please join with '/join <username>' first.".to_string())).await?;
                            }
                        }
                    }
                    Some(Err(err)) => return Err(err.into()),
                    None => return Ok(()),
                }
            }

            // Handle messages from the broadcast channel
            msg = bcast_rx.recv() => {
                ws_stream.send(Message::text(msg?)).await?;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (bcast_tx, _) = channel(16);
    let state = Arc::new(ServerState {
        users: Arc::new(Mutex::new(HashMap::new())),
        bcast_tx: bcast_tx.clone(),
    });

    //web data processing
    //let (web_sender, mut web_receiver) = mpsc::channel::<SwarmWebMessage>(100);

    let shared_state = web::Data::new(state.clone());
    //Web server
    //Start the web server
    let server = HttpServer::new(move || {
        App::new()
            //.app_data(received_data.clone())
            .app_data(shared_state.clone())
            //.app_data(swarm_controller.clone())
            //.route("/", web::post().to(receive_data))
            .route("/statsuser", web::get().to(index))
            .route("/statsall", web::get().to(statsall))
            .route("/updateuser", web::post().to(indexPost))
    })
        //.bind("127.0.0.1:8080")?
        .bind("0.0.0.0:8080")?
        .run();
    //.await;

    // Start the event loop
    tokio::spawn(server);

    let listener = TcpListener::bind("127.0.0.1:2000").await?;
    println!("Listening on port 2000");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("socket: {:?}", socket);
        println!("address: {:?}", addr);
        let conn_state = state.clone();

        tokio::spawn(async move {
            let ws_stream = ServerBuilder::new().accept(socket).await?;
            handle_connection(addr, ws_stream, conn_state).await
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;
    use tokio_websockets::{Message, ServerBuilder};
    use tokio::net::{TcpListener, TcpStream};
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_user_join() {
        let (bcast_tx, _) = broadcast::channel(16);
        let state = Arc::new(ServerState {
            users: Mutex::new(HashMap::new()),
            bcast_tx: bcast_tx.clone(),
        });

        // Create a TCP listener for the server
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

        // Simulate a client connecting to the server
        tokio::spawn(async move {
            let (client_socket, _) = listener.accept().await.unwrap();
            let (mut ws_stream, _) = ServerBuilder::new().accept(client_socket).await.unwrap(); // Correctly destructuring the tuple
            handle_connection("127.0.0.1:8080".parse().unwrap(), ws_stream, state.clone()).await.unwrap();
        });

        // Create a client-side TCP stream
        let client_socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
        let (mut client_ws_stream, _) = ServerBuilder::new().accept(client_socket).await.unwrap(); // Correctly destructuring the tuple

        // Simulate client sending a message
        let msg = Message::text("/join username");
        client_ws_stream.send(msg).await.unwrap();  // Client sends a join message

        // Simulate receiving the broadcasted message from the server
        if let Some(Ok(received)) = client_ws_stream.next().await {
            assert!(received.as_text().unwrap().contains("username has joined the chat"));
        }
    }
}

