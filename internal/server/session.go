package server

import (
	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server/ratelimit"
)

// Session holds per-connected-user state needed by the server runtime.
type Session struct {
	// userID is the stable identifier associated with this connection.
	userID string
	// out is the buffered outbound event queue consumed by the stream writer.
	out chan *pb.ServerEvent
	// rl enforces per-session message send limits.
	rl ratelimit.RateLimiter
}

// NewSession creates a session with its own outbound queue and rate limiter.
func NewSession(userID string, outQ int, rl ratelimit.RateLimiter) *Session {
	return &Session{
		userID: userID,
		out:    make(chan *pb.ServerEvent, outQ),
		rl:     rl,
	}
}

// UserID returns the session's associated user identifier.
func (s *Session) UserID() string { return s.userID }
