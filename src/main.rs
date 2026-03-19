fn main() {
    println!("Simple Chat - Project Usage Guide");
    println!("====================================\n");

    println!("To get the chat system running, follow these steps in separate terminals:\n");

    println!("1.  Start the Server:");
    println!("    cargo run -p server\n");

    println!("2.  Start the first Client:");
    println!("    cargo run -p client\n");

    println!("3.  Start the second Client:");
    println!("    cargo run -p client\n");

    println!("Once connected, any message typed in one client will be broadcast to all other connected clients.");
    println!("The server runs locally on 127.0.0.1:8080.\n");
}
