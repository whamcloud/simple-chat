package integration

import (
	"context"
	"testing"
	"time"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// TestJoinSendBroadcast validates end-to-end chat semantics over gRPC:
// join handshake, cross-user broadcast, and no self-echo.
func TestJoinSendBroadcast(t *testing.T) {
	cfg := server.DefaultConfig()
	cfg.SessionOutQ = 8
	cfg.InboundQ = 64

	ts := StartTestServer(t, cfg)
	defer ts.Stop()

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	conn, err := grpc.NewClient("passthrough:///bufnet",
		grpc.WithContextDialer(ts.Dialer),
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dial: %v", err)
	}
	defer conn.Close()

	c := pb.NewChatServiceClient(conn)

	a, err := c.Chat(ctx)
	if err != nil {
		t.Fatalf("chat A: %v", err)
	}
	if err := a.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: "alice"}}}); err != nil {
		t.Fatalf("join A: %v", err)
	}
	if _, err := a.Recv(); err != nil {
		t.Fatalf("recv joined A: %v", err)
	}

	b, err := c.Chat(ctx)
	if err != nil {
		t.Fatalf("chat B: %v", err)
	}
	if err := b.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: "bob"}}}); err != nil {
		t.Fatalf("join B: %v", err)
	}
	if _, err := b.Recv(); err != nil {
		t.Fatalf("recv joined B: %v", err)
	}

	if err := a.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Send{Send: &pb.Send{Text: "hi"}}}); err != nil {
		t.Fatalf("send: %v", err)
	}

	got := make(chan *pb.ServerEvent, 1)
	go func() {
		ev, _ := b.Recv()
		got <- ev
	}()

	select {
	case ev := <-got:
		m := ev.GetMessage()
		if m == nil || m.FromUserId != "alice" || m.Text != "hi" {
			t.Fatalf("unexpected message: %#v", ev)
		}
	case <-ctx.Done():
		t.Fatalf("timeout waiting for bob message")
	}

	aRecv := make(chan error, 1)
	go func() {
		_, err := a.Recv()
		aRecv <- err
	}()
	select {
	case <-aRecv:
		t.Fatalf("alice unexpectedly received an event (should not receive own message)")
	case <-time.After(150 * time.Millisecond):
	}
}
