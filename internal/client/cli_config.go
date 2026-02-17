// Package client contains runtime configuration and chat client logic.
package client

import (
	"flag"
	"fmt"
	"io"
	"strconv"
)

// ParseCLIConfig builds client runtime config from defaults, environment,
// and CLI flags.
//
// Precedence:
//  1. CLI flags
//  2. Environment variables
//  3. Built-in defaults
//
// Supported environment variables:
// - CHAT_USER
// - CHAT_SERVER_HOST
// - CHAT_SERVER_PORT
func ParseCLIConfig(args []string, getenv func(string) string) (Config, error) {
	// Start from stable defaults so omitted inputs remain predictable.
	cfg := DefaultConfig()

	// Apply environment values first; flags can override later.
	if v := getenv("CHAT_USER"); v != "" {
		cfg.UserID = v
	}
	if v := getenv("CHAT_SERVER_HOST"); v != "" {
		cfg.ServerHost = v
	}
	if v := getenv("CHAT_SERVER_PORT"); v != "" {
		p, err := strconv.Atoi(v)
		if err != nil {
			// Return the original value to make troubleshooting obvious.
			return cfg, fmt.Errorf("invalid CHAT_SERVER_PORT: %q", v)
		}
		cfg.ServerPort = p
	}

	// Bind flags using env-adjusted cfg as defaults so flag precedence is natural.
	fs := flag.NewFlagSet("chat-client", flag.ContinueOnError)
	// Tests assert error values directly; suppress usage output noise.
	fs.SetOutput(io.Discard)
	fs.StringVar(&cfg.UserID, "user", cfg.UserID, "user id (required)")
	fs.StringVar(&cfg.ServerHost, "server-host", cfg.ServerHost, "server host")
	fs.IntVar(&cfg.ServerPort, "server-port", cfg.ServerPort, "server port")

	if err := fs.Parse(args); err != nil {
		return cfg, err
	}
	// Username is mandatory even when host/port are present.
	if cfg.UserID == "" {
		return cfg, fmt.Errorf("--user is required (or set CHAT_USER)")
	}
	return cfg, nil
}
