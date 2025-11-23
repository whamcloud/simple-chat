# Usage Guide

## Building

```bash
cargo build --release
```

## Running the Server

The server can be run with default settings (127.0.0.1:8080) or with custom host/port via environment variables:

```bash
# Default (127.0.0.1:8080)
cargo run --bin server

# Custom host and port
HOST=0.0.0.0 PORT=9000 cargo run --bin server
```

## Running the Client

The client requires a username and can connect to a server using command-line arguments or environment variables:

```bash
# Using command-line arguments
cargo run --bin client -- --username alice --host 127.0.0.1 --port 8080

# Using environment variables
USERNAME=alice HOST=127.0.0.1 PORT=8080 cargo run --bin client

# Mix of both (CLI args take precedence)
USERNAME=alice cargo run --bin client -- --host 127.0.0.1
```

## Client Commands

Once connected, the client supports the following commands:

- `send <MSG>` - Send a message to the chat room
- `leave` - Disconnect from the server and exit

Example:
```
> send Hello, everyone!
> send How are you?
> leave
```

## Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_test
cargo test --test ci_integration_test
```

## Code Quality Checks

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy --all-targets -- -D warnings

# Check compilation
cargo check --all-targets
```

## Setting Up Pre-commit Hook

To install the pre-commit hook that automatically checks formatting, compilation, and clippy:

```bash
# On Unix-like systems (Linux, macOS, WSL)
chmod +x setup-hooks.sh
./setup-hooks.sh

# Or manually
cp hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Example Session

Terminal 1 (Server):
```bash
$ cargo run --bin server
Server listening on 127.0.0.1:8080
```

Terminal 2 (Client 1):
```bash
$ cargo run --bin client -- --username alice
Connecting to 127.0.0.1:8080...
You joined the chat
> send Hello!
alice: Hello!
> 
```

Terminal 3 (Client 2):
```bash
$ cargo run --bin client -- --username bob
Connecting to 127.0.0.1:8080...
[System] alice joined the chat
You joined the chat
> alice: Hello!
> send Hi alice!
bob: Hi alice!
> leave
Leaving chat...
```

