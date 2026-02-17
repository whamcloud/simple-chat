package client

import "strconv"

// Config contains runtime settings for the chat CLI client.
type Config struct {
	// UserID is sent during Join and identifies this client in chat events.
	UserID string
	// ServerHost is the network host/IP of the chat server.
	ServerHost string
	// ServerPort is the chat server TCP port.
	ServerPort int
}

// DefaultConfig provides local-dev defaults so only user id must be supplied.
func DefaultConfig() Config {
	return Config{
		ServerHost: "localhost",
		ServerPort: 9786,
	}
}

// Addr returns the host:port target for grpc.Dial.
func (c Config) Addr() string {
	return c.ServerHost + ":" + strconv.Itoa(c.ServerPort)
}
