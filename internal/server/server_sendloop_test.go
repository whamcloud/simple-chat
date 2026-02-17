package server

import (
	"context"
	"errors"
	"fmt"
	"strings"
	"sync"
	"testing"

	pb "github.com/arunksaha/simple-chat/gen/chat/v1"
	"github.com/arunksaha/simple-chat/internal/server/ratelimit"
	"github.com/arunksaha/simple-chat/internal/server/telemetry"
)

type testLogger struct {
	mu   sync.Mutex
	errs []string
}

func (l *testLogger) InfoContext(context.Context, string, ...any) {}

func (l *testLogger) ErrorContext(_ context.Context, msg string, kv ...any) {
	l.mu.Lock()
	defer l.mu.Unlock()

	var b strings.Builder
	b.WriteString(msg)
	for i := 0; i+1 < len(kv); i += 2 {
		b.WriteString(" ")
		b.WriteString(fmt.Sprint(kv[i]))
		b.WriteString("=")
		b.WriteString(fmt.Sprint(kv[i+1]))
	}
	l.errs = append(l.errs, b.String())
}

func (l *testLogger) errorLogs() []string {
	l.mu.Lock()
	defer l.mu.Unlock()
	out := make([]string, len(l.errs))
	copy(out, l.errs)
	return out
}

type failSender struct {
	err error
}

func (f failSender) Send(*pb.ServerEvent) error { return f.err }

type okSender struct{}

func (okSender) Send(*pb.ServerEvent) error { return nil }

func TestSendLoopLogsOnSendError(t *testing.T) {
	lg := &testLogger{}
	srv := NewServer(DefaultConfig(), telemetry.NoopMetrics{}, lg)
	sess := NewSession("bob", 1, ratelimit.NewFixedWindow(1000))

	sess.out <- &pb.ServerEvent{
		Kind: &pb.ServerEvent_Message{
			Message: &pb.Message{FromUserId: "alice", Text: "hello", Seq: 1},
		},
	}

	err := srv.sendLoop(context.Background(), sess, failSender{err: errors.New("boom")})
	if err == nil {
		t.Fatal("expected sendLoop to return send error")
	}

	logs := lg.errorLogs()
	if len(logs) == 0 {
		t.Fatal("expected at least one error log entry")
	}

	found := false
	for _, line := range logs {
		if strings.Contains(line, "sendLoop: stream.Send failed") {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("expected sendLoop error log, got: %#v", logs)
	}
}

func TestSendLoopExitsOnContextDoneWithoutSendErrorLog(t *testing.T) {
	lg := &testLogger{}
	srv := NewServer(DefaultConfig(), telemetry.NoopMetrics{}, lg)
	sess := NewSession("bob", 1, ratelimit.NewFixedWindow(1000))

	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	err := srv.sendLoop(ctx, sess, okSender{})
	if !errors.Is(err, context.Canceled) {
		t.Fatalf("expected context canceled, got: %v", err)
	}

	logs := lg.errorLogs()
	for _, line := range logs {
		if strings.Contains(line, "sendLoop: stream.Send failed") {
			t.Fatalf("did not expect sendLoop send-error log on context cancellation, got: %#v", logs)
		}
	}
}
