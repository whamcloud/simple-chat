package main

import (
	"os"

	"github.com/arunksaha/simple-chat/internal/client"
)

// parseConfig applies flags/env with validation so main() can stay small.
func parseConfig() (client.Config, error) {
	return client.ParseCLIConfig(os.Args[1:], os.Getenv)
}
