package telemetry

import "context"

// Attr is a lightweight metric/log label pair.
type Attr struct {
	Key   string
	Value string
}

// Logger captures structured log sinks used by server components.
type Logger interface {
	InfoContext(ctx context.Context, msg string, kv ...any)
	ErrorContext(ctx context.Context, msg string, kv ...any)
}

// Metrics is the metric emission surface consumed by runtime code.
type Metrics interface {
	IncCounter(name string, delta int64, attrs ...Attr)
	ObserveHistogram(name string, value float64, attrs ...Attr)
	SetGauge(name string, value int64, attrs ...Attr)
}
