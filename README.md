# simple-chat

A single-room chat server and CLI client. The server broadcasts each message to everyone except the sender. Usernames must be unique.

## Build

```bash
cargo build
```

## Run

Start the server:

```bash
cargo run -p server
```

By default it listens on `0.0.0.0:3000`. Override with flags or env vars:

```bash
cargo run -p server -- --host 127.0.0.1 --port 4000
# or
CHAT_HOST=127.0.0.1 CHAT_PORT=4000 cargo run -p server
```

Connect a client (username is required):

```bash
cargo run -p client -- --username alice
# or
CHAT_USERNAME=alice cargo run -p client
```

The client defaults to `127.0.0.1:3000`. To point it elsewhere:

```bash
cargo run -p client -- --username alice --host 192.168.1.5 --port 4000
```

## Chat

Once connected, you'll see a `>` prompt.

| Input | What it does |
|---|---|
| `send hello everyone` | Sends a message to the room |
| `leave` | Disconnects and exits |
| `Ctrl+C` | Same as leave |

Join/leave notifications for other users are printed automatically as they happen. You won't see your own messages echoed back.

If the username you picked is already taken, the server will reject the connection immediately.

## Tests

```bash
cargo test
```
