//! Integration tests for the WebSocket macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use server_less::{server, ws};

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
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Create an item
    pub fn create_item(&self, name: String) -> Item {
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Item { id, name }
    }

    /// Search with optional limit
    pub fn search(&self, query: String, limit: Option<u32>) -> Vec<Item> {
        let limit = limit.unwrap_or(10);
        (0..limit)
            .map(|i| Item {
                id: i,
                name: format!("{} {}", query, i),
            })
            .collect()
    }
}

#[test]
fn test_ws_methods_generated() {
    let methods = TestService::ws_methods();
    assert_eq!(methods.len(), 5);
    assert!(methods.contains(&"echo".to_string()));
    assert!(methods.contains(&"add".to_string()));
    assert!(methods.contains(&"next_id".to_string()));
    assert!(methods.contains(&"create_item".to_string()));
    assert!(methods.contains(&"search".to_string()));
}

#[test]
fn test_ws_handle_echo() {
    let service = TestService::new();
    let response =
        service.ws_handle_message(r#"{"method": "echo", "params": {"message": "hello"}}"#);
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
    let response =
        service.ws_handle_message(r#"{"method": "add", "params": {"a": 1, "b": 2}, "id": 42}"#);
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
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown method")
    );
}

#[test]
fn test_ws_handle_missing_param() {
    let service = TestService::new();
    let response = service.ws_handle_message(r#"{"method": "echo", "params": {}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Missing required parameter")
    );
}

#[test]
fn test_ws_handle_optional_param() {
    let service = TestService::new();

    // Without optional param
    let response =
        service.ws_handle_message(r#"{"method": "search", "params": {"query": "test"}}"#);
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let items = json["result"].as_array().unwrap();
    assert_eq!(items.len(), 10); // default limit

    // With optional param
    let response = service
        .ws_handle_message(r#"{"method": "search", "params": {"query": "test", "limit": 3}}"#);
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let items = json["result"].as_array().unwrap();
    assert_eq!(items.len(), 3);
}

#[test]
fn test_ws_handle_create_item() {
    let service = TestService::new();
    let response =
        service.ws_handle_message(r#"{"method": "create_item", "params": {"name": "Test Item"}}"#);
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

// ============================================================================
// Async Method Tests
// ============================================================================

/// Service with async methods
#[derive(Clone)]
struct AsyncWsService;

#[ws(path = "/async-ws")]
impl AsyncWsService {
    /// Sync method - works with both sync and async handlers
    pub fn sync_echo(&self, message: String) -> String {
        format!("Sync: {}", message)
    }

    /// Async method - only works with async handler
    pub async fn async_fetch(&self, url: String) -> String {
        // Simulate async fetch
        format!("Fetched: {}", url)
    }

    /// Async method returning computed value
    pub async fn async_compute(&self, n: i64) -> i64 {
        n * n
    }
}

#[test]
fn test_ws_sync_method_with_sync_handler() {
    let service = AsyncWsService;

    // Sync method should work with sync handler
    let response =
        service.ws_handle_message(r#"{"method": "sync_echo", "params": {"message": "test"}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], "Sync: test");
}

#[test]
fn test_ws_async_method_with_sync_handler_returns_error() {
    let service = AsyncWsService;

    // Async method should return error with sync handler
    let response = service
        .ws_handle_message(r#"{"method": "async_fetch", "params": {"url": "http://example.com"}}"#);
    assert!(response.is_ok()); // Response is OK, but contains error in body

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not supported in sync context")
    );
}

#[tokio::test]
async fn test_ws_sync_method_with_async_handler() {
    let service = AsyncWsService;

    // Sync method should work with async handler
    let response = service
        .ws_handle_message_async(r#"{"method": "sync_echo", "params": {"message": "async test"}}"#)
        .await;
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], "Sync: async test");
}

#[tokio::test]
async fn test_ws_async_method_with_async_handler() {
    let service = AsyncWsService;

    // Async method should work with async handler
    let response = service
        .ws_handle_message_async(
            r#"{"method": "async_fetch", "params": {"url": "http://example.com"}}"#,
        )
        .await;
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], "Fetched: http://example.com");
}

#[tokio::test]
async fn test_ws_async_compute() {
    let service = AsyncWsService;

    let response = service
        .ws_handle_message_async(r#"{"method": "async_compute", "params": {"n": 7}}"#)
        .await;
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], 49);
}

#[tokio::test]
async fn test_ws_async_with_request_id() {
    let service = AsyncWsService;

    let response = service
        .ws_handle_message_async(
            r#"{"method": "async_compute", "params": {"n": 5}, "id": "req-123"}"#,
        )
        .await;
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], 25);
    assert_eq!(json["id"], "req-123");
}

#[test]
fn test_ws_openapi_paths_generated() {
    let paths = TestService::ws_openapi_paths();

    // Should have 1 path: GET /ws (WebSocket upgrade)
    assert_eq!(paths.len(), 1);

    let ws_path = &paths[0];
    assert_eq!(ws_path.path, "/ws");
    assert_eq!(ws_path.method, "get");
    assert!(
        ws_path
            .operation
            .summary
            .as_ref()
            .unwrap()
            .contains("WebSocket")
    );

    // Check that response includes 101 Switching Protocols
    assert!(ws_path.operation.responses.contains_key("101"));

    // Check extra contains WebSocket protocol info
    assert!(ws_path.operation.extra.contains_key("x-websocket-protocol"));
}

// ============================================================================
// Mount Point Tests
// ============================================================================

