//! Integration tests for the HTTP macro.

use serde::{Deserialize, Serialize};
use trellis::http;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Item {
    id: String,
    name: String,
}

#[derive(Debug)]
#[allow(dead_code)]
enum ItemError {
    NotFound,
    Invalid,
}

impl std::fmt::Display for ItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemError::NotFound => write!(f, "Item not found"),
            ItemError::Invalid => write!(f, "Invalid item"),
        }
    }
}

impl std::error::Error for ItemError {}

#[derive(Clone)]
struct ItemService {
    items: std::sync::Arc<std::sync::Mutex<Vec<Item>>>,
}

impl ItemService {
    fn new() -> Self {
        Self {
            items: std::sync::Arc::new(std::sync::Mutex::new(vec![Item {
                id: "1".to_string(),
                name: "Test".to_string(),
            }])),
        }
    }
}

#[http(prefix = "/api/v1")]
impl ItemService {
    /// List all items
    pub fn list_items(&self) -> Vec<Item> {
        self.items.lock().unwrap().clone()
    }

    /// Get item by ID
    pub fn get_item(&self, item_id: String) -> Option<Item> {
        self.items.lock().unwrap().iter().find(|i| i.id == item_id).cloned()
    }

    /// Create an item
    pub fn create_item(&self, name: String) -> Result<Item, ItemError> {
        if name.is_empty() {
            return Err(ItemError::Invalid);
        }
        let mut items = self.items.lock().unwrap();
        let item = Item {
            id: (items.len() + 1).to_string(),
            name,
        };
        items.push(item.clone());
        Ok(item)
    }
}

#[test]
fn test_http_router_created() {
    let service = ItemService::new();
    let _router = service.http_router();
    // Router is created successfully
}

#[test]
fn test_openapi_spec_generated() {
    let spec = ItemService::openapi_spec();

    // Check basic structure
    assert_eq!(spec.get("openapi").unwrap(), "3.0.0");

    let info = spec.get("info").unwrap();
    assert_eq!(info.get("title").unwrap(), "ItemService");

    // Check paths exist
    let paths = spec.get("paths").unwrap().as_object().unwrap();
    assert!(paths.contains_key("/api/v1/items"));
}

#[test]
fn test_openapi_contains_operations() {
    let spec = ItemService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // GET /api/v1/items should have get and post
    let items_path = paths.get("/api/v1/items").unwrap();
    assert!(items_path.get("get").is_some());
    assert!(items_path.get("post").is_some());

    // Check operation details
    let get_op = items_path.get("get").unwrap();
    assert_eq!(get_op.get("operationId").unwrap(), "list_items");
    assert_eq!(get_op.get("summary").unwrap(), "List all items");
}

// HTTP handler tests would require setting up an actual test server
// For comprehensive HTTP testing, consider using axum::TestClient
// These tests verify the macro generates the expected code structure
