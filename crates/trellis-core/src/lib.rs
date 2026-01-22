//! Core traits and types for trellis.
//!
//! This crate provides the foundational types that trellis macros generate code against.

pub mod error;
pub mod extract;

pub use error::{ErrorCode, ErrorResponse, IntoErrorCode};
pub use extract::Context;

/// Method metadata extracted from an impl block.
/// Used internally by macros but exposed for advanced use cases.
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

/// Parameter metadata
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
