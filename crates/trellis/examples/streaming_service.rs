//! Example streaming service demonstrating SSE with the #[http] macro.
//!
//! Run with: cargo run --example streaming_service
//! Test with: curl http://localhost:3000/api/count?n=5

use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use trellis::http;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub message: String,
}

#[derive(Clone)]
pub struct StreamService;

impl Default for StreamService {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamService {
    pub fn new() -> Self {
        Self
    }
}

#[http(prefix = "/api")]
impl StreamService {
    /// Count from 0 to n (SSE stream)
    /// Note: Using `use<>` syntax for Rust 2024 impl Trait capture rules
    pub fn stream_count(&self, n: u64) -> impl Stream<Item = Event> + use<> {
        stream::iter((0..n).map(|i| Event {
            id: i,
            message: format!("Count: {}", i),
        }))
    }

    /// Stream a list of items
    pub fn list_events(&self) -> Vec<Event> {
        vec![
            Event {
                id: 1,
                message: "First".to_string(),
            },
            Event {
                id: 2,
                message: "Second".to_string(),
            },
            Event {
                id: 3,
                message: "Third".to_string(),
            },
        ]
    }
}

#[tokio::main]
async fn main() {
    let service = StreamService::new();

    println!("Starting streaming server on http://localhost:3000");
    println!("\nTest SSE streaming:");
    println!("  curl http://localhost:3000/api/stream-counts?n=5");
    println!("\nTest regular list:");
    println!("  curl http://localhost:3000/api/events");

    let app = service.http_router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
