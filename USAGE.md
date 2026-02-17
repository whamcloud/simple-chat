# Simple Chat - Usage Guide

## Installation

### Prerequisites

- Rust 1.70 or later
- Cargo (comes with Rust)

Install Rust from: https://rustup.rs/

### Installing Pre-commit Hook (Optional)

To automatically check formatting, compilation, and linting before each commit:

```bash
# Copy the pre-commit hook
cp pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Building the Project

```bash
# Build debug version
cargo build

# Build release version (optimized)
cargo build --release
```

## Running the Server

```bash
# Run server (listens on 127.0.0.1:8080 by default)
cargo run --bin server

# Or run the compiled binary
./target/debug/server
```

The server will print:
```
Server listening on: 127.0.0.1:8080
```

## Running the Client

Open a new terminal and run:

```bash
# Run client with username
cargo run --bin client -- --username alice

# Or specify host and port
cargo run --bin client -- --username bob --host 127.0.0.1 --port 8080

# Short form
cargo run --bin client -- -u charlie -H 127.0.0.1 -p 8080
```

## Client Commands

Once connected, you can use these commands:

```
send <message>  - Send a message to all other users
leave           - Disconnect from the chat and exit
```

### Example Session

```
> send Hello everyone!
> send How's it going?
[alice]: Hi there!
[bob]: Great, thanks!
> leave
Disconnected
```

## Testing Multiple Clients

Open multiple terminals and run different clients:

**Terminal 1: Server**
```bash
cargo run --bin server
```

**Terminal 2: Alice**
```bash
cargo run --bin client -- --username alice
> send Hi, I'm Alice
```

**Terminal 3: Bob**
```bash
cargo run --bin client -- --username bob
[alice]: Hi, I'm Alice
> send Hello Alice, I'm Bob
```

**Terminal 4: Charlie**
```bash
cargo run --bin client -- --username charlie
[alice]: Hi, I'm Alice
[bob]: Hello Alice, I'm Bob
> send Hey everyone!
```

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_add_client
```

## Code Quality

```bash
# Format code
cargo fmt

# Check with clippy (linter)
cargo clippy

# Check clippy with warnings as errors
cargo clippy -- -D warnings
```

## Architecture

### Components

- **Server** (`src/server.rs`): Main server binary that accepts WebSocket connections
- **Client** (`src/client.rs`): CLI client that connects to the server
- **Models** (`src/websocket/models/`): Data structures for messages and clients
- **Repository** (`src/websocket/repository.rs`): State management for connected clients
- **Service** (`src/websocket/service.rs`): Business logic for chat operations
- **Handler** (`src/websocket/handlers/`): WebSocket connection handling
- **Error** (`src/websocket/error.rs`): Custom error types

### Message Protocol

Messages are JSON with a type field:

```json
// Join the chat
{"type": "Join", "data": {"username": "alice"}}

// Send a message
{"type": "Send", "data": {"content": "Hello!"}}

// Leave the chat
{"type": "Leave"}

// Broadcast (server to clients)
{"type": "Broadcast", "data": {"username": "alice", "content": "Hello!"}}

// Error message
{"type": "Error", "data": {"message": "Username already exists"}}
```

## Features

- ✅ Asynchronous I/O with Tokio
- ✅ WebSocket communication
- ✅ Single chat room
- ✅ Unique username enforcement
- ✅ Non-blocking concurrent connections
- ✅ Unit and integration tests
- ✅ Code formatting (rustfmt)
- ✅ Clippy linting without errors

## Notes

- Usernames must be unique - duplicate usernames will be rejected
- Messages are only sent to other users (not echoed back to sender)
- Server automatically cleans up when clients disconnect
- All code is non-blocking for maximum concurrency
