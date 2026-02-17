# simple-chat (Go + gRPC bidi streaming)

This repo contains:
- `chat-server`: async single-room chat server over gRPC bidirectional streaming
- `chat-client`: async CLI client (`send <MSG>`, `leave`)
- unit + integration tests (integration uses bufconn)

## Quickstart

### 1) Generate gRPC code (required)
Protobuf source:
- `api/proto/chat/v1/chat.proto`

Generated Go code goes to:
- `gen/chat/v1`

Install generators:
```bash
make install
```

Generate:
```bash
make gen
```

### 2) Build / run
```bash
make build
./bin/chat-server
./bin/chat-client --user alice
```

In another terminal:
```bash
./bin/chat-client --user bob
send hello
```

### 3) Tests
```bash
make test
```
