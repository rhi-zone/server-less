//! Example WebSocket service demonstrating the #[ws] macro.
//!
//! Run with: cargo run --example ws_service
//! Test with: websocat ws://localhost:3000/ws
//!
//! Send JSON-RPC messages like:
//! {"method": "echo", "params": {"message": "hello"}}
//! {"method": "add", "params": {"a": 5, "b": 3}}

use rhizome_trellis::ws;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct EchoService {
    message_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Default for EchoService {
    fn default() -> Self {
        Self::new()
    }
}

impl EchoService {
    pub fn new() -> Self {
        Self {
            message_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
}

#[ws(path = "/ws")]
impl EchoService {
    /// Echo a message back
    pub fn echo(&self, message: String) -> Message {
        let id = self
            .message_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Message {
            id,
            content: format!("Echo: {}", message),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Add two numbers
    pub fn add(&self, a: i64, b: i64) -> i64 {
        a + b
    }

    /// Multiply two numbers
    pub fn multiply(&self, a: i64, b: i64) -> i64 {
        a * b
    }

    /// Get message count
    pub fn get_count(&self) -> u64 {
        self.message_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Reverse a string
    pub fn reverse(&self, text: String) -> String {
        text.chars().rev().collect()
    }
}

#[tokio::main]
async fn main() {
    let service = EchoService::new();

    println!(
        "Available WebSocket methods: {:?}",
        EchoService::ws_methods()
    );
    println!("\nStarting WebSocket server on ws://localhost:3000/ws");
    println!("Test with: websocat ws://localhost:3000/ws");
    println!("\nExample messages:");
    println!(r#"  {{"method": "echo", "params": {{"message": "hello"}}}}"#);
    println!(r#"  {{"method": "add", "params": {{"a": 5, "b": 3}}}}"#);
    println!(r#"  {{"method": "reverse", "params": {{"text": "hello world"}}}}"#);

    let app = service.ws_router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
