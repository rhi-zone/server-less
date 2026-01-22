//! Example streaming service demonstrating SSE with the #[http] macro.
//!
//! This example shows how to return Server-Sent Events (SSE) streams from HTTP endpoints.
//! The `impl Stream<Item = T>` return type automatically enables SSE responses.
//!
//! **Important:** When using Rust 2024 edition, you must add `+ use<>` to your
//! impl Trait return types. This tells the compiler to capture all generic parameters.
//!
//! Run with: cargo run --example streaming_service
//! Test with: curl http://localhost:3000/api/stream-counts?n=5

use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use server_less::http;

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
    ///
    /// Returns a stream that emits events in real-time as SSE.
    /// The `+ use<>` syntax is required for Rust 2024 edition.
    ///
    /// Example: GET /api/stream-counts?n=5
    /// Response: SSE stream with events as they're generated
    pub fn stream_count(&self, n: u64) -> impl Stream<Item = Event> + use<> {
        stream::iter((0..n).map(|i| Event {
            id: i,
            message: format!("Count: {}", i),
        }))
    }

    /// Stream events with delays (simulates real-time data)
    ///
    /// Example: GET /api/stream-ticks?n=3
    pub async fn stream_ticks(&self, n: u64) -> impl Stream<Item = Event> + use<> {
        use tokio::time::{Duration, interval};
        let ticker = interval(Duration::from_secs(1));

        stream::unfold((ticker, 0, n), |(mut ticker, count, max)| async move {
            if count >= max {
                return None;
            }
            ticker.tick().await;
            Some((
                Event {
                    id: count,
                    message: format!("Tick {}", count),
                },
                (ticker, count + 1, max),
            ))
        })
    }

    /// Regular list endpoint (non-streaming)
    ///
    /// Example: GET /api/events
    /// Response: JSON array
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
    println!("\nSSE Streaming endpoints:");
    println!("  curl http://localhost:3000/api/stream-counts?n=5");
    println!("    └─ Immediate stream of count events");
    println!("  curl http://localhost:3000/api/stream-ticks?n=3");
    println!("    └─ Delayed stream (1 event per second)");
    println!("\nRegular JSON endpoint:");
    println!("  curl http://localhost:3000/api/events");
    println!("    └─ Returns array of events\n");

    let app = service.http_router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
