package ratelimit

import (
	"sync"
	"time"
)

// FixedWindow allows up to maxPerSec events per whole-second bucket.
type FixedWindow struct {
	// maxPerSec is the allowed number of events per one-second window.
	maxPerSec uint32

	// mu serializes updates so Allow is safe for concurrent callers.
	mu sync.Mutex
	// sec is the Unix second for the currently tracked window.
	sec int64
	// count is how many events have been admitted in the current window.
	count uint32
}

// NewFixedWindow normalizes zero into a minimal non-zero limit.
func NewFixedWindow(maxPerSec uint32) *FixedWindow {
	if maxPerSec == 0 {
		maxPerSec = 1
	}
	return &FixedWindow{maxPerSec: maxPerSec}
}

// Allow returns true while the current second's quota is not exhausted.
func (f *FixedWindow) Allow(now time.Time) bool {
	s := now.Unix()
	f.mu.Lock()
	defer f.mu.Unlock()

	if f.sec != s {
		f.sec = s
		f.count = 0
	}
	if f.count >= f.maxPerSec {
		return false
	}
	f.count++
	return true
}
