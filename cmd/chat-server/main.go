package main

import (
	"context"
	"net"
	"os"
	"os/signal"
	"syscall"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server"
	"github.com/arunksaha/simple-chat/internal/server/telemetry"
	"google.golang.org/grpc"
)

func main() {
	lg := telemetry.NewSimpleLogger()
	metrics := telemetry.NoopMetrics{}
	ctx := context.Background()

	// Load runtime settings (listen address, limits, etc.).
	cfg := parseConfig()
	lg.InfoContext(ctx, "chat-server config", "listen_addr", cfg.ListenAddr, "inbound_q", cfg.InboundQ, "session_out_q", cfg.SessionOutQ, "max_msgs_per_sec", cfg.MaxMsgsPerSec, "max_username_len", cfg.MaxUsernameLen, "max_message_len", cfg.MaxMessageLen)

	// Open the TCP listener before constructing and serving gRPC.
	lis, err := net.Listen("tcp", cfg.ListenAddr)
	if err != nil {
		lg.ErrorContext(ctx, "chat-server listen failed", "err", err, "listen_addr", cfg.ListenAddr)
		os.Exit(1)
	}

	// Wire server dependencies.
	srv := server.NewServer(cfg, metrics, lg)

	grpcServer := grpc.NewServer()
	pb.RegisterChatServiceServer(grpcServer, srv)

	// Run the broadcast loop in the background until shutdown is requested.
	runCtx, cancel := context.WithCancel(context.Background())
	go srv.Broadcaster().Run(runCtx)

	// Trap SIGINT/SIGTERM so the process can shut down gracefully instead of
	// exiting abruptly: we stop the broadcaster loop and ask gRPC to drain
	// in-flight RPCs before closing listeners.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		cancel()
		grpcServer.GracefulStop()
	}()

	lg.InfoContext(ctx, "chat-server listening", "listen_addr", cfg.ListenAddr)
	err = grpcServer.Serve(lis)
	lg.InfoContext(ctx, "chat-server final metrics",
		"dropped_inbound", srv.Broadcaster().DroppedInbound(),
		"dropped_outbound", srv.Broadcaster().DroppedOutbound(),
		"dropped_rate_limited", srv.Broadcaster().DroppedRateLimited(),
		"messages_received", srv.Broadcaster().MessagesReceived(),
		"messages_transmitted", srv.Broadcaster().MessagesTransmitted(),
	)
	if err != nil {
		lg.ErrorContext(ctx, "chat-server serve failed", "err", err)
		os.Exit(1)
	}
}
