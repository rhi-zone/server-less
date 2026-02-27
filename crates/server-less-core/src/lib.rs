//! Core traits and types for server-less.
//!
//! This crate provides the foundational types that server-less macros generate code against.

pub mod error;
pub mod extract;

pub use error::{ErrorCode, ErrorResponse, IntoErrorCode, SchemaValidationError};
pub use extract::Context;

#[cfg(feature = "ws")]
pub use extract::WsSender;

/// Trait for types that can be mounted as CLI subcommand groups.
///
/// Implemented automatically by `#[cli]` on an impl block. Allows nested
/// composition: a parent CLI can mount a child's commands as a subcommand group.
#[cfg(feature = "cli")]
pub trait CliSubcommand {
    /// Build the clap Command tree for this type's subcommands.
    fn cli_command() -> ::clap::Command;

    /// Dispatch a matched subcommand to the appropriate method.
    fn cli_dispatch(&self, matches: &::clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>>;
}

/// Trait for types that can be mounted as MCP tool namespaces.
///
/// Implemented automatically by `#[mcp]` on an impl block. Allows nested
/// composition: a parent MCP server can mount a child's tools with a name prefix.
#[cfg(feature = "mcp")]
pub trait McpNamespace {
    /// Get tool definitions for this namespace.
    fn mcp_namespace_tools() -> Vec<serde_json::Value>;

    /// Get tool names for this namespace (without prefix).
    fn mcp_namespace_tool_names() -> Vec<String>;

    /// Call a tool by name (sync). Returns error for async-only methods.
    fn mcp_namespace_call(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String>;

    /// Call a tool by name (async).
    fn mcp_namespace_call_async(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
}

/// Trait for types that can be mounted as JSON-RPC method namespaces.
///
/// Implemented automatically by `#[jsonrpc]` on an impl block. Allows nested
/// composition: a parent JSON-RPC server can mount a child's methods with a dot-separated prefix.
#[cfg(feature = "jsonrpc")]
pub trait JsonRpcMount {
    /// Get method names for this mount (without prefix).
    fn jsonrpc_mount_methods() -> Vec<String>;

    /// Dispatch a method call (async).
    fn jsonrpc_mount_dispatch(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
}

/// Trait for types that can be mounted as WebSocket method namespaces.
///
/// Implemented automatically by `#[ws]` on an impl block. Allows nested
/// composition: a parent WebSocket server can mount a child's methods with a dot-separated prefix.
#[cfg(feature = "ws")]
pub trait WsMount {
    /// Get method names for this mount (without prefix).
    fn ws_mount_methods() -> Vec<String>;

    /// Dispatch a method call (sync). Returns error for async-only methods.
    fn ws_mount_dispatch(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String>;

    /// Dispatch a method call (async).
    fn ws_mount_dispatch_async(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
}

/// Trait for types that can be mounted as HTTP route groups.
///
/// Implemented automatically by `#[http]` on an impl block. Allows nested
/// composition: a parent HTTP server can mount a child's routes under a path prefix.
#[cfg(feature = "http")]
pub trait HttpMount: Send + Sync + 'static {
    /// Build an axum Router for this mount's routes.
    fn http_mount_router(self: ::std::sync::Arc<Self>) -> ::axum::Router;

    /// Get OpenAPI path definitions for this mount.
    fn http_mount_openapi_paths() -> Vec<crate::HttpMountPathInfo>
    where
        Self: Sized;
}

/// Simplified path info for HttpMount composition.
#[cfg(feature = "http")]
#[derive(Debug, Clone)]
pub struct HttpMountPathInfo {
    /// The path (relative to the mount point).
    pub path: String,
    /// The HTTP method (get, post, etc.).
    pub method: String,
    /// Summary text.
    pub summary: Option<String>,
}

/// Format CLI output according to output flags.
///
/// - `jsonl`: one JSON object per line for array values
/// - `json`: machine-readable JSON (no whitespace)
/// - `jq`: filter through the `jq` binary
/// - Default: pretty-printed JSON
#[cfg(feature = "cli")]
pub fn cli_format_output(
    value: serde_json::Value,
    jsonl: bool,
    json: bool,
    jq: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(filter) = jq {
        let input = serde_json::to_string(&value)?;
        let output = std::process::Command::new("jq")
            .arg(filter)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(input.as_bytes())?;
                }
                child.wait_with_output()
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("jq failed: {stderr}").into());
        }
        Ok(String::from_utf8(output.stdout)?.trim_end().to_string())
    } else if jsonl {
        match value {
            serde_json::Value::Array(items) => {
                let lines: Vec<String> = items
                    .iter()
                    .map(serde_json::to_string)
                    .collect::<Result<_, _>>()?;
                Ok(lines.join("\n"))
            }
            other => Ok(serde_json::to_string(&other)?),
        }
    } else if json {
        Ok(serde_json::to_string(&value)?)
    } else {
        Ok(serde_json::to_string_pretty(&value)?)
    }
}

/// Runtime method metadata with string-based types.
///
/// This is a simplified, serialization-friendly representation of method
/// information intended for runtime introspection and tooling. Types are
/// stored as strings rather than `syn` AST nodes.
///
/// **Not to be confused with [`server_less_parse::MethodInfo`]**, which is
/// the richer, `syn`-based representation used internally by proc macros
/// during code generation. The parse version retains full type information
/// (`syn::Type`, `syn::Ident`) and supports `#[param(...)]` attributes.
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Method name (e.g., "create_user")
    pub name: String,
    /// Documentation string from /// comments
    pub docs: Option<String>,
    /// Parameter names and their type strings
    pub params: Vec<ParamInfo>,
    /// Return type string
    pub return_type: String,
    /// Whether the method is async
    pub is_async: bool,
    /// Whether the return type is a Stream
    pub is_streaming: bool,
    /// Whether the return type is `Option<T>`
    pub is_optional: bool,
    /// Whether the return type is `Result<T, E>`
    pub is_result: bool,
}

/// Runtime parameter metadata with string-based types.
///
/// See [`MethodInfo`] for the relationship between this type and
/// `server_less_parse::ParamInfo`.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Parameter name
    pub name: String,
    /// Type as string
    pub ty: String,
    /// Whether this is an `Option<T>`
    pub is_optional: bool,
    /// Whether this looks like an ID parameter (ends with _id or is named id)
    pub is_id: bool,
}

