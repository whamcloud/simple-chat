// Configuration structure for WebSocket service

/// Server configuration
#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
}

impl Config {
    #[allow(dead_code)]
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}
