# Usage Guide

## Quick Start

### Terminal 1: Start Server

```bash
cargo run --bin server --release
```

Server starts on `127.0.0.1:8080` by default.

### Terminal 2: Connect as Alice

```bash
cargo run --bin client --release -- -u alice
```

### Terminal 3: Connect as Bob

```bash
cargo run --bin client --release -- -u bob
```

### Send Messages

In each client terminal:

```text
>> send Hello everyone!
>> send How are you?
```

### Exit

```text
>> leave
```

---

## Server

### Options

**Environment Variable:**

```bash
CHAT_HOST=0.0.0.0:8080 cargo run --bin server --release
```

**Format:** `host:port`  
**Default:** `127.0.0.1:8080`

### Examples

```bash
# Default (localhost:8080)
cargo run --bin server --release

# Custom port
CHAT_HOST=127.0.0.1:9999 cargo run --bin server --release

# Listen on all interfaces
CHAT_HOST=0.0.0.0:8080 cargo run --bin server --release
```

---

## Client

### CLI Options

```text
Usage: client [OPTIONS] --username <USERNAME>

Options:
  -H, --host <HOST>          Server host [env: CHAT_HOST=] [default: 127.0.0.1]
  -p, --port <PORT>          Server port [env: CHAT_PORT=] [default: 8080]
  -u, --username <USERNAME>  Username [env: CHAT_USERNAME=] [REQUIRED]
  -h, --help                 Print help
```

### Usage Examples

```bash
# Simplest (default server)
cargo run --bin client --release -- -u alice

# Custom server
cargo run --bin client --release -- -H 192.168.1.100 -p 9999 -u alice

# Using environment variables
export CHAT_HOST=127.0.0.1
export CHAT_PORT=8080
export CHAT_USERNAME=alice
cargo run --bin client --release

# Built binary
./target/release/client -u bob
```

### Commands

Once connected:

| Command | Description |
|---------|-------------|
| `send <message>` | Send message to all users (except yourself) |
| `leave` | Disconnect and exit |

### What You'll See

```text
Connecting to 127.0.0.1:8080...
Connected!
Joined chat as 'alice'

Commands:
  send <message>  - Send a message
  leave           - Leave the chat

>> JOINED bob
>> send Hello Bob!
>> MESSAGE bob: Hi Alice!
>> leave
Goodbye!
```

---

## Complete Example Session

### Server Output

```text
Chat server listening on 127.0.0.1:8080
New connection from 127.0.0.1:xxxxx
User 'alice' joined
New connection from 127.0.0.1:xxxxx
User 'bob' joined
User 'alice' left
```

### Alice's Terminal

```text
Connecting to 127.0.0.1:8080...
Connected!
Joined chat as 'alice'

>> JOINED bob
>> send Hello Bob!
>> MESSAGE bob: Hi Alice!
>> leave
Goodbye!
```

### Bob's Terminal

```text
Connecting to 127.0.0.1:8080...
Connected!
Joined chat as 'bob'

>> MESSAGE alice: Hello Bob!
>> send Hi Alice!
>> LEFT alice
>> leave
Goodbye!
```

---

## Using Built Binaries

### Build Release

```bash
cargo build --release --workspace
```

### Run

```bash
# Server
./target/release/server

# Client
./target/release/client -u alice
```

---

## Troubleshooting

### Address Already in Use

**Error:** `Address already in use (os error 98)`  
**Solution:** Server configured with SO_REUSEADDR - should restart
immediately. If not, wait 30 seconds or use different port.

### Connection Refused

**Error:** `Failed to connect to server`  
**Solution:** Ensure server is running on specified host:port.

### Username Taken

**Error:** `Failed to join: ERROR Username 'alice' is already taken`  
**Solution:** Choose a different username.

---

## Tips

💡 **Username required** - Client won't start without it  
💡 **Case sensitive** - "Alice" and "alice" are different  
💡 **No message history** - Only see messages while connected  
💡 **Ctrl+C works** - Clean shutdown on both server and client  
💡 **Test locally** - Default settings work for local testing
