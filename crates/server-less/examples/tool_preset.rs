//! Example demonstrating the #[tool] blessed preset.
//!
//! `#[tool]` = `#[mcp]` + `#[jsonschema]` in one attribute.
//!
//! ```bash
//! cargo run --example tool_preset
//! ```

use server_less::tool;

pub struct FileTools;

#[tool(namespace = "file")]
impl FileTools {
    /// Read a file from disk
    pub fn read_file(&self, path: String) -> String {
        format!("(contents of {})", path)
    }

    /// Write content to a file
    pub fn write_file(&self, path: String, content: String) -> bool {
        println!("Writing {} bytes to {}", content.len(), path);
        true
    }

    /// List files in a directory
    pub fn list_dir(&self, path: String) -> Vec<String> {
        vec![format!("{}/file1.txt", path), format!("{}/file2.txt", path)]
    }
}

fn main() {
    let tools = FileTools;

    // Show MCP tool definitions
    println!("MCP tools:");
    for tool in FileTools::mcp_tools() {
        println!("  {}", serde_json::to_string_pretty(&tool).unwrap());
    }

    // Show JSON Schema
    println!("\nJSON Schema:");
    println!(
        "{}",
        serde_json::to_string_pretty(&FileTools::json_schema()).unwrap()
    );

    // Call a tool
    let result = tools
        .mcp_call("file_read_file", serde_json::json!({"path": "/etc/hosts"}))
        .unwrap();
    println!("\nfile_read_file result: {}", result);
}
