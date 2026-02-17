package server

import (
	"context"
	"sync/atomic"
	"time"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server/telemetry"
)

// InboundMessage is the normalized message payload accepted by broadcaster.
type InboundMessage struct {
	fromUser string
	text     string
}

// Broadcaster fans inbound chat messages out to active sessions.
type Broadcaster struct {
	// reg is consulted on each broadcast to find current recipients.
	reg *Registry
	// inbound buffers accepted messages so recv loops do not block on fan-out.
	inbound chan InboundMessage
	// seq assigns monotonic ids for message ordering/debugging.
	seq uint64

	metrics telemetry.Metrics

	droppedInbound      uint64
	droppedOutbound     uint64
	droppedRateLimited  uint64
	messagesReceived    uint64
	messagesTransmitted uint64
}

// NewBroadcaster creates a broadcaster with bounded inbound buffering.
func NewBroadcaster(reg *Registry, inboundQ int, metrics telemetry.Metrics) *Broadcaster {
	if metrics == nil {
		metrics = telemetry.NoopMetrics{}
	}
	return &Broadcaster{
		reg:     reg,
		inbound: make(chan InboundMessage, inboundQ),
		metrics: metrics,
	}
}

// InboundChan exposes a send-only channel for upstream producers.
func (b *Broadcaster) InboundChan() chan<- InboundMessage { return b.inbound }

func (b *Broadcaster) DroppedInbound() uint64 {
	return atomic.LoadUint64(&b.droppedInbound)
}

func (b *Broadcaster) DroppedOutbound() uint64 {
	return atomic.LoadUint64(&b.droppedOutbound)
}

func (b *Broadcaster) DroppedRateLimited() uint64 {
	return atomic.LoadUint64(&b.droppedRateLimited)
}

func (b *Broadcaster) MessagesReceived() uint64 {
	return atomic.LoadUint64(&b.messagesReceived)
}

func (b *Broadcaster) MessagesTransmitted() uint64 {
	return atomic.LoadUint64(&b.messagesTransmitted)
}

// IncRateLimitedDrop records messages rejected by per-session rate limiting.
func (b *Broadcaster) IncRateLimitedDrop() {
	atomic.AddUint64(&b.droppedRateLimited, 1)
	b.metrics.IncCounter("chat_rate_limited_dropped_total", 1)
}

// TryEnqueueInbound performs non-blocking enqueue to preserve stream responsiveness.
func (b *Broadcaster) TryEnqueueInbound(_ context.Context, msg InboundMessage) bool {
	select {
	case b.inbound <- msg:
		atomic.AddUint64(&b.messagesReceived, 1)
		b.metrics.IncCounter("chat_messages_received_total", 1)
		return true
	default:
		atomic.AddUint64(&b.droppedInbound, 1)
		b.metrics.IncCounter("chat_inbound_dropped_total", 1)
		return false
	}
}

// Run processes inbound messages and periodically exports queue/session gauges.
func (b *Broadcaster) Run(ctx context.Context) {
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			b.metrics.SetGauge("chat_inbound_depth", int64(len(b.inbound)))
			// Active sessions is a gauge (point-in-time), not a counter.
			b.metrics.SetGauge("chat_active_sessions", int64(b.reg.Count()))
		case m := <-b.inbound:
			b.broadcastOne(m)
		}
	}
}

func (b *Broadcaster) broadcastOne(m InboundMessage) {
	// Assign a server-side sequence so receivers can order/debug message flow.
	seq := atomic.AddUint64(&b.seq, 1)
	evt := &pb.ServerEvent{
		Kind: &pb.ServerEvent_Message{
			Message: &pb.Message{
				FromUserId: m.fromUser,
				Text:       m.text,
				Seq:        seq,
			},
		},
	}

	// Iterate active sessions once and attempt best-effort fan-out.
	// Delivery is intentionally non-blocking: slow clients drop messages
	// instead of stalling broadcasts for other sessions.
	b.reg.Range(func(user string, sess *Session) {
		if user == m.fromUser {
			// Do not echo a sender's message back to the same session.
			return
		}
		select {
		case sess.out <- evt:
			atomic.AddUint64(&b.messagesTransmitted, 1)
			b.metrics.IncCounter("chat_messages_transmitted_total", 1)
		default:
			atomic.AddUint64(&b.droppedOutbound, 1)
			b.metrics.IncCounter("chat_outbound_dropped_total", 1)
		}
	})
}
