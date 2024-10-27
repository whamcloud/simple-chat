### Requirements:

- **Command-line Arguments & Environment Variables:**
    - Uses the `clap` crate to parse command-line arguments (host, port, and username).
    - Environment variables (HOST, PORT, and USERNAME) are fallback options.
- **Networking:**
    - A `TcpStream` is created to connect to the specified server.
    - The `Poll` and `Events` are used to asynchronously handle events like reading from the TCP stream or stdin.
- **Handling Events:**
    - The client listens for incoming messages from the server or inputs from the user.
    - When the user types send <MSG>, the message is sent to the server.
    - If leave is typed, the client disconnects and exits.
- **Async I/O:**
    - mio is used for non-blocking, event-based network communication.
    - `Stdin` is handled in a separate thread, and input is sent to the main loop using an mpsc channel.
- **Interactive Prompt:**
    The user can type send <MSG> to send a message and leave to exit the program.

### Usage

Run the client using:
```sh
cargo run -- --host {} --port {} --username "{}"
```