/// Child service for mount testing
#[derive(Clone)]
struct MathWs;

#[ws]
impl MathWs {
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
struct EchoWs;

#[ws]
impl EchoWs {
    /// Echo a message
    fn echo(&self, msg: String) -> String {
        format!("Echo: {}", msg)
    }
}

/// Parent with static mounts
#[derive(Clone)]
struct WsApp {
    math: MathWs,
    echo_svc: EchoWs,
}

#[ws]
impl WsApp {
    /// Ping health check
    fn ping(&self) -> String {
        "pong".to_string()
    }

    /// Mount math tools
    fn math(&self) -> &MathWs {
        &self.math
    }

    /// Mount echo tools
    fn echo_svc(&self) -> &EchoWs {
        &self.echo_svc
    }
}

#[test]
fn test_ws_static_mount_methods_listed() {
    let methods = WsApp::ws_methods();

    // Leaf method
    assert!(methods.contains(&"ping".to_string()));
    // Mounted methods (dot-separated)
    assert!(methods.contains(&"math.add".to_string()));
    assert!(methods.contains(&"math.double".to_string()));
    assert!(methods.contains(&"echo_svc.echo".to_string()));
}

#[test]
fn test_ws_static_mount_sync_dispatch() {
    let app = WsApp {
        math: MathWs,
        echo_svc: EchoWs,
    };

    // Dispatch to leaf
    let response = app.ws_handle_message(r#"{"method":"ping","params":{}}"#);
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], "pong");

    // Dispatch to mounted child
    let response = app.ws_handle_message(r#"{"method":"math.add","params":{"a":10,"b":5}}"#);
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], 15);

    // Dispatch to another mount
    let response = app.ws_handle_message(r#"{"method":"echo_svc.echo","params":{"msg":"hello"}}"#);
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], "Echo: hello");
}

#[tokio::test]
async fn test_ws_static_mount_async_dispatch() {
    let app = WsApp {
        math: MathWs,
        echo_svc: EchoWs,
    };

    let response = app
        .ws_handle_message_async(r#"{"method":"math.double","params":{"n":21}}"#)
        .await;
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], 42);
}

/// Slug mount: parent with parameterized child
#[derive(Clone)]
struct WsSlugApp {
    math: MathWs,
}

#[ws]
impl WsSlugApp {
    /// Access a calculator by ID
    fn calc(&self, id: String) -> &MathWs {
        let _ = &id;
        &self.math
    }
}

#[test]
fn test_ws_slug_mount_methods_listed() {
    let methods = WsSlugApp::ws_methods();

    assert!(methods.contains(&"calc.add".to_string()));
    assert!(methods.contains(&"calc.double".to_string()));
}

#[test]
fn test_ws_slug_mount_sync_dispatch() {
    let app = WsSlugApp { math: MathWs };

    let response =
        app.ws_handle_message(r#"{"method":"calc.add","params":{"id":"calc-1","a":3,"b":4}}"#);
    assert!(response.is_ok());
    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert_eq!(json["result"], 7);
}

/// WsMount trait test
#[test]
fn test_ws_mount_trait_implemented() {
    use server_less::WsMount;

    let methods = <MathWs as WsMount>::ws_mount_methods();
    assert_eq!(methods.len(), 2);
    assert!(methods.contains(&"add".to_string()));
    assert!(methods.contains(&"double".to_string()));

    let svc = MathWs;
    let result =
        <MathWs as WsMount>::ws_mount_dispatch(&svc, "add", serde_json::json!({"a": 1, "b": 2}));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), serde_json::json!(3));
}

// ============================================================================
// Hidden Method Tests
// ============================================================================

#[derive(Clone)]
struct HiddenWsService;

#[ws]
impl HiddenWsService {
    /// Public WS method
    pub fn public_ws(&self) -> String {
        "public".to_string()
    }

    /// Hidden WS method - callable but not listed
    #[server(hidden)]
    pub fn hidden_ws(&self, x: i32) -> i32 {
        x + 1
    }
}

#[test]
fn test_ws_hidden_method_not_in_methods_list() {
    let methods = HiddenWsService::ws_methods();
    // Public method appears
    assert!(methods.contains(&"public_ws".to_string()));
    // Hidden method does NOT appear
    assert!(!methods.contains(&"hidden_ws".to_string()));
}

#[test]
fn test_ws_hidden_method_still_dispatchable_sync() {
    let svc = HiddenWsService;
    let msg = r#"{"method": "hidden_ws", "params": {"x": 41}, "id": 1}"#;
    let result = svc.ws_handle_message(msg);
    assert!(result.is_ok());
    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(response["result"], serde_json::json!(42));
}

#[tokio::test]
async fn test_ws_hidden_method_still_dispatchable_async() {
    let svc = HiddenWsService;
    let msg = r#"{"method": "hidden_ws", "params": {"x": 9}, "id": 1}"#;
    let result = svc.ws_handle_message_async(msg).await;
    assert!(result.is_ok());
    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(response["result"], serde_json::json!(10));
}

#[test]
fn test_ws_hidden_method_not_in_openapi_summary() {
    let paths = HiddenWsService::ws_openapi_paths();
    assert_eq!(paths.len(), 1);
    let path = &paths[0];
    // The x-websocket-protocol extension lists methods; hidden must not appear
    let extra_str = serde_json::to_string(&path.operation.extra).unwrap();
    assert!(extra_str.contains("public_ws"), "public_ws must be in WS openapi extra");
    assert!(!extra_str.contains("hidden_ws"), "hidden_ws must not be in WS openapi extra");
}
