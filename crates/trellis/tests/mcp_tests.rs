//! Integration tests for the MCP macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use rhizome_trellis::mcp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Item {
    id: String,
    name: String,
}

#[derive(Clone)]
struct TestService {
    items: Vec<Item>,
}

#[mcp(namespace = "test")]
impl TestService {
    /// Get all items
    pub fn list_items(&self) -> Vec<Item> {
        self.items.clone()
    }

    /// Get item by ID
    pub fn get_item(&self, item_id: String) -> Option<Item> {
        self.items.iter().find(|i| i.id == item_id).cloned()
    }

    /// Create an item
    pub fn create_item(&self, name: String) -> Item {
        Item {
            id: "new".to_string(),
            name,
        }
    }

    /// Search items with optional limit
    pub fn search_items(&self, query: String, limit: Option<u32>) -> Vec<Item> {
        let limit = limit.unwrap_or(10) as usize;
        self.items
            .iter()
            .filter(|i| i.name.contains(&query))
            .take(limit)
            .cloned()
            .collect()
    }
}

#[test]
fn test_mcp_tools_generated() {
    let tools = TestService::mcp_tools();
    assert_eq!(tools.len(), 4);

    // Check tool names
    let names: Vec<_> = tools
        .iter()
        .map(|t| t.get("name").unwrap().as_str().unwrap())
        .collect();
    assert!(names.contains(&"test_list_items"));
    assert!(names.contains(&"test_get_item"));
    assert!(names.contains(&"test_create_item"));
    assert!(names.contains(&"test_search_items"));
}

#[test]
fn test_mcp_tool_names() {
    let names = TestService::mcp_tool_names();
    assert_eq!(names.len(), 4);
    assert!(names.contains(&"test_list_items"));
}

#[test]
fn test_mcp_call_list() {
    let service = TestService {
        items: vec![Item {
            id: "1".to_string(),
            name: "Test".to_string(),
        }],
    };

    let result = service.mcp_call("test_list_items", serde_json::json!({}));
    assert!(result.is_ok());

    let items: Vec<Item> = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Test");
}

#[test]
fn test_mcp_call_get_option() {
    let service = TestService {
        items: vec![Item {
            id: "1".to_string(),
            name: "Test".to_string(),
        }],
    };

    // Found
    let result = service.mcp_call("test_get_item", serde_json::json!({"item_id": "1"}));
    assert!(result.is_ok());
    let item: Item = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(item.id, "1");

    // Not found
    let result = service.mcp_call("test_get_item", serde_json::json!({"item_id": "999"}));
    assert!(result.is_ok());
    assert!(result.unwrap().is_null());
}

#[test]
fn test_mcp_call_create() {
    let service = TestService { items: vec![] };

    let result = service.mcp_call("test_create_item", serde_json::json!({"name": "NewItem"}));
    assert!(result.is_ok());

    let item: Item = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(item.name, "NewItem");
}

#[test]
fn test_mcp_call_with_optional_param() {
    let service = TestService {
        items: vec![
            Item {
                id: "1".to_string(),
                name: "Apple".to_string(),
            },
            Item {
                id: "2".to_string(),
                name: "Apricot".to_string(),
            },
        ],
    };

    // Without limit
    let result = service.mcp_call("test_search_items", serde_json::json!({"query": "Ap"}));
    assert!(result.is_ok());
    let items: Vec<Item> = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(items.len(), 2);

    // With limit
    let result = service.mcp_call(
        "test_search_items",
        serde_json::json!({"query": "Ap", "limit": 1}),
    );
    assert!(result.is_ok());
    let items: Vec<Item> = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(items.len(), 1);
}

#[test]
fn test_mcp_call_unknown_tool() {
    let service = TestService { items: vec![] };
    let result = service.mcp_call("unknown_tool", serde_json::json!({}));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown tool"));
}

#[test]
fn test_mcp_tool_schema() {
    let tools = TestService::mcp_tools();

    // Find create_item tool
    let create_tool = tools
        .iter()
        .find(|t| t.get("name").unwrap() == "test_create_item")
        .unwrap();

    // Check input schema
    let schema = create_tool.get("inputSchema").unwrap();
    assert_eq!(schema.get("type").unwrap(), "object");

    let properties = schema.get("properties").unwrap().as_object().unwrap();
    assert!(properties.contains_key("name"));

    let required = schema.get("required").unwrap().as_array().unwrap();
    assert!(required.contains(&serde_json::json!("name")));
}

// ============================================================================
// Async Method Tests
// ============================================================================

/// Service with async methods
#[derive(Clone)]
struct AsyncService;

#[mcp(namespace = "async")]
impl AsyncService {
    /// Sync method - works with both sync and async call
    pub fn sync_add(&self, a: i64, b: i64) -> i64 {
        a + b
    }

    /// Async method - only works with async call
    pub async fn async_fetch(&self, id: String) -> String {
        // Simulate async operation
        format!("Fetched: {}", id)
    }

    /// Another async method
    pub async fn async_compute(&self, n: i64) -> i64 {
        // Simulate async computation
        n * 2
    }
}

#[test]
fn test_mcp_sync_method_with_sync_call() {
    let service = AsyncService;

    // Sync method should work with sync call
    let result = service.mcp_call("async_sync_add", serde_json::json!({"a": 5, "b": 3}));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), serde_json::json!(8));
}

#[test]
fn test_mcp_async_method_with_sync_call_returns_error() {
    let service = AsyncService;

    // Async method should return error with sync call
    let result = service.mcp_call("async_async_fetch", serde_json::json!({"id": "123"}));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Async methods not supported"));
}

#[tokio::test]
async fn test_mcp_sync_method_with_async_call() {
    let service = AsyncService;

    // Sync method should work with async call
    let result = service
        .mcp_call_async("async_sync_add", serde_json::json!({"a": 10, "b": 7}))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), serde_json::json!(17));
}

#[tokio::test]
async fn test_mcp_async_method_with_async_call() {
    let service = AsyncService;

    // Async method should work with async call
    let result = service
        .mcp_call_async("async_async_fetch", serde_json::json!({"id": "abc"}))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), serde_json::json!("Fetched: abc"));
}

#[tokio::test]
async fn test_mcp_async_compute() {
    let service = AsyncService;

    let result = service
        .mcp_call_async("async_async_compute", serde_json::json!({"n": 21}))
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), serde_json::json!(42));
}
