//! Example demonstrating parameter inference and customization.
//!
//! The `#[param(...)]` attribute is consumed by the proc macro at compile time
//! and works on stable Rust. It is fully supported with `#[cli]` (which strips
//! the attributes from emitted code). For `#[http]`, the inference rules handle
//! most cases without explicit attributes — see docs/design/param-attributes.md.
//!
//! This example demonstrates the inference rules:
//! - `id` / `*_id` params become path parameters
//! - GET methods use query params, POST/PUT/PATCH use body params
//! - `Option<T>` params are optional
//!
//! ```bash
//! cargo run --example param_service
//! # Then:
//! curl 'http://localhost:3000/search?query=rust&limit=5'
//! curl http://localhost:3000/items/42
//! curl -X POST http://localhost:3000/items -d '{"name":"widget"}'
//! ```

use serde::{Deserialize, Serialize};
use server_less::http;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub score: f64,
}

#[derive(Clone)]
pub struct SearchService;

#[http]
impl SearchService {
    /// Search for items (all params inferred as query for GET)
    pub fn search(&self, query: String, limit: Option<u32>) -> Vec<SearchResult> {
        let limit = limit.unwrap_or(20);
        vec![SearchResult {
            title: format!("Result for '{}' (limit={})", query, limit),
            score: 0.95,
        }]
    }

    /// Get item by ID (item_id inferred as path param via is_id heuristic)
    pub fn get_item(&self, item_id: u32) -> Option<String> {
        if item_id == 42 {
            Some("The answer".into())
        } else {
            None
        }
    }

    /// Create item (name inferred as body param for POST)
    pub fn create_item(&self, name: String, description: Option<String>) -> String {
        format!("Created '{}': {}", name, description.unwrap_or_default())
    }
}

#[tokio::main]
async fn main() {
    let service = SearchService;

    println!("OpenAPI spec:");
    println!(
        "{}",
        serde_json::to_string_pretty(&SearchService::openapi_spec()).unwrap()
    );

    println!("\nStarting server on http://localhost:3000");
    let app = service.http_router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
