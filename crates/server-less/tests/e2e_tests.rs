//! End-to-end tests that verify generated code works correctly.

#![allow(dead_code)]
#![allow(unused_variables)]
//!
//! These tests define a service with known behavior (the "reference implementation"),
//! apply macros to generate protocol handlers, and verify the results match.

use serde::{Deserialize, Serialize};
use server_less::{cli, http, mcp, ws};

// ============================================================================
// Reference Implementation
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Calculator {
    pub precision: u32,
}

impl Calculator {
    pub fn new(precision: u32) -> Self {
        Self { precision }
    }

    // Reference implementations - these define expected behavior
    pub fn ref_add(&self, a: i64, b: i64) -> i64 {
        a + b
    }

    pub fn ref_divide(&self, a: i64, b: i64) -> Result<i64, String> {
        if b == 0 {
            Err("Division by zero".to_string())
        } else {
            Ok(a / b)
        }
    }

    pub fn ref_find_sqrt(&self, n: i64) -> Option<i64> {
        if n < 0 {
            None
        } else {
            Some((n as f64).sqrt() as i64)
        }
    }

    pub fn ref_greet(&self, name: Option<String>) -> String {
        match name {
            Some(n) => format!("Hello, {}!", n),
            None => "Hello, stranger!".to_string(),
        }
    }
}

// ============================================================================
// MCP Service (applies macro)
// ============================================================================

#[derive(Clone)]
pub struct McpCalculator(Calculator);

#[mcp(namespace = "calc")]
impl McpCalculator {
    /// Add two numbers
    pub fn add(&self, a: i64, b: i64) -> i64 {
        self.0.ref_add(a, b)
    }

    /// Divide two numbers
    pub fn divide(&self, a: i64, b: i64) -> Result<i64, String> {
        self.0.ref_divide(a, b)
    }

    /// Find square root (returns None for negative)
    pub fn find_sqrt(&self, n: i64) -> Option<i64> {
        self.0.ref_find_sqrt(n)
    }

    /// Greet someone
    pub fn greet(&self, name: Option<String>) -> String {
        self.0.ref_greet(name)
    }
}

// ============================================================================
// WebSocket Service (applies macro)
// ============================================================================

#[derive(Clone)]
pub struct WsCalculator(Calculator);

#[ws(path = "/ws")]
impl WsCalculator {
    /// Add two numbers
    pub fn add(&self, a: i64, b: i64) -> i64 {
        self.0.ref_add(a, b)
    }

    /// Divide two numbers
    pub fn divide(&self, a: i64, b: i64) -> Result<i64, String> {
        self.0.ref_divide(a, b)
    }

    /// Find square root
    pub fn find_sqrt(&self, n: i64) -> Option<i64> {
        self.0.ref_find_sqrt(n)
    }

    /// Greet someone
    pub fn greet(&self, name: Option<String>) -> String {
        self.0.ref_greet(name)
    }
}

// ============================================================================
// HTTP Service (applies macro)
// ============================================================================

#[derive(Clone)]
pub struct HttpCalculator(Calculator);

#[http(prefix = "/api")]
impl HttpCalculator {
    /// Add two numbers
    pub fn create_sum(&self, a: i64, b: i64) -> i64 {
        self.0.ref_add(a, b)
    }

    /// Get square root
    pub fn get_sqrt(&self, n: i64) -> Option<i64> {
        self.0.ref_find_sqrt(n)
    }
}

// ============================================================================
// CLI Service (applies macro)
// ============================================================================

#[derive(Clone)]
pub struct CliCalculator(Calculator);

#[cli(name = "calc", version = "1.0.0", about = "Calculator CLI")]
impl CliCalculator {
    /// Add two numbers
    pub fn add(&self, a: i64, b: i64) -> i64 {
        self.0.ref_add(a, b)
    }

    /// Greet someone
    pub fn greet(&self, name: Option<String>) -> String {
        self.0.ref_greet(name)
    }
}

// ============================================================================
// MCP E2E Tests
// ============================================================================

