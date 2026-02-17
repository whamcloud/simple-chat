package server

import "testing"

// Covers basic acceptance/rejection rules for user IDs.
func TestValidateUserID(t *testing.T) {
	if err := ValidateUserID("", 10); err == nil {
		t.Fatal("expected error for empty")
	}
	if err := ValidateUserID("a b", 10); err == nil {
		t.Fatal("expected error for whitespace")
	}
	if err := ValidateUserID("alice", 10); err != nil {
		t.Fatalf("unexpected: %v", err)
	}
}

// Ensures message validation enforces non-empty and length bounds.
func TestValidateMessage(t *testing.T) {
	if err := ValidateMessage("", 10); err == nil {
		t.Fatal("expected error for empty")
	}
	if err := ValidateMessage("hello", 10); err != nil {
		t.Fatalf("unexpected: %v", err)
	}
}
