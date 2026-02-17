package server

type Config struct {
	// ListenAddr is the TCP address the gRPC server binds to.
	// It determines where clients must connect (host/IP + port).
	ListenAddr string
	// InboundQ is the server-wide buffer size for incoming client events.
	// Larger values absorb short bursts; too small can increase dropped/backpressured events.
	InboundQ int
	// SessionOutQ is the per-session outbound buffer size for server->client events.
	// It controls tolerance for slow readers before they affect overall fan-out.
	SessionOutQ int
	// MaxMsgsPerSec is the per-client rate limit for sent messages.
	// It protects the service from spam/flooding and keeps usage fair across clients.
	MaxMsgsPerSec uint32
	// MaxUsernameLen is the maximum accepted username length in characters.
	// It bounds memory/log footprint and enforces a predictable identity format.
	MaxUsernameLen int
	// MaxMessageLen is the maximum accepted chat message length in characters.
	// It prevents oversized payloads from harming latency, memory usage, or UX.
	MaxMessageLen int
}

func DefaultConfig() Config {
	return Config{
		ListenAddr:     ":9786",
		InboundQ:       1024,
		SessionOutQ:    256,
		MaxMsgsPerSec:  20,
		MaxUsernameLen: 32,
		MaxMessageLen:  1024,
	}
}
