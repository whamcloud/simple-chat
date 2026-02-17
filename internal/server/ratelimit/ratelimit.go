package ratelimit

import "time"

// RateLimiter abstracts per-session message admission decisions.
type RateLimiter interface {
	// Allow reports whether one event at time now should be admitted.
	// Callers should treat false as a soft drop (reject without blocking/retrying).
	Allow(now time.Time) bool
}
