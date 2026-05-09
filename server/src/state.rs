use std::{collections::HashSet, sync::Arc};

use proto::ServerMsg;
use tokio::sync::{Mutex, broadcast};

pub const BROADCAST_CAPACITY: usize = 512;

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<Inner>,
}

pub struct Inner {
    pub users: Mutex<HashSet<String>>,
    pub tx: broadcast::Sender<Arc<ServerMsg>>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            inner: Arc::new(Inner {
                users: Mutex::new(HashSet::new()),
                tx,
            }),
        }
    }

    pub async fn register(&self, username: &str) -> bool {
        self.inner.users.lock().await.insert(username.to_owned())
    }

    pub async fn deregister(&self, username: &str) {
        self.inner.users.lock().await.remove(username);
    }

    pub fn broadcast(&self, msg: Arc<ServerMsg>) {
        let _ = self.inner.tx.send(msg);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<ServerMsg>> {
        self.inner.tx.subscribe()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_new_username_returns_true() {
        let state = AppState::new();
        assert!(state.register("alice").await);
    }

    #[tokio::test]
    async fn register_duplicate_username_returns_false() {
        let state = AppState::new();
        state.register("alice").await;
        assert!(!state.register("alice").await);
    }

    #[tokio::test]
    async fn deregister_allows_reregistration() {
        let state = AppState::new();
        state.register("alice").await;
        state.deregister("alice").await;
        assert!(state.register("alice").await);
    }

    #[tokio::test]
    async fn broadcast_with_no_receivers_does_not_panic() {
        let state = AppState::new();
        state.broadcast(Arc::new(ServerMsg::UserJoined {
            username: "alice".into(),
        }));
    }

    #[tokio::test]
    async fn subscribe_receives_broadcast() {
        let state = AppState::new();
        let mut rx = state.subscribe();
        let msg = Arc::new(ServerMsg::UserJoined {
            username: "alice".into(),
        });
        state.broadcast(Arc::clone(&msg));
        let received = rx.recv().await.unwrap();
        assert_eq!(*received, *msg);
    }
}
