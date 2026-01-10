# Development Guide

## Project Structure

```
chatserver/
├── src/
│   ├── lib.rs              # Shared message types and tests
│   └── bin/
│       ├── server.rs       # Server binary
│       └── client.rs       # Client binary
├── tests/
│   └── integration_tests.rs # Integration tests
├── Cargo.toml              # Project manifest
├── rustfmt.toml            # Formatting configuration
├── .clippy.toml            # Clippy configuration
├── .git/hooks/pre-commit   # Pre-commit hook
├── .github/workflows/ci.yml # GitHub Actions workflow
└── README.md               # User documentation
```

## Architecture

### Message Protocol

The application uses a JSON-based message protocol with line-delimited messages:

```
Client -> Server: {"action":"join","username":"alice"}
Server -> Others: {"type":"user_joined","data":{"username":"alice"}}
Client -> Server: {"action":"send","content":"hello"}
Server -> Others: {"type":"chat","data":{"username":"alice","content":"hello"}}
Client -> Server: {"action":"leave"}
Server -> Others: {"type":"user_left","data":{"username":"alice"}}
```

### Concurrency Model

- **Server**: Uses Tokio async runtime with one task per client connection
- **Channels**: UnboundedSender for message passing to clients
- **Synchronization**: RwLock for the client map (allows concurrent reads)
- **Non-blocking**: All I/O is async, no blocking operations

### Key Components

#### Server (`src/bin/server.rs`)

- Accepts TCP connections on `127.0.0.1:8080`
- Expects first message to be a `Join` message
- Maintains a HashMap of active users (username -> sender)
- Broadcasts messages to all users except sender
- Cleans up on disconnect or `Leave` message

#### Client (`src/bin/client.rs`)

- Connects to server with configurable host/port
- Sends initial `Join` message with username
- Spawns async task to receive server messages
- Uses stdin to read user commands in main task
- Supports environment variables for configuration

#### Library (`src/lib.rs`)

- Defines `ClientMessage` enum (Join, Send, Leave)
- Defines `ServerMessage` enum (Chat, UserJoined, UserLeft, Error)
- Implements serialization/deserialization with JSON
- Includes unit tests for message types

## Development Workflow

### Setup

1. Clone the repository
2. Ensure Rust and Cargo are installed
3. Navigate to project directory

### Building

```bash
cargo build               # Debug build
cargo build --release    # Release build (optimized)
```

### Testing

```bash
cargo test               # Run all tests
cargo test --lib        # Unit tests only
cargo test --test '*'   # Integration tests only
cargo test -- --nocapture # Show println! output
```

### Code Quality

```bash
cargo fmt               # Format code
cargo fmt -- --check   # Check formatting
cargo clippy --all-targets --all-features -- -D warnings
```

### Running

Terminal 1:

```bash
cargo run --bin server
```

Terminal 2:

```bash
cargo run --bin client
```

Or with arguments:

```bash
cargo run --bin client -- --host 127.0.0.1 --port 8080 --username alice
```

## Pre-commit Hook

The pre-commit hook automatically:

1. Checks code formatting
2. Runs tests
3. Runs clippy

To enable:

```bash
chmod +x .git/hooks/pre-commit
```

## Adding New Features

### Example: Adding a Kick Command

1. **Update message types** in `src/lib.rs`:

```rust
pub enum ClientMessage {
    // ... existing variants
    #[serde(rename = "kick")]
    Kick { username: String },
}
```

2. **Update server** in `src/bin/server.rs`:

```rust
ClientMessage::Kick { username } => {
    // Handle kick logic
}
```

3. **Add tests** in `src/lib.rs`:

```rust
#[test]
fn test_client_message_kick() {
    let msg = ClientMessage::Kick {
        username: "target".to_string(),
    };
    // Test serialization
}
```

4. **Update client** in `src/bin/client.rs`:

```rust
} else if input.starts_with("kick ") {
    let target = input[5..].to_string();
    let kick_msg = ClientMessage::Kick { username: target };
    // Send message
}
```

5. **Run tests and clippy**:

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt
```

## Performance Considerations

### Memory Usage

- Server holds one `UnboundedSender` per connected client (~200 bytes)
- Username string (variable)
- Total per user: ~300-500 bytes depending on username length

### Throughput

- Can handle thousands of concurrent connections
- Message broadcast is O(n) where n = number of clients
- Uses RwLock for efficient concurrent reads
