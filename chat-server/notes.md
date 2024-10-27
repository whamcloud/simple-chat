### Requirements:

- **User Management:** Each user is uniquely identified by a username. It will prompt the user for a username and check for uniqueness.
- **Message Broadcasting:** Messages are broadcasted to all users except the sender.
- **Leave or Disconnect:** When a user sends a /leave message or disconnects, the server removes the user from the active user list.
- **Threaded for Concurrency:** Each client connection is handled in a separate thread for parallelism, ensuring low latency for multiple users.
- **Memory Efficient:** The server uses `Arc<Mutex<>>` and `Arc<String>` to share state (user connections and usernames) between threads with minimal memory overhead.