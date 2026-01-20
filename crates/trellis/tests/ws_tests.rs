//! Integration tests for the WebSocket macro.

use serde::{Deserialize, Serialize};
use trellis::ws;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Item {
    id: u32,
    name: String,
}

#[derive(Clone)]
struct TestService {
    counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl TestService {
    fn new() -> Self {
        Self {
            counter: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }
}

#[ws(path = "/ws")]
impl TestService {
    /// Echo a message
    pub fn echo(&self, message: String) -> String {
        format!("Echo: {}", message)
    }

    /// Add two numbers
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Get next counter value
    pub fn next_id(&self) -> u32 {
        self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Create an item
    pub fn create_item(&self, name: String) -> Item {
        let id = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Item { id, name }
    }

    /// Search with optional limit
    pub fn search(&self, query: String, limit: Option<u32>) -> Vec<Item> {
        let limit = limit.unwrap_or(10);
        (0..limit).map(|i| Item {
            id: i,
            name: format!("{} {}", query, i),
        }).collect()
    }
}

#[test]
fn test_ws_methods_generated() {
    let methods = TestService::ws_methods();
    assert_eq!(methods.len(), 5);
    assert!(methods.contains(&"echo"));
    assert!(methods.contains(&"add"));
    assert!(methods.contains(&"next_id"));
    assert!(methods.contains(&"create_item"));
    assert!(methods.contains(&"search"));
}

#[test]
fn test_ws_handle_echo() {
    let service = TestService::new();
    let response = service.ws_handle_message(r#"{"method": "echo", "params": {"message": "hello"}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], "Echo: hello");
}

#[test]
fn test_ws_handle_add() {
    let service = TestService::new();
    let response = service.ws_handle_message(r#"{"method": "add", "params": {"a": 5, "b": 3}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], 8);
}

#[test]
fn test_ws_handle_with_id() {
    let service = TestService::new();
    let response = service.ws_handle_message(r#"{"method": "add", "params": {"a": 1, "b": 2}, "id": 42}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], 3);
    assert_eq!(json["id"], 42);
}

#[test]
fn test_ws_handle_unknown_method() {
    let service = TestService::new();
    let response = service.ws_handle_message(r#"{"method": "unknown", "params": {}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert!(json["error"]["message"].as_str().unwrap().contains("Unknown method"));
}

#[test]
fn test_ws_handle_missing_param() {
    let service = TestService::new();
    let response = service.ws_handle_message(r#"{"method": "echo", "params": {}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert!(json["error"]["message"].as_str().unwrap().contains("Missing required parameter"));
}

#[test]
fn test_ws_handle_optional_param() {
    let service = TestService::new();

    // Without optional param
    let response = service.ws_handle_message(r#"{"method": "search", "params": {"query": "test"}}"#);
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let items = json["result"].as_array().unwrap();
    assert_eq!(items.len(), 10); // default limit

    // With optional param
    let response = service.ws_handle_message(r#"{"method": "search", "params": {"query": "test", "limit": 3}}"#);
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let items = json["result"].as_array().unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn test_ws_handle_create_item() {
    let service = TestService::new();
    let response = service.ws_handle_message(r#"{"method": "create_item", "params": {"name": "Test Item"}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"]["name"], "Test Item");
    assert!(json["result"]["id"].as_u64().is_some());
}

#[test]
fn test_ws_router_created() {
    let service = TestService::new();
    let _router = service.ws_router();
    // Router is created successfully
}

#[test]
fn test_ws_invalid_json() {
    let service = TestService::new();
    let response = service.ws_handle_message("not valid json");
    // Invalid JSON returns Err early (before forming a JSON-RPC response)
    assert!(response.is_err());
    assert!(response.unwrap_err().contains("Invalid JSON"));
}
