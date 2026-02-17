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

func TestLeaveRemovesSessionAndTerminatesStream(t *testing.T) {
	cfg := server.DefaultConfig()
	cfg.InboundQ = 64
	cfg.SessionOutQ = 8

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

	alice, err := c.Chat(ctx)
	if err != nil {
		t.Fatalf("chat alice: %v", err)
	}
	bob, err := c.Chat(ctx)
	if err != nil {
		t.Fatalf("chat bob: %v", err)
	}

	if err := alice.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: "alice"}}}); err != nil {
		t.Fatalf("join alice: %v", err)
	}
	if _, err := alice.Recv(); err != nil {
		t.Fatalf("recv joined alice: %v", err)
	}

	if err := bob.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: "bob"}}}); err != nil {
		t.Fatalf("join bob: %v", err)
	}
	if _, err := bob.Recv(); err != nil {
		t.Fatalf("recv joined bob: %v", err)
	}

	if err := bob.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Leave{Leave: &pb.Leave{}}}); err != nil {
		t.Fatalf("leave bob: %v", err)
	}

	recvErrCh := make(chan error, 1)
	go func() {
		_, err := bob.Recv()
		recvErrCh <- err
	}()

	select {
	case err := <-recvErrCh:
		if err == nil {
			t.Fatal("expected bob stream to terminate after Leave")
		}
	case <-time.After(300 * time.Millisecond):
		t.Fatal("expected bob stream to terminate after Leave")
	}

	if err := alice.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Send{Send: &pb.Send{Text: "after-leave"}}}); err != nil {
		t.Fatalf("alice send after leave: %v", err)
	}

	// Bob's stream should remain terminated; no post-leave broadcasts can arrive.
	recvErrCh2 := make(chan error, 1)
	go func() {
		_, err := bob.Recv()
		recvErrCh2 <- err
	}()
	select {
	case err := <-recvErrCh2:
		if err == nil {
			t.Fatal("expected bob stream to remain terminated")
		}
	case <-time.After(300 * time.Millisecond):
		t.Fatal("expected bob stream to remain terminated")
	}
}
