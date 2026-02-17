package telemetry

import "context"

// NoopMetrics drops all metric emissions.
type NoopMetrics struct{}

func (NoopMetrics) IncCounter(string, int64, ...Attr)         {}
func (NoopMetrics) ObserveHistogram(string, float64, ...Attr) {}
func (NoopMetrics) SetGauge(string, int64, ...Attr)           {}

// NoopLogger drops logs while satisfying the Logger interface.
type NoopLogger struct{}

func (NoopLogger) InfoContext(context.Context, string, ...any)  {}
func (NoopLogger) ErrorContext(context.Context, string, ...any) {}
