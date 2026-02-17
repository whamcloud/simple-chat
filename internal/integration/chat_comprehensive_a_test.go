package integration

import (
	"context"
	"io"
	"testing"
	"time"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

func TestComprehensiveThreeUserChatHappyPath(t *testing.T) {
	cfg := server.DefaultConfig()
	cfg.SessionOutQ = 16
	cfg.InboundQ = 128
	cfg.MaxMsgsPerSec = 50

	ts := StartTestServer(t, cfg)
	defer ts.Stop()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	conn, err := grpc.NewClient("passthrough:///bufnet",
		grpc.WithContextDialer(ts.Dialer),
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		t.Fatalf("dial: %v", err)
	}
	defer conn.Close()

	cli := pb.NewChatServiceClient(conn)

	alice := mustJoin(t, ctx, cli, "alice")
	bob := mustJoin(t, ctx, cli, "bob")
	charlie := mustJoin(t, ctx, cli, "charlie")

	seq1 := sendAndExpectFanout(t, alice, "alice", "hello from alice", bob, charlie)
	seq2 := sendAndExpectFanout(t, bob, "bob", "hello from bob", alice, charlie)
	seq3 := sendAndExpectFanout(t, charlie, "charlie", "hello from charlie", alice, bob)

	if seq1 >= seq2 || seq2 >= seq3 {
		t.Fatalf("expected strictly increasing sequence ids, got %d, %d, %d", seq1, seq2, seq3)
	}

	// Exercise explicit Leave path for one user.
	if err := bob.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Leave{Leave: &pb.Leave{}}}); err != nil {
		t.Fatalf("bob leave: %v", err)
	}
	if _, err := bob.Recv(); err != io.EOF {
		t.Fatalf("expected bob stream EOF after leave, got: %v", err)
	}

	// Remaining users should still exchange messages normally.
	_ = sendAndExpectFanout(t, alice, "alice", "after-bob-left", charlie)
}

func mustJoin(t *testing.T, ctx context.Context, cli pb.ChatServiceClient, user string) pb.ChatService_ChatClient {
	t.Helper()

	stream, err := cli.Chat(ctx)
	if err != nil {
		t.Fatalf("chat %s: %v", user, err)
	}
	if err := stream.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Join{Join: &pb.Join{UserId: user}}}); err != nil {
		t.Fatalf("join %s: %v", user, err)
	}
	ev, err := stream.Recv()
	if err != nil {
		t.Fatalf("recv joined %s: %v", user, err)
	}
	j := ev.GetJoined()
	if j == nil || j.UserId != user {
		t.Fatalf("unexpected join ack for %s: %#v", user, ev)
	}
	return stream
}

func sendAndExpectFanout(
	t *testing.T,
	sender pb.ChatService_ChatClient,
	fromUser, text string,
	recipients ...pb.ChatService_ChatClient,
) uint64 {
	t.Helper()

	if err := sender.Send(&pb.ClientEvent{Kind: &pb.ClientEvent_Send{Send: &pb.Send{Text: text}}}); err != nil {
		t.Fatalf("send from %s: %v", fromUser, err)
	}

	var seq uint64
	for i, r := range recipients {
		ev, err := r.Recv()
		if err != nil {
			t.Fatalf("recv fanout for %s on recipient #%d: %v", fromUser, i+1, err)
		}
		m := ev.GetMessage()
		if m == nil {
			t.Fatalf("expected message event from %s, got: %#v", fromUser, ev)
		}
		if m.FromUserId != fromUser || m.Text != text {
			t.Fatalf("unexpected message payload: got from=%q text=%q want from=%q text=%q",
				m.FromUserId, m.Text, fromUser, text)
		}
		if i == 0 {
			seq = m.Seq
			continue
		}
		if m.Seq != seq {
			t.Fatalf("expected same seq across fanout, got %d and %d", seq, m.Seq)
		}
	}
	return seq
}
