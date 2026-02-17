// Package client tests CLI/environment config parsing behavior.
package client

import "testing"

// getenvFromMap creates a deterministic getenv stub for table-like tests.
func getenvFromMap(m map[string]string) func(string) string {
	return func(k string) string { return m[k] }
}

// TestDefaultsWhenNoEnvNoFlags verifies defaults apply but missing user fails.
func TestDefaultsWhenNoEnvNoFlags(t *testing.T) {
	cfg, err := ParseCLIConfig(nil, getenvFromMap(map[string]string{}))
	// User is required, so this should fail even with valid host/port defaults.
	if err == nil {
		t.Fatal("expected error for missing user")
	}
	if cfg.ServerHost != "localhost" {
		t.Fatalf("host mismatch: got %q want %q", cfg.ServerHost, "localhost")
	}
	if cfg.ServerPort != 9786 {
		t.Fatalf("port mismatch: got %d want %d", cfg.ServerPort, 9786)
	}
}

// TestUserFromEnv verifies CHAT_USER satisfies required username input.
func TestUserFromEnv(t *testing.T) {
	cfg, err := ParseCLIConfig(nil, getenvFromMap(map[string]string{
		"CHAT_USER": "alice",
	}))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.UserID != "alice" {
		t.Fatalf("user mismatch: got %q want %q", cfg.UserID, "alice")
	}
}

// TestHostPortFromEnv verifies host/port env overrides are applied.
func TestHostPortFromEnv(t *testing.T) {
	cfg, err := ParseCLIConfig(nil, getenvFromMap(map[string]string{
		"CHAT_USER":        "alice",
		"CHAT_SERVER_HOST": "1.2.3.4",
		"CHAT_SERVER_PORT": "9999",
	}))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.ServerHost != "1.2.3.4" {
		t.Fatalf("host mismatch: got %q want %q", cfg.ServerHost, "1.2.3.4")
	}
	if cfg.ServerPort != 9999 {
		t.Fatalf("port mismatch: got %d want %d", cfg.ServerPort, 9999)
	}
}

// TestFlagsOverrideEnv verifies flags take precedence over environment values.
func TestFlagsOverrideEnv(t *testing.T) {
	cfg, err := ParseCLIConfig(
		[]string{"--user", "bob", "--server-host", "example.com", "--server-port", "1111"},
		getenvFromMap(map[string]string{
			"CHAT_USER":        "alice",
			"CHAT_SERVER_HOST": "1.2.3.4",
			"CHAT_SERVER_PORT": "9999",
		}),
	)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cfg.UserID != "bob" {
		t.Fatalf("user mismatch: got %q want %q", cfg.UserID, "bob")
	}
	if cfg.ServerHost != "example.com" {
		t.Fatalf("host mismatch: got %q want %q", cfg.ServerHost, "example.com")
	}
	if cfg.ServerPort != 1111 {
		t.Fatalf("port mismatch: got %d want %d", cfg.ServerPort, 1111)
	}
}

// TestInvalidEnvPort verifies malformed CHAT_SERVER_PORT is rejected.
func TestInvalidEnvPort(t *testing.T) {
	_, err := ParseCLIConfig(nil, getenvFromMap(map[string]string{
		"CHAT_USER":        "alice",
		"CHAT_SERVER_PORT": "notanint",
	}))
	if err == nil {
		t.Fatal("expected error for invalid env port")
	}
}

// TestMissingUserEvenIfHostPortPresent verifies user is still required.
func TestMissingUserEvenIfHostPortPresent(t *testing.T) {
	_, err := ParseCLIConfig(nil, getenvFromMap(map[string]string{
		"CHAT_SERVER_HOST": "1.2.3.4",
		"CHAT_SERVER_PORT": "9999",
	}))
	if err == nil {
		t.Fatal("expected error for missing user")
	}
}
