//! Integration tests for the JSON-RPC over HTTP macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde_json::json;
use server_less::jsonrpc;

#[derive(Clone)]
struct Calculator;

#[jsonrpc]
impl Calculator {
    /// Add two numbers
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Subtract two numbers
    pub fn subtract(&self, a: i32, b: i32) -> i32 {
        a - b
    }

    /// Multiply two numbers
    pub fn multiply(&self, a: i32, b: i32) -> i32 {
        a * b
    }

    /// Echo a message
    pub fn echo(&self, message: String) -> String {
        message
    }
}

#[test]
fn test_jsonrpc_methods_list() {
    let methods = Calculator::jsonrpc_methods();
    assert!(methods.contains(&"add"));
    assert!(methods.contains(&"subtract"));
    assert!(methods.contains(&"multiply"));
    assert!(methods.contains(&"echo"));
}

#[tokio::test]
async fn test_jsonrpc_handle_add() {
    let calc = Calculator;
    let request = json!({
        "jsonrpc": "2.0",
        "method": "add",
        "params": {"a": 5, "b": 3},
        "id": 1
    });

    let response = calc.jsonrpc_handle(request).await;

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["result"], 8);
    assert_eq!(response["id"], 1);
}

#[tokio::test]
async fn test_jsonrpc_handle_subtract() {
    let calc = Calculator;
    let request = json!({
        "jsonrpc": "2.0",
        "method": "subtract",
        "params": {"a": 10, "b": 4},
        "id": 2
    });

    let response = calc.jsonrpc_handle(request).await;
    assert_eq!(response["result"], 6);
}

#[tokio::test]
async fn test_jsonrpc_handle_string_params() {
    let calc = Calculator;
    let request = json!({
        "jsonrpc": "2.0",
        "method": "echo",
        "params": {"message": "hello world"},
        "id": 3
    });

    let response = calc.jsonrpc_handle(request).await;
    assert_eq!(response["result"], "hello world");
}

#[tokio::test]
async fn test_jsonrpc_method_not_found() {
    let calc = Calculator;
    let request = json!({
        "jsonrpc": "2.0",
        "method": "nonexistent",
        "params": {},
        "id": 4
    });

    let response = calc.jsonrpc_handle(request).await;
    assert!(response["error"].is_object());
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

#[tokio::test]
async fn test_jsonrpc_invalid_version() {
    let calc = Calculator;
    let request = json!({
        "jsonrpc": "1.0",
        "method": "add",
        "params": {"a": 1, "b": 2},
        "id": 5
    });

    let response = calc.jsonrpc_handle(request).await;
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32600);
}

#[tokio::test]
async fn test_jsonrpc_notification_no_response() {
    let calc = Calculator;
    // Notification = no id field
    let request = json!({
        "jsonrpc": "2.0",
        "method": "add",
        "params": {"a": 1, "b": 2}
    });

    let response = calc.jsonrpc_handle(request).await;
    // Notifications return null (no response)
    assert!(response.is_null());
}

#[tokio::test]
async fn test_jsonrpc_batch_request() {
    let calc = Calculator;
    let request = json!([
        {"jsonrpc": "2.0", "method": "add", "params": {"a": 1, "b": 2}, "id": 1},
        {"jsonrpc": "2.0", "method": "multiply", "params": {"a": 3, "b": 4}, "id": 2}
    ]);

    let response = calc.jsonrpc_handle(request).await;

    assert!(response.is_array());
    let arr = response.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["result"], 3);
    assert_eq!(arr[1]["result"], 12);
}

#[tokio::test]
async fn test_jsonrpc_batch_with_notifications() {
    let calc = Calculator;
    let request = json!([
        {"jsonrpc": "2.0", "method": "add", "params": {"a": 1, "b": 2}, "id": 1},
        {"jsonrpc": "2.0", "method": "multiply", "params": {"a": 3, "b": 4}}  // notification
    ]);

    let response = calc.jsonrpc_handle(request).await;

    // Only the non-notification gets a response
    assert!(response.is_array());
    let arr = response.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["result"], 3);
}

// Test async methods
#[derive(Clone)]
struct AsyncService;

#[jsonrpc]
impl AsyncService {
    pub async fn async_echo(&self, message: String) -> String {
        // Simulate async work
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        message
    }
}

#[tokio::test]
async fn test_jsonrpc_async_method() {
    let svc = AsyncService;
    let request = json!({
        "jsonrpc": "2.0",
        "method": "async_echo",
        "params": {"message": "async works"},
        "id": 1
    });

    let response = svc.jsonrpc_handle(request).await;
    assert_eq!(response["result"], "async works");
}

// Test custom path
#[derive(Clone)]
struct CustomPathService;

#[jsonrpc(path = "/api/v1/rpc")]
impl CustomPathService {
    pub fn ping(&self) -> String {
        "pong".to_string()
    }
}

#[test]
fn test_jsonrpc_custom_path_compiles() {
    // Just verify it compiles with custom path
    let methods = CustomPathService::jsonrpc_methods();
    assert!(methods.contains(&"ping"));
}
