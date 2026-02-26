//! Example demonstrating the #[server] blessed preset.
//!
//! `#[server]` = `#[http]` + `#[serve(http)]` in one attribute.
//!
//! ```bash
//! cargo run --example server_preset
//! # Then: curl http://localhost:3000/items
//! # Then: curl -X POST http://localhost:3000/items -H 'Content-Type: application/json' -d '{"name":"widget"}'
//! ```

use serde::{Deserialize, Serialize};
use server_less::server;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: u32,
    pub name: String,
}

#[derive(Clone)]
pub struct ItemService;

#[server]
impl ItemService {
    /// List all items
    pub fn list_items(&self) -> Vec<Item> {
        vec![
            Item {
                id: 1,
                name: "Widget".into(),
            },
            Item {
                id: 2,
                name: "Gadget".into(),
            },
        ]
    }

    /// Create a new item
    pub fn create_item(&self, name: String) -> Item {
        Item { id: 42, name }
    }

    /// Get item by ID
    pub fn get_item(&self, item_id: u32) -> Option<Item> {
        if item_id == 1 {
            Some(Item {
                id: 1,
                name: "Widget".into(),
            })
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Starting server on http://localhost:3000");
    println!("  GET  /items       - list items");
    println!("  POST /items       - create item");
    println!("  GET  /items/{{id}} - get item by ID");
    println!("  GET  /health      - health check");
    ItemService.serve("0.0.0.0:3000").await.unwrap();
}
