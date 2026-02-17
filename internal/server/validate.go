package server

import (
	"fmt"
	"strings"
	"unicode"
)

// ValidateUserID enforces a conservative identifier format for stable UX/logging.
func ValidateUserID(userID string, maxLen int) error {
	userID = strings.TrimSpace(userID)
	if userID == "" {
		return fmt.Errorf("empty user_id")
	}
	if len(userID) > maxLen {
		return fmt.Errorf("user_id too long (max=%d)", maxLen)
	}
	for _, r := range userID {
		if (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') || r == '_' || r == '-' || r == '.' {
			continue
		}
		if unicode.IsSpace(r) {
			return fmt.Errorf("user_id contains whitespace")
		}
		return fmt.Errorf("user_id contains invalid character: %q", r)
	}
	return nil
}

// ValidateMessage rejects empty/oversized payloads before enqueue/broadcast.
func ValidateMessage(text string, maxLen int) error {
	text = strings.TrimSpace(text)
	if text == "" {
		return fmt.Errorf("empty message")
	}
	if len(text) > maxLen {
		return fmt.Errorf("message too long (max=%d)", maxLen)
	}
	return nil
}
