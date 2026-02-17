package main

import (
	"context"
	"os"

	"github.com/arunksaha/simple-chat/internal/client"
	"github.com/arunksaha/simple-chat/internal/server/telemetry"
)

// main is a thin wrapper around config parsing and client runtime execution.
func main() {
	lg := telemetry.NewSimpleLogger()
	ctx := context.Background()

	cfg, err := parseConfig()
	if err != nil {
		lg.ErrorContext(ctx, "chat-client config error", "err", err)
		os.Exit(2)
	}
	if err := client.Run(cfg, os.Stdin, os.Stdout, os.Stderr); err != nil {
		lg.ErrorContext(ctx, "chat-client runtime error", "err", err)
		os.Exit(1)
	}
}
