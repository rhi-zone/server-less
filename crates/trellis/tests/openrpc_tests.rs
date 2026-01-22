//! Integration tests for the OpenRPC specification generation macro.

use trellis::openrpc;

#[derive(Clone)]
struct Calculator;

#[openrpc(title = "Calculator API", version = "2.0.0")]
impl Calculator {
    /// Add two numbers together
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Subtract b from a
    pub fn subtract(&self, a: i32, b: i32) -> i32 {
        a - b
    }

    /// Echo a message back
    pub fn echo(&self, message: String) -> String {
        message
    }

    /// Get a boolean value
    pub fn is_ready(&self) -> bool {
        true
    }
}

#[test]
fn test_openrpc_spec_structure() {
    let spec = Calculator::openrpc_spec();

    assert_eq!(spec["openrpc"], "1.0.0");
    assert_eq!(spec["info"]["title"], "Calculator API");
    assert_eq!(spec["info"]["version"], "2.0.0");
    assert!(spec["methods"].is_array());
}

#[test]
fn test_openrpc_methods_present() {
    let spec = Calculator::openrpc_spec();
    let methods = spec["methods"].as_array().unwrap();

    let method_names: Vec<&str> = methods.iter().filter_map(|m| m["name"].as_str()).collect();

    assert!(method_names.contains(&"add"));
    assert!(method_names.contains(&"subtract"));
    assert!(method_names.contains(&"echo"));
    assert!(method_names.contains(&"isReady")); // camelCase
}

#[test]
fn test_openrpc_method_params() {
    let spec = Calculator::openrpc_spec();
    let methods = spec["methods"].as_array().unwrap();

    let add_method = methods.iter().find(|m| m["name"] == "add").unwrap();
    let params = add_method["params"].as_array().unwrap();

    assert_eq!(params.len(), 2);
    assert_eq!(params[0]["name"], "a");
    assert_eq!(params[1]["name"], "b");
    assert_eq!(params[0]["schema"]["type"], "integer");
}

#[test]
fn test_openrpc_method_result() {
    let spec = Calculator::openrpc_spec();
    let methods = spec["methods"].as_array().unwrap();

    let echo_method = methods.iter().find(|m| m["name"] == "echo").unwrap();
    assert_eq!(echo_method["result"]["schema"]["type"], "string");

    let ready_method = methods.iter().find(|m| m["name"] == "isReady").unwrap();
    assert_eq!(ready_method["result"]["schema"]["type"], "boolean");
}

#[test]
fn test_openrpc_method_description() {
    let spec = Calculator::openrpc_spec();
    let methods = spec["methods"].as_array().unwrap();

    let add_method = methods.iter().find(|m| m["name"] == "add").unwrap();
    assert_eq!(add_method["description"], "Add two numbers together");
}

#[test]
fn test_openrpc_json_output() {
    let json = Calculator::openrpc_json();

    assert!(json.contains("\"openrpc\""));
    assert!(json.contains("Calculator API"));
    assert!(json.contains("\"methods\""));
}

// Test default title and version
#[derive(Clone)]
struct SimpleService;

#[openrpc]
impl SimpleService {
    pub fn ping(&self) -> String {
        "pong".to_string()
    }
}

#[test]
fn test_openrpc_defaults() {
    let spec = SimpleService::openrpc_spec();

    // Default title should be struct name
    assert_eq!(spec["info"]["title"], "SimpleService");
    // Default version
    assert_eq!(spec["info"]["version"], "1.0.0");
}

// Test with optional params
#[derive(Clone)]
struct OptionalService;

#[openrpc]
impl OptionalService {
    pub fn search(&self, query: String, limit: Option<i32>) -> Vec<String> {
        vec![]
    }
}

#[test]
fn test_openrpc_optional_params() {
    let spec = OptionalService::openrpc_spec();
    let methods = spec["methods"].as_array().unwrap();

    let search = methods.iter().find(|m| m["name"] == "search").unwrap();
    let params = search["params"].as_array().unwrap();

    // query is required
    assert_eq!(params[0]["required"], true);
    // limit is optional
    assert_eq!(params[1]["required"], false);
}

// Combined with jsonrpc
#[derive(Clone)]
struct CombinedService;

#[trellis::jsonrpc]
#[openrpc(title = "Combined Service")]
impl CombinedService {
    pub fn greet(&self, name: String) -> String {
        format!("Hello, {}!", name)
    }
}

#[test]
fn test_openrpc_with_jsonrpc() {
    // Both macros work together
    let spec = CombinedService::openrpc_spec();
    assert_eq!(spec["info"]["title"], "Combined Service");

    let methods = CombinedService::jsonrpc_methods();
    assert!(methods.contains(&"greet"));
}
