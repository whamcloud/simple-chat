package client

import (
	"context"
	"io"
	"net"
	"strconv"
	"strings"
	"sync"
	"testing"
	"time"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server"
	"github.com/arunksaha/simple-chat/internal/server/telemetry"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type safeBuffer struct {
	mu sync.Mutex
	b  strings.Builder
}

func (w *safeBuffer) Write(p []byte) (int, error) {
	w.mu.Lock()
	defer w.mu.Unlock()
	return w.b.Write(p)
}

func (w *safeBuffer) String() string {
	w.mu.Lock()
	defer w.mu.Unlock()
	return w.b.String()
}

func TestRunSendReceiveAndLeave(t *testing.T) {
	addr, stop := startClientTestServer(t)
	defer stop()

	host, port := splitAddr(t, addr)
	cfg := Config{
		UserID:     "alice",
		ServerHost: host,
		ServerPort: port,
	}

	inR, inW := io.Pipe()
	var out, errOut safeBuffer

	runErrCh := make(chan error, 1)
	go func() {
		runErrCh <- Run(cfg, inR, &out, &errOut)
	}()

	waitForContains(t, &out, "commands: send <MSG> | leave", 2*time.Second)

	// Connect a peer user to verify alice's sent message is delivered.
	peerConn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		t.Fatalf("peer dial: %v", err)
	}
	defer peerConn.Close()

	peerClient := pb.NewChatServiceClient(peerConn)
	ctx, cancel := context.WithTimeout(context.Background(), 4*time.Second)
	defer cancel()

	bob, err := peerClient.Chat(ctx)
	if err != nil {
		t.Fatalf("peer chat: %v", err)
	}
	if err := bob.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: "bob"}}}); err != nil {
		t.Fatalf("peer join: %v", err)
	}
	if _, err := bob.Recv(); err != nil {
		t.Fatalf("peer recv joined: %v", err)
	}

	// Exercise unknown command handling and valid send path in Run().
	if _, err := inW.Write([]byte("noop\n")); err != nil {
		t.Fatalf("write noop: %v", err)
	}
	if _, err := inW.Write([]byte("send hello-from-alice\n")); err != nil {
		t.Fatalf("write send: %v", err)
	}

	ev, err := bob.Recv()
	if err != nil {
		t.Fatalf("peer recv fanout: %v", err)
	}
	msg := ev.GetMessage()
	if msg == nil || msg.FromUserId != "alice" || msg.Text != "hello-from-alice" {
		t.Fatalf("unexpected fanout message: %#v", ev)
	}

	// Exercise client receive path by sending a message from bob to alice.
	if err := bob.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Send{Send: &pb.Send{Text: "hello-from-bob"}}}); err != nil {
		t.Fatalf("peer send: %v", err)
	}
	waitForContains(t, &out, "[bob] hello-from-bob", 2*time.Second)

	// Exercise leave path and ensure Run exits cleanly.
	if _, err := inW.Write([]byte("leave\n")); err != nil {
		t.Fatalf("write leave: %v", err)
	}
	_ = inW.Close()

	select {
	case err := <-runErrCh:
		if err != nil {
			t.Fatalf("Run returned error: %v", err)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for Run to exit")
	}

	if !strings.Contains(errOut.String(), "unknown command. use: send <MSG> | leave") {
		t.Fatalf("expected unknown-command hint in stderr, got: %q", errOut.String())
	}
}

func startClientTestServer(t *testing.T) (addr string, stop func()) {
	t.Helper()

	cfg := server.DefaultConfig()
	cfg.SessionOutQ = 16
	cfg.InboundQ = 128
	cfg.MaxMsgsPerSec = 100

	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}

	grpcServer := grpc.NewServer()
	srv := server.NewServer(cfg, telemetry.NoopMetrics{}, telemetry.NoopLogger{})
	pb.RegisterChatServiceServer(grpcServer, srv)

	ctx, cancel := context.WithCancel(context.Background())
	go srv.Broadcaster().Run(ctx)
	go func() { _ = grpcServer.Serve(lis) }()

	return lis.Addr().String(), func() {
		cancel()
		grpcServer.Stop()
		_ = lis.Close()
	}
}

func splitAddr(t *testing.T, addr string) (host string, port int) {
	t.Helper()

	h, p, err := net.SplitHostPort(addr)
	if err != nil {
		t.Fatalf("split host/port: %v", err)
	}
	pi, err := strconv.Atoi(p)
	if err != nil {
		t.Fatalf("parse port: %v", err)
	}
	return h, pi
}

func waitForContains(t *testing.T, b *safeBuffer, want string, timeout time.Duration) {
	t.Helper()

	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if strings.Contains(b.String(), want) {
			return
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %q in output: %q", want, b.String())
}
