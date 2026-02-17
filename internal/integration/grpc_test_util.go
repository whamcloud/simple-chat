package integration

import (
	"context"
	"net"
	"testing"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server"
	"github.com/arunksaha/simple-chat/internal/server/telemetry"
	"google.golang.org/grpc"
	"google.golang.org/grpc/test/bufconn"
)

// bufSize keeps in-memory transport large enough for test traffic bursts.
const bufSize = 1024 * 1024

// TestServer bundles server runtime and bufconn resources for integration tests.
type TestServer struct {
	Lis       *bufconn.Listener
	GRPC      *grpc.Server
	Srv       *server.Server
	CtxCancel func()
}

// StartTestServer boots a real gRPC server on an in-memory listener.
func StartTestServer(t *testing.T, cfg server.Config) *TestServer {
	t.Helper()

	lis := bufconn.Listen(bufSize)
	grpcServer := grpc.NewServer()

	srv := server.NewServer(cfg, telemetry.NoopMetrics{}, telemetry.NoopLogger{})
	pb.RegisterChatServiceServer(grpcServer, srv)

	ctx, cancel := context.WithCancel(context.Background())
	go srv.Broadcaster().Run(ctx)
	go func() { _ = grpcServer.Serve(lis) }()

	return &TestServer{Lis: lis, GRPC: grpcServer, Srv: srv, CtxCancel: cancel}
}

// Dialer is passed to grpc.WithContextDialer for bufconn client dials.
func (ts *TestServer) Dialer(context.Context, string) (net.Conn, error) {
	return ts.Lis.Dial()
}

// Stop tears down broadcaster, gRPC server, and listener resources.
func (ts *TestServer) Stop() {
	ts.CtxCancel()
	ts.GRPC.Stop()
	_ = ts.Lis.Close()
}
