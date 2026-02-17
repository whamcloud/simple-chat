package main

import (
	"github.com/arunksaha/simple-chat/internal/server"
)

// parseConfig currently returns defaults; this hook keeps startup extensible.
func parseConfig() server.Config {
	cfg := server.DefaultConfig()
	return cfg
}
