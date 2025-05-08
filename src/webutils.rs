
use std::collections::HashMap;
use std::net::SocketAddr;
use actix_web::{web, Responder, HttpResponse};
//use crate::SwarmWebMessage;
use tokio::{sync::mpsc};
use serde::{Serialize, Deserialize};
use tokio::sync::{broadcast::{channel, Sender}, Mutex};
use std::sync::Arc;

//#[path = "../src/bin/server.rs"]
//mod server;

#[derive(Deserialize)]
pub struct MyQueryParams {
    user_name: String,
}

// Structure to hold user data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub username: String,
    pub addr: SocketAddr,
    //ws_stream: WebSocketStream<TcpStream>, // do we need this?
    pub lifetime_cnt: i32,
}

// Structure to hold server state
#[derive(Debug, Clone)]
pub struct ServerState {
    pub users: Arc<Mutex<HashMap<SocketAddr, User>>>,
    //pub users: Mutex<HashMap<SocketAddr,User>> , // map addr to username
    //users: Mutex<HashMap<SocketAddr,(String,i32)>> , // map addr to username
    pub bcast_tx: Sender<String>, // broadcast channel for sending messages to all users
}

//Make sure to broadcast to all others except sender
impl ServerState {
    pub async fn broadcast_message(&self, addr: &SocketAddr, message: String) {
        let users = self.users.lock().await;
        if let Some(sname) = users.get(addr).map(|t| &t.username){
            let sender_name = sname;
            let full_msg = format!("{}: {}", sender_name, message);

            for (user_addr, _) in users.iter() {
                if user_addr != addr {
                    self.bcast_tx.send(full_msg.clone()).unwrap();
                }
            }
        }
        /*let sender_name = users.get(addr).as_ref().0.unwrap();
        let full_msg = format!("{}: {}", sender_name, message);

        for (user_addr, _) in users.iter() {
            if user_addr != addr {
                self.bcast_tx.send(full_msg.clone()).unwrap();
            }
        }*/
    }
}

//get handler
pub async fn statsall(query: web::Query<MyQueryParams>, state: web::Data<Arc<ServerState>>) -> impl Responder {
    let users_map = state.users.lock().await; // be careful with unwrap
    let count = users_map.len();
    let s_users_map = users_map.clone();
    for (sock_add, U) in s_users_map.into_iter() {
        println!("{} / {}", U.username, U.lifetime_cnt);
    }
    HttpResponse::Ok().body(format!("Total users: {}", count))
    //HttpResponse::Ok().body(format!("Data sent to libp2p swarm: {}", users.keys().len()))
}

//get handler
pub async fn index(query: web::Query<MyQueryParams>) -> impl Responder {
    let name = &query.user_name;

    HttpResponse::Ok().body(format!("Data sent to libp2p swarm: {}", name))

    //let sender_clone = sender.get_ref().clone();

    // Send the query parameter data to the libp2p swarm through the sender channel
    /*if sender_clone.send(SwarmWebMessage::DataGet(name.to_owned())).await.is_ok() {
    HttpResponse::Ok().body(format!("Data sent to libp2p swarm: {}", name))
} else {
    HttpResponse::InternalServerError().body("Failed to send data to libp2p swarm")
}*/
}

pub async fn indexPost(query: web::Json<MyQueryParams>) -> impl Responder {
    /*let model_type = &query.mtype;
let model_loc = &query.mlocation;
let model_data_loc = &query.mdataloc;
let model_algo = &query.malgo;
let model_output_loc = &query.moutputloc;*/

    HttpResponse::Ok().body("Data sent to libp2p swarm")
    //let sender_clone = sender.get_ref().clone();

    /*let metadata = serde_json::json!(
                        {
                          "mtype": model_type,
                          "mlocation": model_loc,   //e.g. s3://picxelate/dgp/create_model.py
                          "mdataloc": model_data_loc,
                          "malgo": model_algo,
                          "moutputloc": model_output_loc,
                        }
                        );*/
    //e.g. s3://picxelate/dgp/create_model.py
    /*let metadata =     r#"{
                          "mtype": model_type,
                          "mlocation": model_loc,
                          "mdataloc": model_data_loc,
                          "malgo": model_algo,
                          "moutputloc": model_output_loc,
                        }"#;*/

    //do metadata check about known peers

    // Send the query parameter data to the libp2p swarm through the sender channel
    //if sender_clone.send(SwarmWebMessage::Data(model_loc.to_owned())).await.is_ok() {
    /*if sender_clone.send(SwarmWebMessage::Data(query)).await.is_ok() {
    //HttpResponse::Ok().body(format!("Data sent to libp2p swarm: {}", model_loc))
    HttpResponse::Ok().body("Data sent to libp2p swarm")

} else {
    HttpResponse::InternalServerError().body("Failed to send data to libp2p swarm")
}*/
}
