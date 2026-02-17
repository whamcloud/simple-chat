# Design Notes of simple-chat

Please read the [Architecture Notes](./Architecture-Notes.md) first.

This document describes the detailed design with emphasis on:
- goroutines (threads)
- channels (queues)
- data structures
- exact message flow and shutdown rules

Code snippets are illustrative and match the structure used in the repository.

---

## gRPC API (proto mental model)

### Client to Server: `ClientEvent`
- `Join { user_id }`
- `Send { text }`
- `Leave {}`

### Server to Client: `ServerEvent`
- `Joined { user_id }`
- `Message { from_user_id, text, seq }`
- `Error { code, detail }`

The server requires the **first** client event on a stream to be `Join`.
After Join succeeds, subsequent events may be `Send` or `Leave`.

---

## Core types and data structures

### Session
Represents a connected user after successful Join.

Fields (conceptual):
- `userID string`
- `out chan *pb.ServerEvent` (bounded outbound queue)
- `limiter RateLimiter` (per-sender)
- (optional internal state such as closed flags may exist, but correctness does not rely on them)

Key invariants:
- The broadcaster is the only writer to `sess.out`.
- The sendLoop goroutine for that session is the only reader from `sess.out`.

### Registry
Tracks active sessions.

Structure:
- `mu sync.RWMutex`
- `sessions map[string]*Session` (key: userID)

Operations:
- `Register(userID, sess) bool` (false if already exists)
- `Unregister(userID)`
- `Count() int`
- `Range(fn func(user string, sess *Session))`

The broadcaster uses `Range()` to iterate over currently registered sessions.

### InboundMessage
Represents a message accepted from a client and queued for broadcast fanout.

Fields:
- `fromUser string`
- `text string`

The design uses `fromUser` (userID) as the sender identity for fanout decisions (e.g., â€œdo not echo to senderâ€).

### Broadcaster
Owns the global inbound queue and performs fanout.

Fields (conceptual):
- `inbound chan InboundMessage` (bounded)
- `reg *Registry`
- `seq uint64` (atomic; monotonic sequence for outbound Message events)
- `metrics/telemetry`

---

## Channels (queues)

### Global inbound queue
- Name: `b.inbound`
- Type: `chan InboundMessage`
- Capacity: `InboundQ` (e.g., 1024)
- Writers: all session recv loops (Chat handlers)
- Reader: exactly one broadcaster goroutine

Enqueue is non-blocking; if the queue is full, the message is dropped:

```go
select {
case b.inbound <- msg:
    // accepted
default:
    // drop + metric
}
```

### Per-session outbound queue
- Name: `sess.out`
- Type: `chan *pb.ServerEvent`
- Capacity: `SessionOutQ` (e.g., 256)
- Writer: broadcaster goroutine
- Reader: sendLoop goroutine for that session

Enqueue is non-blocking; if the queue is full, the event is dropped:

```go
select {
case sess.out <- evt:
default:
    // drop + metric
}
```

---

## Goroutines (crystal clear mapping)

### 1) gRPC handler goroutine (per session)
This is the goroutine executing `Server.Chat(stream)` (managed by gRPC, one per stream).

Responsibilities:
- Read the first client event; enforce that it is `Join`
- Validate username and enforce uniqueness via the registry
- Construct a `Session` and register it
- Start the session sendLoop goroutine
- Run recvLoop to process subsequent client events until termination
- Ensure cleanup via deferred actions (unregister, close outbound queue, cancel context)

### 2) sendLoop goroutine (per session)
Dedicated goroutine responsible for outbound delivery on that stream:

- Reads `*pb.ServerEvent` from `sess.out`
- Calls `stream.Send(evt)` for each event

Termination conditions:
- `sess.out` is closed (normal shutdown path)
- context is canceled (`ctx.Done()`)
- `stream.Send` returns an error (e.g., client disconnected or network failure)

The Chat handler terminates the session when either recvLoop finishes or sendLoop fails; cancellation is used to stop the other side promptly.

### 3) broadcaster goroutine (single global)
Dedicated goroutine started at server startup.

Responsibilities:
- Read one inbound message at a time from `b.inbound`
- Increment a global monotonic sequence number `seq`
- Build a `ServerEvent_Message` containing `{from_user_id, text, seq}`
- Iterate current sessions via `reg.Range()` and enqueue the event to each sessionâ€™s outbound queue (except the sender)
- Periodically export gauges:
  - `chat_inbound_depth` = `len(b.inbound)`
  - `chat_active_sessions` = `reg.Count()`

---

## End-to-end flows (step-by-step)

