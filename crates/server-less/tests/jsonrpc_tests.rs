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

#[test]
fn test_jsonrpc_openapi_paths_generated() {
    let paths = Calculator::jsonrpc_openapi_paths();

    // Should have 1 path: POST /rpc
    assert_eq!(paths.len(), 1);

    let rpc_path = &paths[0];
    assert_eq!(rpc_path.path, "/rpc");
    assert_eq!(rpc_path.method, "post");
    assert!(
        rpc_path
            .operation
            .summary
            .as_ref()
            .unwrap()
            .contains("JSON-RPC")
    );
    assert!(rpc_path.operation.request_body.is_some());

    // Check that responses include 200 and 204
    assert!(rpc_path.operation.responses.contains_key("200"));
    assert!(rpc_path.operation.responses.contains_key("204"));
}

// ============================================================================
// Mount Point Tests
// ============================================================================

/// Child service for mount testing
#[derive(Clone)]
struct MathTools;

#[jsonrpc]
impl MathTools {
    /// Add two numbers
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Double a number
    fn double(&self, n: i32) -> i32 {
        n * 2
    }
}

/// Another child service
#[derive(Clone)]
struct StringTools;

#[jsonrpc]
impl StringTools {
    /// Uppercase a string
    fn upper(&self, s: String) -> String {
        s.to_uppercase()
    }
}

/// Parent with static mounts
#[derive(Clone)]
struct JsonRpcApp {
    math: MathTools,
    strings: StringTools,
}

#[jsonrpc]
impl JsonRpcApp {
    /// Ping health check
    fn ping(&self) -> String {
        "pong".to_string()
    }

    /// Mount math tools
    fn math(&self) -> &MathTools {
        &self.math
    }

    /// Mount string tools
    fn strings(&self) -> &StringTools {
        &self.strings
    }
}

#[test]
fn test_jsonrpc_static_mount_methods_listed() {
    let methods = JsonRpcApp::jsonrpc_methods();

    // Leaf method
    assert!(methods.contains(&"ping"));
    // Mounted methods (dot-separated)
    assert!(methods.contains(&"math.add"));
    assert!(methods.contains(&"math.double"));
    assert!(methods.contains(&"strings.upper"));
}

#[tokio::test]
async fn test_jsonrpc_static_mount_dispatch() {
    let app = JsonRpcApp {
        math: MathTools,
        strings: StringTools,
    };

    // Dispatch to leaf
    let response = app
        .jsonrpc_handle(json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "params": {},
            "id": 1
        }))
        .await;
    assert_eq!(response["result"], "pong");

    // Dispatch to mounted child
    let response = app
        .jsonrpc_handle(json!({
            "jsonrpc": "2.0",
            "method": "math.add",
            "params": {"a": 10, "b": 5},
            "id": 2
        }))
        .await;
    assert_eq!(response["result"], 15);

    // Dispatch to another mount
    let response = app
        .jsonrpc_handle(json!({
            "jsonrpc": "2.0",
            "method": "strings.upper",
            "params": {"s": "hello"},
            "id": 3
        }))
        .await;
    assert_eq!(response["result"], "HELLO");
}

#[tokio::test]
async fn test_jsonrpc_static_mount_double() {
    let app = JsonRpcApp {
        math: MathTools,
        strings: StringTools,
    };

    let response = app
        .jsonrpc_handle(json!({
            "jsonrpc": "2.0",
            "method": "math.double",
            "params": {"n": 21},
            "id": 1
        }))
        .await;
    assert_eq!(response["result"], 42);
}

/// Slug mount: parent with parameterized child
#[derive(Clone)]
struct JsonRpcSlugApp {
    math: MathTools,
}

#[jsonrpc]
impl JsonRpcSlugApp {
    /// Access a calculator by ID
    fn calc(&self, id: String) -> &MathTools {
        let _ = &id;
        &self.math
    }
}

#[test]
fn test_jsonrpc_slug_mount_methods_listed() {
    let methods = JsonRpcSlugApp::jsonrpc_methods();

    assert!(methods.contains(&"calc.add"));
    assert!(methods.contains(&"calc.double"));
}

#[tokio::test]
async fn test_jsonrpc_slug_mount_dispatch() {
    let app = JsonRpcSlugApp { math: MathTools };

    let response = app
        .jsonrpc_handle(json!({
            "jsonrpc": "2.0",
            "method": "calc.add",
            "params": {"id": "calc-1", "a": 3, "b": 4},
            "id": 1
        }))
        .await;
    assert_eq!(response["result"], 7);
}

