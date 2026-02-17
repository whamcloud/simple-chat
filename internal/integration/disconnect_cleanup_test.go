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

func TestDisconnectCleanupKeepsRoomHealthy(t *testing.T) {
	cfg := server.DefaultConfig()
	cfg.InboundQ = 64
	cfg.SessionOutQ = 8

	ts := StartTestServer(t, cfg)
	defer ts.Stop()

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	connAlice, err := grpc.NewClient("passthrough:///bufnet",
		grpc.WithContextDialer(ts.Dialer),
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dial alice: %v", err)
	}
	defer connAlice.Close()

	connBob, err := grpc.NewClient("passthrough:///bufnet",
		grpc.WithContextDialer(ts.Dialer),
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dial bob: %v", err)
	}

	cA := pb.NewChatServiceClient(connAlice)
	cB := pb.NewChatServiceClient(connBob)

	streamA, err := cA.Chat(ctx)
	if err != nil {
		t.Fatalf("chat alice: %v", err)
	}
	if err := streamA.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: "alice"}}}); err != nil {
		t.Fatalf("join alice: %v", err)
	}
	if _, err := streamA.Recv(); err != nil {
		t.Fatalf("recv joined alice: %v", err)
	}

	streamB, err := cB.Chat(ctx)
	if err != nil {
		t.Fatalf("chat bob: %v", err)
	}
	if err := streamB.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: "bob"}}}); err != nil {
		t.Fatalf("join bob: %v", err)
	}
	if _, err := streamB.Recv(); err != nil {
		t.Fatalf("recv joined bob: %v", err)
	}

	bobErrCh := make(chan error, 1)
	go func() {
		_, err := streamB.Recv()
		bobErrCh <- err
	}()

	if err := connBob.Close(); err != nil {
		t.Fatalf("close bob conn: %v", err)
	}

	select {
	case err := <-bobErrCh:
		if err == nil {
			t.Fatal("expected bob recv error after disconnect")
		}
	case <-time.After(500 * time.Millisecond):
		t.Fatal("bob recv did not unblock after disconnect")
	}

	if err := streamA.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Send{Send: &pb.Send{Text: "hi-after-disconnect"}}}); err != nil {
		t.Fatalf("alice send #1 failed: %v", err)
	}
	if err := streamA.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Send{Send: &pb.Send{Text: "still-alive"}}}); err != nil {
		t.Fatalf("alice send #2 failed: %v", err)
	}
}
