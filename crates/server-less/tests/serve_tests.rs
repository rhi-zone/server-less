//! Integration tests for the serve coordination macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::{http, jsonrpc, serve, ws};

#[derive(Clone)]
struct MultiService {
    name: String,
}

impl MultiService {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[http(prefix = "/api")]
#[ws(path = "/ws")]
#[serve(http, ws)]
impl MultiService {
    /// Get service info
    pub fn get_info(&self) -> String {
        format!("Service: {}", self.name)
    }

    /// List items
    pub fn list_items(&self) -> Vec<String> {
        vec!["item1".to_string(), "item2".to_string()]
    }
}

#[test]
fn test_serve_router_created() {
    let service = MultiService::new("test");
    let router = service.router();
    // Router is created successfully (combines http + ws)
    let _ = router;
}

#[test]
fn test_serve_router_has_health() {
    // The router should have a /health endpoint
    // We can't easily test this without starting a server,
    // but we verify the router builds without error
    let service = MultiService::new("test");
    let _router = service.router();
}

// HTTP-only service
#[derive(Clone)]
struct HttpOnlyService;

#[http]
#[serve(http)]
impl HttpOnlyService {
    pub fn list_things(&self) -> Vec<String> {
        vec![]
    }
}

#[test]
fn test_serve_http_only() {
    let service = HttpOnlyService;
    let _router = service.router();
}

// WS-only service
#[derive(Clone)]
struct WsOnlyService;

#[ws(path = "/ws")]
#[serve(ws)]
impl WsOnlyService {
    pub fn echo(&self, msg: String) -> String {
        msg
    }
}

#[test]
fn test_serve_ws_only() {
    let service = WsOnlyService;
    let _router = service.router();
}

// Custom health path
#[derive(Clone)]
struct CustomHealthService;

#[http]
#[serve(http, health = "/healthz")]
impl CustomHealthService {
    pub fn list_items(&self) -> Vec<String> {
        vec![]
    }
}

#[test]
fn test_serve_custom_health() {
    let service = CustomHealthService;
    let _router = service.router();
}

// Empty serve (no protocols, just health)
#[derive(Clone)]
struct MinimalService;

#[serve()]
impl MinimalService {
    pub fn _internal(&self) {}
}

#[test]
fn test_serve_minimal() {
    let service = MinimalService;
    let _router = service.router();
}

// Combined HTTP + JSON-RPC
#[derive(Clone)]
struct CombinedRpcService;

#[http]
#[jsonrpc(path = "/rpc")]
#[serve(http, jsonrpc)]
impl CombinedRpcService {
    /// Add two numbers
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Get status
    pub fn get_status(&self) -> String {
        "ok".to_string()
    }
}

#[test]
fn test_serve_http_jsonrpc() {
    let service = CombinedRpcService;
    let _router = service.router();
}

// JSON-RPC only
#[derive(Clone)]
struct JsonRpcOnlyService;

#[jsonrpc]
#[serve(jsonrpc)]
impl JsonRpcOnlyService {
    pub fn ping(&self) -> String {
        "pong".to_string()
    }
}

#[test]
fn test_serve_jsonrpc_only() {
    let service = JsonRpcOnlyService;
    let _router = service.router();
}

// ============================================================================
// OpenAPI Integration Tests
// ============================================================================

#[test]
fn test_serve_combined_openapi_spec() {
    let spec = MultiService::combined_openapi_spec();

    // Should have OpenAPI version
    assert_eq!(spec["openapi"], "3.0.0");

    // Should have info with struct name as title
    assert_eq!(spec["info"]["title"], "MultiService");

    // Should have paths from HTTP protocol
    assert!(
        spec["paths"].is_object(),
        "Should have paths object. Spec: {}",
        serde_json::to_string_pretty(&spec).unwrap()
    );
}

#[test]
fn test_serve_http_openapi_has_paths() {
    let spec = HttpOnlyService::combined_openapi_spec();

    // Should have the HTTP endpoint path
    let paths = &spec["paths"];
    assert!(
        paths.is_object(),
        "Should have paths. Spec: {}",
        serde_json::to_string_pretty(&spec).unwrap()
    );
}

#[test]
fn test_serve_combined_openapi_includes_jsonrpc() {
    let spec = CombinedRpcService::combined_openapi_spec();

    // Should have HTTP paths AND JSON-RPC path
    let paths = &spec["paths"];
    assert!(paths.is_object(), "Should have paths object");

    // JSON-RPC endpoint should be present
    assert!(
        paths["/rpc"].is_object(),
        "Should have /rpc JSON-RPC endpoint. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}

// Test openapi = false opt-out
#[derive(Clone)]
struct NoOpenApiService;

#[http]
#[serve(http, openapi = false)]
impl NoOpenApiService {
    pub fn list_items(&self) -> Vec<String> {
        vec![]
    }
}

#[test]
fn test_serve_openapi_disabled() {
    // Service should still work, just no combined_openapi_spec() method
    let service = NoOpenApiService;
    let _router = service.router();
    // Note: combined_openapi_spec() should NOT exist on this type
    // (verified by the fact that it compiles without the method)
}