### Join flow
1. Client opens stream `Chat()`.
2. Client sends `Join(userID)`.
3. Server reads first event in recvLoop:
   - validate username syntax/length
   - `reg.Register(userID, sess)` must succeed (enforces uniqueness)
4. On success:
   - create `Session{ userID, out: make(chan..., SessionOutQ), limiter: ... }`
   - start sendLoop goroutine
   - send `Joined` ack event to the client
5. Client blocks in `WaitForJoinAck` until it sees `Joined` or an `Error`.

If Join fails (e.g., username taken), the server sends an `Error` and terminates the stream; the client exits immediately on a pre-join error.

### Send flow (non-blocking)
1. Client sends `Send(text)`.
2. Server recvLoop:
   - validate message length
   - apply per-session fixed-window rate limit
   - attempt non-blocking enqueue into `b.inbound`
   - if inbound queue is full: drop + increment `dropped_inbound_total`

### Broadcast (fanout) flow
1. Broadcaster receives `InboundMessage`.
2. `seq` increments atomically.
3. Broadcaster builds:

```go
evt := &pb.ServerEvent{
  Kind: &pb.ServerEvent_Message{
    Message: &pb.Message{
      FromUserId: msg.fromUser,
      Text: msg.text,
      Seq: seq,
    },
  },
}
```

4. Broadcaster iterates `reg.Range()`:
   - skip `user == msg.fromUser` (no self-echo)
   - non-blocking enqueue to `sess.out`
   - if session outbound queue is full: drop + increment `dropped_outbound_total`

### Leave flow
1. Client sends `Leave{}`.
2. Server recvLoop returns nil to indicate clean exit.
3. Chat handler triggers cleanup:
   - unregister session from registry
   - close `sess.out` (causes sendLoop to exit after draining/observing closure)
   - cancel context to stop any remaining work
4. Stream ends.

### Disconnect flow
Two common cases:
- Client disconnects while receiving: `stream.Recv()` returns EOF/error in recvLoop.
- Client disconnects while server is sending: `stream.Send()` fails in sendLoop.

In either case:
- Chat handler cancels the session context
- unregisters the user
- closes `sess.out`
- returns from `Chat()`

---

## Shutdown and cancellation mechanics (exact behavior)

The Chat handler derives a cancellable context from `stream.Context()` and runs:
- recvLoop (in the handler goroutine)
- sendLoop (in a dedicated goroutine)

The handler waits for the first of the two to complete:
- If recvLoop completes (Leave/EOF/error), the handler cancels the context so sendLoop exits quickly.
- If sendLoop errors (Send failure), the handler cancels the context so recvLoop exits quickly.

This prevents a half-dead session from lingering (e.g., outbound path broken but recv still running).

---

## Drop policy details (what is dropped and why)

### Inbound drop (`b.inbound` full)
Purpose:
- Bound memory usage for pending fanout work.
- Prevent a burst of incoming `Send` events (from one or many clients) from accumulating unbounded backlog when broadcaster throughput is momentarily lower than inbound rate.

Effect:
- The senderâ€™s message is not broadcast.
- The server remains responsive (no blocking).
- Metric/counter increments: `dropped_inbound_total`.

Note: This is separate from the rate limiter. The limiter caps per-sender send rate; inbound drop is a global â€œqueue is saturatedâ€ signal.

### Outbound drop (`sess.out` full)
Purpose:
- Prevent one slow receiver (or many slow receivers) from blocking fanout.
- Bound memory usage per session.

Effect:
- That receiver misses some messages.
- Other receivers continue to receive (non-blocking fanout).
- Metric/counter increments: `dropped_outbound_total`.

---

## Rate limiter interface (KISS fixed-window)

Interface:

```go
type RateLimiter interface {
  Allow(now time.Time) bool
}
```

Implementation: fixed window (1 second)
- Allow up to `MaxMsgsPerSec` messages per second per session.
- If exceeded: message is dropped and `dropped_rate_limited_total` increments.

Rate limiting is per-sender and independent from queue saturation drops.

---

## Testing strategy (brief)

Unit tests:
- username/message validation
- rate limiter correctness
- client config and command parsing
- sendLoop behavior

Integration tests (bufconn):
- join/send/broadcast correctness
- no self echo
- leave cleanup
- disconnect cleanup
- monotonic `seq` ordering in a multi-user scenario

CI smoke test:
- starts server + three clients
- verifies bobâ†’alice delivery, aliceâ†’bob delivery, and charlie receives both
- writes artifacts under `smoke_test_output/` for debugging
