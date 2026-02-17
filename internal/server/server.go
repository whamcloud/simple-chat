package server

import (
	"context"
	"io"
	"strings"
	"time"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server/ratelimit"
	"github.com/arunksaha/simple-chat/internal/server/telemetry"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// Server owns chat stream handling and shared runtime components.
type Server struct {
	pb.UnimplementedChatServiceServer

	cfg Config
	reg *Registry
	br  *Broadcaster
	mtr telemetry.Metrics
	lg  telemetry.Logger
}

// NewServer wires registry + broadcaster so all sessions share fan-out state.
func NewServer(cfg Config, metrics telemetry.Metrics, logger telemetry.Logger) *Server {
	if metrics == nil {
		metrics = telemetry.NoopMetrics{}
	}
	if logger == nil {
		logger = telemetry.NoopLogger{}
	}
	reg := NewRegistry()
	br := NewBroadcaster(reg, cfg.InboundQ, metrics)
	return &Server{cfg: cfg, reg: reg, br: br, mtr: metrics, lg: logger}
}

// Broadcaster exposes the broadcaster loop for process-level lifecycle wiring.
func (s *Server) Broadcaster() *Broadcaster { return s.br }

// logInfo emits structured logs through the configured telemetry logger.
func (s *Server) logInfo(ctx context.Context, msg string, kv ...any) {
	s.lg.InfoContext(ctx, msg, kv...)
}

// Chat enforces join-first protocol, then runs recv/send loops for one client.
func (s *Server) Chat(stream pb.ChatService_ChatServer) error {
	ctx := stream.Context()
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	// Protocol contract: first client frame must be Join.
	first, err := stream.Recv()
	if err != nil {
		return err
	}
	join := first.GetJoin()
	if join == nil {
		_ = stream.Send(&pb.ServerEvent{Kind: &pb.ServerEvent_Error{
			Error: &pb.Error{Code: "PROTOCOL", Detail: "first event must be Join"},
		}})
		return status.Error(codes.InvalidArgument, "first event must be Join")
	}

	userID := strings.TrimSpace(join.UserId)
	// Validate early so invalid users do not consume registry/session resources.
	if err := ValidateUserID(userID, s.cfg.MaxUsernameLen); err != nil {
		_ = stream.Send(&pb.ServerEvent{Kind: &pb.ServerEvent_Error{
			Error: &pb.Error{Code: "INVALID_USERNAME", Detail: err.Error()},
		}})
		return status.Error(codes.InvalidArgument, "invalid username")
	}

	sess := NewSession(userID, s.cfg.SessionOutQ, ratelimit.NewFixedWindow(s.cfg.MaxMsgsPerSec))
	// Enforce one active session per user id.
	if ok := s.reg.Register(userID, sess); !ok {
		_ = stream.Send(&pb.ServerEvent{Kind: &pb.ServerEvent_Error{
			Error: &pb.Error{Code: "USERNAME_TAKEN", Detail: "username already in use"},
		}})
		return status.Error(codes.AlreadyExists, "username taken")
	}
	s.logInfo(ctx, "chat user joined", "user_id", userID)

	defer func() {
		// Centralized teardown keeps state/metrics consistent for all exit paths.
		s.reg.Unregister(userID)
		close(sess.out)
		s.logInfo(ctx, "chat user left", "user_id", userID)
	}()

	// Acknowledge successful join to the caller before normal chat traffic starts.
	_ = stream.Send(&pb.ServerEvent{Kind: &pb.ServerEvent_Joined{Joined: &pb.Joined{UserId: userID}}})

	// Run send and recv concurrently:
	// - recvLoop handles inbound client events
	// - sendLoop pushes outbound events to the stream
	// If either side exits, we cancel the shared context so the other side stops.
	sendErrCh := make(chan error, 1)
	go func() { sendErrCh <- s.sendLoop(ctx, sess, stream) }()
	recvErrCh := make(chan error, 1)
	go func() { recvErrCh <- s.recvLoop(ctx, sess, stream) }()

	var recvErr error
	select {
	case recvErr = <-recvErrCh:
		// Client side ended (Leave/EOF/error).
	case recvErr = <-sendErrCh:
		// Outbound stream is broken; terminate session promptly.
	}
	cancel()

	// Give sendLoop a short chance to observe ctx/session close and exit.
	// sendLoop itself terminates on ctx.Done(), sess.out close, or Send error.
	select {
	case <-sendErrCh:
	case <-recvErrCh:
	case <-time.After(250 * time.Millisecond):
	}
	return recvErr
}

// recvLoop consumes client events for a single chat session until termination.
//
// Responsibilities:
//   - Read frames from the client stream and enforce protocol semantics.
//   - Handle Leave as a graceful session end.
//   - Validate Send payloads, enforce per-session rate limits, and enqueue
//     accepted messages to the broadcaster path.
//   - Return protocol/runtime errors to the caller so Chat() can tear down.
//
// Return behavior:
// - nil on clean shutdown paths (client EOF or explicit Leave).
// - non-nil on stream receive failures other than EOF.
//
// Notes:
//   - Unknown client event types do not immediately terminate the stream; the
//     server responds with a protocol error event and continues receiving.
//   - Message enqueue is intentionally non-blocking to keep this loop responsive
//     under backpressure.
func (s *Server) recvLoop(ctx context.Context, sess *Session, stream pb.ChatService_ChatServer) error {
	_ = ctx
	for {
		ev, err := stream.Recv()
		if err != nil {
			// EOF means client closed cleanly; any other error is transport/runtime failure.
			if err == io.EOF {
				return nil
			}
			return err
		}

		if ev.GetLeave() != nil {
			// Explicit Leave terminates this session gracefully.
			return nil
		}
		if snd := ev.GetSend(); snd != nil {
			// Validate before enqueue so broadcaster only sees normalized valid payloads.
			if err := ValidateMessage(snd.Text, s.cfg.MaxMessageLen); err != nil {
				_ = stream.Send(&pb.ServerEvent{Kind: &pb.ServerEvent_Error{
					Error: &pb.Error{Code: "INVALID_MESSAGE", Detail: err.Error()},
				}})
				continue
			}

			if !sess.rl.Allow(time.Now()) {
				// Rate-limited messages are dropped to protect shared server capacity.
				s.br.IncRateLimitedDrop()
				continue
			}
			// Non-blocking enqueue keeps this recv path responsive under load.
			_ = s.br.TryEnqueueInbound(ctx, InboundMessage{fromUser: sess.userID, text: strings.TrimSpace(snd.Text)})
			continue
		}

		// Unknown client event types are protocol violations; report and continue.
		_ = stream.Send(&pb.ServerEvent{Kind: &pb.ServerEvent_Error{
			Error: &pb.Error{Code: "PROTOCOL", Detail: "unknown client event"},
		}})
	}
}

// sendLoop drains session outbound events and sends them on the gRPC stream.
//
// Responsibilities:
// - Wait for either session events or cancellation from the parent context.
// - Forward each queued ServerEvent to the client stream in order.
//
// Return behavior:
// - nil when sess.out is closed during normal teardown.
// - ctx.Err() when session/server context is canceled.
// - non-nil stream.Send error when transport delivery fails.
//
// This loop is run in a dedicated goroutine so outbound delivery latency does
// not block recvLoop() from processing client input.
type eventSender interface {
	Send(*pb.ServerEvent) error
}

func (s *Server) sendLoop(ctx context.Context, sess *Session, sender eventSender) error {
	for {
		select {
		case <-ctx.Done():
			// Parent context cancellation ends outbound delivery for this stream.
			return ctx.Err()
		case evt, ok := <-sess.out:
			// Receive next outbound event; ok=false means sess.out was closed during teardown.
			if !ok {
				// Session teardown closes out channel to stop sender goroutine.
				return nil
			}
			if err := sender.Send(evt); err != nil {
				// Transport send errors are terminal for this stream.
				s.lg.ErrorContext(ctx, "sendLoop: stream.Send failed", "user", sess.userID, "err", err)
				return err
			}
		}
	}
}
