# Architecture Notes — simple-chat

## Goal
A single-room chat system with:
- Unique usernames
- Join / Send / Leave
- Server broadcasts each message to all connected users except the sender
- Cleanup on Leave and disconnect
- Fully non-blocking server hot path (bounded queues; drop when full)
- CLI client that supports interactive `send <MSG>` and `leave`

Non-goals:
- Multi-room support
- Horizontal scaling across multiple server instances
- Message durability / replay / ordering guarantees across reconnects

---

## High-level shape

**Client ⇄ Server** uses a single **gRPC bidirectional streaming RPC** per connected user.

- Client sends **ClientEvent** frames: `Join`, `Send`, `Leave`
- Server sends **ServerEvent** frames: `Joined`, `Message`, `Error`

One stream = one user session (after successful Join).

---

## Key architectural decisions

### 1) Bidirectional streaming (single RPC per client)
Benefits:
- Natural “session” boundary
- Server push delivery without polling
- Clean mapping: 1 stream ↔ 1 session

### 2) Bounded queues + drop instead of backpressure
To keep the server non-blocking:
- **Inbound queue** (global): bounded channel
- **Outbound queue** (per session): bounded channel

When a queue is full:
- drop silently (but increment drop metrics/counters)

This avoids:
- blocking producers
- unbounded memory growth under load

Trade-off:
- Under load, some messages may be dropped (explicitly accepted by design)

### 3) Single broadcaster loop for fanout
The server uses a single broadcaster goroutine that:
- pulls messages from the inbound queue
- fans them out to all connected sessions

This keeps concurrency manageable and makes correctness simpler.
Scaling ceiling is O(N users) work per message.

### 4) Rate limiting per sender (KISS fixed-window)
Per-session rate limiter prevents a hot sender from overwhelming the system.
Implementation is behind an interface so it can be swapped later.

---

## Dataflow overview

### Join
1. Client opens gRPC stream and sends `Join(userId)`
2. Server validates username + uniqueness
3. On success:
   - creates a session
   - registers it
   - starts send goroutine for that stream
   - sends `Joined` ack

### Send
1. Client sends `Send(text)`
2. Server validates message length + rate limits
3. Server attempts a non-blocking enqueue to global inbound queue:
   - success → broadcaster will deliver
   - full → drop + increment drop metric

### Broadcast delivery
1. Broadcaster dequeues inbound message
2. For each connected session except sender:
   - non-blocking enqueue to that session’s outbound queue
   - if full → drop + increment drop metric

### Leave / Disconnect
- Leave: client sends `Leave`, server terminates stream and unregisters session
- Disconnect: server sees Recv EOF/error or Send error, terminates and unregisters

---

## Concurrency model (goroutines)

Per connected session:
- **1 goroutine** for gRPC recv loop is effectively the handler goroutine (the Chat() call)
- **1 goroutine** for gRPC send loop (reads from outbound queue and calls stream.Send)

Global:
- **1 broadcaster goroutine** (reads inbound queue and fans out)

So total goroutines:
- O(#sessions) send goroutines
- O(#sessions) handler goroutines (gRPC-managed)
- +1 broadcaster

---

## Queues (channels) and load behavior

### Global inbound queue (bounded)
Purpose:
- Decouple receiving client sends from broadcasting fanout

Behavior when full:
- drop inbound message (silent) + `dropped_inbound_total++`

### Per-session outbound queue (bounded)
Purpose:
- Decouple broadcaster fanout from slow client streams

Behavior when full:
- drop outbound event (silent) + `dropped_outbound_total++`

---

## Observability (minimal baseline)
Telemetry is intentionally minimal for the take-home:
- Counters for drops and message counts
- Gauges for inbound depth and active sessions
- Structured logs for joins/leaves/errors

---

## Scaling notes
- Single broadcaster goroutine is the throughput ceiling
- Fanout is O(N users) per message
- Registry iteration can slow join/leave under heavy broadcast load (trade-off chosen for simplicity)
- Drop policies prevent memory blow-up under overload (preferred for non-blocking behavior)
