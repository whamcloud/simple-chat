package ratelimit

import (
	"testing"
	"time"
)

// Verifies quota enforcement within one second and reset on next second.
func TestFixedWindow(t *testing.T) {
	rl := NewFixedWindow(2)
	base := time.Unix(1000, 0)

	if !rl.Allow(base) {
		t.Fatal("expected allow #1")
	}
	if !rl.Allow(base) {
		t.Fatal("expected allow #2")
	}
	if rl.Allow(base) {
		t.Fatal("expected deny #3")
	}

	if !rl.Allow(base.Add(1 * time.Second)) {
		t.Fatal("expected allow after window reset")
	}
}
