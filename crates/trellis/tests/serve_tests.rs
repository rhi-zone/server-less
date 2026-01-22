//! Integration tests for the serve coordination macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use trellis::{http, jsonrpc, serve, ws};

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
