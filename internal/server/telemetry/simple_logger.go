package telemetry

import (
	"context"
	"log/slog"
	"os"
)

// SimpleLogger writes structured info logs to stdout and error logs to stderr.
type SimpleLogger struct {
	info *slog.Logger
	err  *slog.Logger
}

// NewSimpleLogger returns a structured logger suitable for local runs.
func NewSimpleLogger() SimpleLogger {
	return SimpleLogger{
		info: slog.New(slog.NewJSONHandler(os.Stdout, nil)),
		err:  slog.New(slog.NewJSONHandler(os.Stderr, nil)),
	}
}

func (l SimpleLogger) InfoContext(ctx context.Context, msg string, kv ...any) {
	l.info.LogAttrs(ctx, slog.LevelInfo, msg, kvToAttrs(kv...)...)
}

func (l SimpleLogger) ErrorContext(ctx context.Context, msg string, kv ...any) {
	l.err.LogAttrs(ctx, slog.LevelError, msg, kvToAttrs(kv...)...)
}

func kvToAttrs(kv ...any) []slog.Attr {
	if len(kv) == 0 {
		return nil
	}
	out := make([]slog.Attr, 0, len(kv)/2+1)
	for i := 0; i < len(kv); i += 2 {
		key := "field"
		if k, ok := kv[i].(string); ok && k != "" {
			key = k
		}
		if i+1 < len(kv) {
			out = append(out, slog.Any(key, kv[i+1]))
			continue
		}
		out = append(out, slog.Any(key, nil))
	}
	return out
}
