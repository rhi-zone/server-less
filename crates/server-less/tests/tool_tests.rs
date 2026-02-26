//! Integration tests for the #[tool] blessed preset.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::tool;

// Basic tool preset (zero-config)
struct BasicTools;

#[tool]
impl BasicTools {
    /// Read a file
    pub fn read_file(&self, path: String) -> String {
        format!("contents of {}", path)
    }

    /// Write a file
    pub fn write_file(&self, path: String, content: String) -> bool {
        true
    }
}

#[test]
fn test_tool_basic_mcp_tools() {
    let tools = BasicTools::mcp_tools();
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_tool_basic_mcp_call() {
    let svc = BasicTools;
    let result = svc.mcp_call("read_file", serde_json::json!({"path": "test.txt"}));
    assert!(result.is_ok());
    assert!(result.unwrap().as_str().unwrap().contains("test.txt"));
}

#[test]
fn test_tool_basic_jsonschema() {
    let schema = BasicTools::json_schema();
    assert!(schema.is_object());
}

// Tool with namespace
struct NamespacedTools;

#[tool(namespace = "file")]
impl NamespacedTools {
    pub fn read(&self, path: String) -> String {
        format!("contents of {}", path)
    }
}

#[test]
fn test_tool_namespace_mcp_tools() {
    let tools = NamespacedTools::mcp_tools();
    assert_eq!(tools.len(), 1);
    let name = tools[0]["name"].as_str().unwrap();
    assert!(
        name.starts_with("file_"),
        "Tool name should be namespaced: {}",
        name
    );
}

#[test]
fn test_tool_namespace_mcp_call() {
    let svc = NamespacedTools;
    let result = svc.mcp_call("file_read", serde_json::json!({"path": "test.txt"}));
    assert!(result.is_ok());
}

// Tool with jsonschema disabled
struct NoSchemaTools;

#[tool(jsonschema = false)]
impl NoSchemaTools {
    pub fn ping(&self) -> String {
        "pong".to_string()
    }
}

#[test]
fn test_tool_no_jsonschema() {
    let tools = NoSchemaTools::mcp_tools();
    assert_eq!(tools.len(), 1);
    // jsonschema() should NOT be available — verified by compilation
}

// Tool with all options
struct FullTools;

#[tool(namespace = "myapp", jsonschema = true)]
impl FullTools {
    pub fn do_thing(&self, input: String) -> String {
        input
    }
}

#[test]
fn test_tool_full_options() {
    let tools = FullTools::mcp_tools();
    assert_eq!(tools.len(), 1);
    let schema = FullTools::json_schema();
    assert!(schema.is_object());
}
