package server

import "sync"

// Registry tracks active chat sessions by user id.
// It is safe for concurrent use by multiple goroutines.
type Registry struct {
	// mu guards all access to byUser.
	mu sync.RWMutex
	// byUser stores the currently active session for each user.
	byUser map[string]*Session
}

// NewRegistry creates an empty, ready-to-use session registry.
func NewRegistry() *Registry {
	return &Registry{byUser: make(map[string]*Session)}
}

// Register inserts a session if the user is not already present.
// It returns false when the user id is already registered.
func (r *Registry) Register(user string, s *Session) bool {
	r.mu.Lock()
	defer r.mu.Unlock()
	if _, ok := r.byUser[user]; ok {
		return false
	}
	r.byUser[user] = s
	return true
}

// Unregister removes the session for a user if it exists.
func (r *Registry) Unregister(user string) {
	r.mu.Lock()
	defer r.mu.Unlock()
	delete(r.byUser, user)
}

// Range calls fn for each (userID, session) while holding RLock.
// fn must NOT call back into Registry methods that take locks.
func (r *Registry) Range(fn func(user string, sess *Session)) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	for k, v := range r.byUser {
		fn(k, v)
	}
}

// Count returns the number of currently registered users.
func (r *Registry) Count() int {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return len(r.byUser)
}
