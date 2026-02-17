// Service layer unit tests

#[cfg(test)]
mod tests {
    use crate::websocket::models::client::Client;
    use crate::websocket::repository::ChatRepository;
    use crate::websocket::service::ChatService;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_handle_join() {
        let repo = ChatRepository::new();
        let service = ChatService::new(repo);
        let (tx, _rx) = mpsc::unbounded_channel();
        let client = Client::new("user1".to_string(), tx);

        assert!(service
            .handle_join("user1".to_string(), client)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_handle_leave() {
        let repo = ChatRepository::new();
        let service = ChatService::new(repo.clone());
        let (tx, _rx) = mpsc::unbounded_channel();
        let client = Client::new("user1".to_string(), tx);

        service
            .handle_join("user1".to_string(), client)
            .await
            .unwrap();
        service.handle_leave("user1").await;

        assert_eq!(repo.client_count().await, 0);
    }
}