/// JsonRpcMount trait test
#[test]
fn test_jsonrpc_mount_trait_implemented() {
    use server_less::JsonRpcMount;

    let methods = <MathTools as JsonRpcMount>::jsonrpc_mount_methods();
    assert_eq!(methods.len(), 2);
    assert!(methods.contains(&"add".to_string()));
    assert!(methods.contains(&"double".to_string()));
}

/// Test sync dispatch via JsonRpcMount::jsonrpc_mount_dispatch
#[test]
fn test_jsonrpc_mount_dispatch_sync() {
    use server_less::JsonRpcMount;

    let math = MathTools;

    // Sync dispatch of a sync method works
    let result = math.jsonrpc_mount_dispatch("add", json!({"a": 7, "b": 3}));
    assert!(result.is_ok(), "sync dispatch should succeed for sync method");
    let val = result.unwrap();
    assert_eq!(val, json!(10));

    // Sync dispatch of another method
    let result = math.jsonrpc_mount_dispatch("double", json!({"n": 6}));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!(12));

    // Sync dispatch of a missing method returns Err
    let result = math.jsonrpc_mount_dispatch("nonexistent", json!({}));
    assert!(result.is_err(), "sync dispatch of unknown method should return Err");
}

/// Test that async-only methods return Err when dispatched synchronously
#[derive(Clone)]
struct AsyncOnlyService;

#[server_less::jsonrpc]
impl AsyncOnlyService {
    pub async fn only_async(&self, x: i32) -> i32 {
        x * 2
    }
    pub fn sync_method(&self, x: i32) -> i32 {
        x + 1
    }
}

#[test]
fn test_jsonrpc_mount_dispatch_sync_rejects_async() {
    use server_less::JsonRpcMount;

    let svc = AsyncOnlyService;

    // Sync method works
    let result = svc.jsonrpc_mount_dispatch("sync_method", json!({"x": 5}));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!(6));

    // Async-only method returns Err in sync context
    let result = svc.jsonrpc_mount_dispatch("only_async", json!({"x": 5}));
    assert!(
        result.is_err(),
        "async method should return Err in sync dispatch context"
    );
    assert!(
        result.unwrap_err().contains("sync context"),
        "error message should mention sync context"
    );
}

/// Test ErrorCode::jsonrpc_code() mapping
#[test]
fn test_error_code_jsonrpc_code() {
    use server_less::ErrorCode;
    // Standard invalid params code
    assert_eq!(ErrorCode::InvalidInput.jsonrpc_code(), -32602);
    // Internal error fallback
    assert_eq!(ErrorCode::Internal.jsonrpc_code(), -32603);
    // Method not found code maps to NotImplemented
    assert_eq!(ErrorCode::NotImplemented.jsonrpc_code(), -32601);
}

/// Test that ServerlessError::jsonrpc_code() propagates to JSON-RPC response
#[derive(Debug, server_less::ServerlessError)]
enum RpcError {
    #[error(code = InvalidInput, jsonrpc_code = -32602)]
    BadParams,
    #[error(code = NotFound)]
    Missing,
}

#[derive(Clone)]
struct ErrorService;

#[server_less::jsonrpc]
impl ErrorService {
    fn get_item(&self, id: i32) -> Result<String, RpcError> {
        if id < 0 {
            Err(RpcError::BadParams)
        } else if id == 0 {
            Err(RpcError::Missing)
        } else {
            Ok(format!("item-{}", id))
        }
    }
}

#[tokio::test]
async fn test_jsonrpc_error_code_from_serverless_error() {
    let svc = ErrorService;

    // BadParams → jsonrpc_code -32602
    let response = svc
        .jsonrpc_handle(json!({
            "jsonrpc": "2.0",
            "method": "get_item",
            "params": {"id": -1},
            "id": 1
        }))
        .await;
    assert!(response["error"].is_object());
    assert_eq!(
        response["error"]["code"],
        -32602,
        "BadParams should produce JSON-RPC code -32602"
    );

    // Missing → jsonrpc_code derived from NotFound (-32002)
    let response = svc
        .jsonrpc_handle(json!({
            "jsonrpc": "2.0",
            "method": "get_item",
            "params": {"id": 0},
            "id": 2
        }))
        .await;
    assert!(response["error"].is_object());
    assert_eq!(
        response["error"]["code"],
        server_less::ErrorCode::NotFound.jsonrpc_code(),
        "Missing should produce the NotFound JSON-RPC code"
    );

    // Successful call
    let response = svc
        .jsonrpc_handle(json!({
            "jsonrpc": "2.0",
            "method": "get_item",
            "params": {"id": 42},
            "id": 3
        }))
        .await;
    assert_eq!(response["result"], "item-42");
}