#[test]
fn test_mcp_add_matches_reference() {
    let calc = McpCalculator(Calculator::new(2));

    // Call via MCP
    let result = calc.mcp_call("calc_add", serde_json::json!({"a": 10, "b": 5}));
    assert!(result.is_ok());

    // Verify matches reference
    let mcp_result: i64 = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(mcp_result, calc.0.ref_add(10, 5));
}

#[test]
fn test_mcp_divide_ok_matches_reference() {
    let calc = McpCalculator(Calculator::new(2));

    let result = calc.mcp_call("calc_divide", serde_json::json!({"a": 20, "b": 4}));
    assert!(result.is_ok());

    let mcp_result: i64 = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(mcp_result, calc.0.ref_divide(20, 4).unwrap());
}

#[test]
fn test_mcp_divide_err_matches_reference() {
    let calc = McpCalculator(Calculator::new(2));

    let result = calc.mcp_call("calc_divide", serde_json::json!({"a": 10, "b": 0}));

    // Should be error, matching reference
    assert!(result.is_err());
    assert!(calc.0.ref_divide(10, 0).is_err());
}

#[test]
fn test_mcp_option_some_matches_reference() {
    let calc = McpCalculator(Calculator::new(2));

    let result = calc.mcp_call("calc_find_sqrt", serde_json::json!({"n": 16}));
    assert!(result.is_ok());

    let mcp_result: i64 = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(mcp_result, calc.0.ref_find_sqrt(16).unwrap());
}

#[test]
fn test_mcp_option_none_matches_reference() {
    let calc = McpCalculator(Calculator::new(2));

    let result = calc.mcp_call("calc_find_sqrt", serde_json::json!({"n": -1}));
    assert!(result.is_ok());

    // Should be null for None
    assert!(result.unwrap().is_null());
    assert!(calc.0.ref_find_sqrt(-1).is_none());
}

#[test]
fn test_mcp_optional_param_provided() {
    let calc = McpCalculator(Calculator::new(2));

    let result = calc.mcp_call("calc_greet", serde_json::json!({"name": "Alice"}));
    assert!(result.is_ok());

    let mcp_result: String = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(mcp_result, calc.0.ref_greet(Some("Alice".to_string())));
}

#[test]
fn test_mcp_optional_param_missing() {
    let calc = McpCalculator(Calculator::new(2));

    let result = calc.mcp_call("calc_greet", serde_json::json!({}));
    assert!(result.is_ok());

    let mcp_result: String = serde_json::from_value(result.unwrap()).unwrap();
    assert_eq!(mcp_result, calc.0.ref_greet(None));
}

// ============================================================================
// WebSocket E2E Tests
// ============================================================================

