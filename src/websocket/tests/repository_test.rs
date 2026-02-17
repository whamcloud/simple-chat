// Repository interface tests

#[cfg(test)]
mod tests {
    use crate::websocket::models::client::Client;
    use crate::websocket::repository::ChatRepository;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_add_client() {
        let repo = ChatRepository::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let client = Client::new("user1".to_string(), tx);

        assert!(repo.add_client("user1".to_string(), client).await.is_ok());
    }

    #[tokio::test]
    async fn test_duplicate_username() {
        let repo = ChatRepository::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        let client1 = Client::new("user1".to_string(), tx1);
        let client2 = Client::new("user1".to_string(), tx2);

        assert!(repo.add_client("user1".to_string(), client1).await.is_ok());
        assert!(repo.add_client("user1".to_string(), client2).await.is_err());
    }

    #[tokio::test]
    async fn test_remove_client() {
        let repo = ChatRepository::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let client = Client::new("user1".to_string(), tx);

        repo.add_client("user1".to_string(), client).await.unwrap();
        assert_eq!(repo.client_count().await, 1);

        repo.remove_client("user1").await;
        assert_eq!(repo.client_count().await, 0);
    }
}