/// HTTP method inferred from function name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    /// Infer HTTP method from function name prefix
    pub fn infer_from_name(name: &str) -> Self {
        if name.starts_with("get_")
            || name.starts_with("fetch_")
            || name.starts_with("read_")
            || name.starts_with("list_")
            || name.starts_with("find_")
            || name.starts_with("search_")
        {
            HttpMethod::Get
        } else if name.starts_with("create_")
            || name.starts_with("add_")
            || name.starts_with("new_")
        {
            HttpMethod::Post
        } else if name.starts_with("update_") || name.starts_with("set_") {
            HttpMethod::Put
        } else if name.starts_with("patch_") || name.starts_with("modify_") {
            HttpMethod::Patch
        } else if name.starts_with("delete_") || name.starts_with("remove_") {
            HttpMethod::Delete
        } else {
            // Default to POST for RPC-style methods
            HttpMethod::Post
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }
}

/// Infer URL path from method name
pub fn infer_path(method_name: &str, http_method: HttpMethod) -> String {
    // Strip common prefixes to get the resource name
    let resource = method_name
        .strip_prefix("get_")
        .or_else(|| method_name.strip_prefix("fetch_"))
        .or_else(|| method_name.strip_prefix("read_"))
        .or_else(|| method_name.strip_prefix("list_"))
        .or_else(|| method_name.strip_prefix("find_"))
        .or_else(|| method_name.strip_prefix("search_"))
        .or_else(|| method_name.strip_prefix("create_"))
        .or_else(|| method_name.strip_prefix("add_"))
        .or_else(|| method_name.strip_prefix("new_"))
        .or_else(|| method_name.strip_prefix("update_"))
        .or_else(|| method_name.strip_prefix("set_"))
        .or_else(|| method_name.strip_prefix("patch_"))
        .or_else(|| method_name.strip_prefix("modify_"))
        .or_else(|| method_name.strip_prefix("delete_"))
        .or_else(|| method_name.strip_prefix("remove_"))
        .unwrap_or(method_name);

    // Pluralize for collection endpoints
    let path_resource = if resource.ends_with('s') {
        resource.to_string()
    } else {
        format!("{resource}s")
    };

    match http_method {
        // Collection operations
        HttpMethod::Post => format!("/{path_resource}"),
        HttpMethod::Get
            if method_name.starts_with("list_")
                || method_name.starts_with("search_")
                || method_name.starts_with("find_") =>
        {
            format!("/{path_resource}")
        }
        // Single resource operations
        HttpMethod::Get | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Delete => {
            format!("/{path_resource}/{{id}}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_inference() {
        assert_eq!(HttpMethod::infer_from_name("get_user"), HttpMethod::Get);
        assert_eq!(HttpMethod::infer_from_name("list_users"), HttpMethod::Get);
        assert_eq!(HttpMethod::infer_from_name("create_user"), HttpMethod::Post);
        assert_eq!(HttpMethod::infer_from_name("update_user"), HttpMethod::Put);
        assert_eq!(
            HttpMethod::infer_from_name("delete_user"),
            HttpMethod::Delete
        );
        assert_eq!(
            HttpMethod::infer_from_name("do_something"),
            HttpMethod::Post
        ); // RPC fallback
    }

    #[test]
    fn test_path_inference() {
        assert_eq!(infer_path("create_user", HttpMethod::Post), "/users");
        assert_eq!(infer_path("get_user", HttpMethod::Get), "/users/{id}");
        assert_eq!(infer_path("list_users", HttpMethod::Get), "/users");
        assert_eq!(infer_path("delete_user", HttpMethod::Delete), "/users/{id}");
    }
}
