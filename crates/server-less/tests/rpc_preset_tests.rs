//! Integration tests for the #[rpc] blessed preset.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde_json::json;
use server_less::rpc;

// Basic RPC preset (zero-config)
#[derive(Clone)]
struct BasicCalc;

#[rpc]
impl BasicCalc {
    /// Add two numbers
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Multiply two numbers
    pub fn multiply(&self, a: i32, b: i32) -> i32 {
        a * b
    }
}

#[test]
fn test_rpc_basic_methods() {
    let methods = BasicCalc::jsonrpc_methods();
    assert!(methods.contains(&"add".to_string()));
    assert!(methods.contains(&"multiply".to_string()));
}

#[tokio::test]
async fn test_rpc_basic_handle() {
    let calc = BasicCalc;
    let request = json!({
        "jsonrpc": "2.0",
        "method": "add",
        "params": {"a": 2, "b": 3},
        "id": 1
    });
    let response = calc.jsonrpc_handle(request).await;
    assert_eq!(response["result"], 5);
}

#[test]
fn test_rpc_basic_openrpc_spec() {
    let spec = BasicCalc::openrpc_spec();
    assert_eq!(spec["openrpc"], "1.0.0");
}

#[test]
fn test_rpc_basic_router() {
    let calc = BasicCalc;
    let _router = calc.router();
}

// RPC with custom path
#[derive(Clone)]
struct PathCalc;

#[rpc(path = "/api/calc")]
impl PathCalc {
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[test]
fn test_rpc_custom_path() {
    let calc = PathCalc;
    let _router = calc.router();
}

// RPC with openrpc disabled
#[derive(Clone)]
struct NoSpecCalc;

#[rpc(openrpc = false)]
impl NoSpecCalc {
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[test]
fn test_rpc_no_openrpc() {
    let methods = NoSpecCalc::jsonrpc_methods();
    assert!(methods.contains(&"add".to_string()));
    // openrpc_spec() should NOT be available — verified by compilation
}

// RPC with all options
#[derive(Clone)]
struct FullCalc;

#[rpc(path = "/rpc", openrpc = true, health = "/healthz")]
impl FullCalc {
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[test]
fn test_rpc_full_options() {
    let calc = FullCalc;
    let _router = calc.router();
    let spec = FullCalc::openrpc_spec();
    assert_eq!(spec["openrpc"], "1.0.0");
}