#[test]
fn test_ws_add_matches_reference() {
    let calc = WsCalculator(Calculator::new(2));

    let response = calc.ws_handle_message(r#"{"method": "add", "params": {"a": 7, "b": 3}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let ws_result = json["result"].as_i64().unwrap();
    assert_eq!(ws_result, calc.0.ref_add(7, 3));
}

#[test]
fn test_ws_divide_ok_matches_reference() {
    let calc = WsCalculator(Calculator::new(2));

    let response = calc.ws_handle_message(r#"{"method": "divide", "params": {"a": 100, "b": 5}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let ws_result = json["result"].as_i64().unwrap();
    assert_eq!(ws_result, calc.0.ref_divide(100, 5).unwrap());
}

#[test]
fn test_ws_divide_err_matches_reference() {
    let calc = WsCalculator(Calculator::new(2));

    let response = calc.ws_handle_message(r#"{"method": "divide", "params": {"a": 10, "b": 0}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    // Should have error field
    assert!(json["error"].is_object());
    assert!(calc.0.ref_divide(10, 0).is_err());
}

#[test]
fn test_ws_option_some_matches_reference() {
    let calc = WsCalculator(Calculator::new(2));

    let response = calc.ws_handle_message(r#"{"method": "find_sqrt", "params": {"n": 25}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let ws_result = json["result"].as_i64().unwrap();
    assert_eq!(ws_result, calc.0.ref_find_sqrt(25).unwrap());
}

#[test]
fn test_ws_option_none_matches_reference() {
    let calc = WsCalculator(Calculator::new(2));

    let response = calc.ws_handle_message(r#"{"method": "find_sqrt", "params": {"n": -5}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    assert!(json["result"].is_null());
    assert!(calc.0.ref_find_sqrt(-5).is_none());
}

#[test]
fn test_ws_optional_param_provided() {
    let calc = WsCalculator(Calculator::new(2));

    let response = calc.ws_handle_message(r#"{"method": "greet", "params": {"name": "Bob"}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let ws_result = json["result"].as_str().unwrap();
    assert_eq!(ws_result, calc.0.ref_greet(Some("Bob".to_string())));
}

#[test]
fn test_ws_optional_param_missing() {
    let calc = WsCalculator(Calculator::new(2));

    let response = calc.ws_handle_message(r#"{"method": "greet", "params": {}}"#);
    assert!(response.is_ok());

    let json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
    let ws_result = json["result"].as_str().unwrap();
    assert_eq!(ws_result, calc.0.ref_greet(None));
}

// ============================================================================
// HTTP E2E Tests (structure only - full HTTP testing would need test client)
// ============================================================================

#[test]
fn test_http_router_created() {
    let calc = HttpCalculator(Calculator::new(2));
    let _router = calc.http_router();
}

#[test]
fn test_http_openapi_has_endpoints() {
    let spec = HttpCalculator::openapi_spec();
    let paths = spec["paths"].as_object().unwrap();

    // Should have our endpoints (create_sum -> POST /sums, get_sqrt -> GET /sqrts)
    assert!(
        paths.contains_key("/api/sums"),
        "Expected /api/sums, got: {:?}",
        paths.keys().collect::<Vec<_>>()
    );
    // get_sqrt takes n which is not an ID, so it's /sqrts not /sqrts/{id}
    assert!(
        paths.contains_key("/api/sqrts"),
        "Expected /api/sqrts, got: {:?}",
        paths.keys().collect::<Vec<_>>()
    );
}

// ============================================================================
// CLI E2E Tests
// ============================================================================

#[test]
fn test_cli_command_created() {
    let cmd = CliCalculator::cli_command();
    assert_eq!(cmd.get_name(), "calc");
}

#[test]
fn test_cli_has_subcommands() {
    let cmd = CliCalculator::cli_command();
    let subcommands: Vec<_> = cmd.get_subcommands().map(|c| c.get_name()).collect();

    assert!(subcommands.contains(&"add"));
    assert!(subcommands.contains(&"greet"));
}

// ============================================================================
// Cross-Protocol Consistency Tests
// ============================================================================

#[test]
fn test_mcp_ws_produce_same_results() {
    let mcp_calc = McpCalculator(Calculator::new(2));
    let ws_calc = WsCalculator(Calculator::new(2));

    // Test add
    let mcp_result = mcp_calc
        .mcp_call("calc_add", serde_json::json!({"a": 15, "b": 7}))
        .unwrap();
    let ws_response = ws_calc
        .ws_handle_message(r#"{"method": "add", "params": {"a": 15, "b": 7}}"#)
        .unwrap();
    let ws_json: serde_json::Value = serde_json::from_str(&ws_response).unwrap();

    assert_eq!(mcp_result.as_i64(), ws_json["result"].as_i64());
}

#[test]
fn test_all_protocols_agree_on_sqrt() {
    let ref_calc = Calculator::new(2);
    let mcp_calc = McpCalculator(ref_calc.clone());
    let ws_calc = WsCalculator(ref_calc.clone());

    let n = 64;
    let expected = ref_calc.ref_find_sqrt(n).unwrap();

    // MCP
    let mcp_result = mcp_calc
        .mcp_call("calc_find_sqrt", serde_json::json!({"n": n}))
        .unwrap();
    assert_eq!(mcp_result.as_i64().unwrap(), expected);

    // WS
    let ws_response = ws_calc
        .ws_handle_message(&format!(
            r#"{{"method": "find_sqrt", "params": {{"n": {}}}}}"#,
            n
        ))
        .unwrap();
    let ws_json: serde_json::Value = serde_json::from_str(&ws_response).unwrap();
    assert_eq!(ws_json["result"].as_i64().unwrap(), expected);
}
