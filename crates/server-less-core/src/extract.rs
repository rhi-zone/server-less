//! Context and parameter extraction types.

use std::collections::HashMap;

/// Protocol-agnostic request context.
///
/// This provides access to protocol-specific metadata in a unified way:
/// - HTTP: headers, query params, cookies
/// - gRPC: metadata
/// - CLI: environment variables, config
/// - MCP: conversation context
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Key-value metadata (headers, env vars, etc.)
    metadata: HashMap<String, String>,
    /// The authenticated user ID, if any
    user_id: Option<String>,
    /// Request ID for tracing
    request_id: Option<String>,
}

impl Context {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create context with metadata
    pub fn with_metadata(metadata: HashMap<String, String>) -> Self {
        Self {
            metadata,
            ..Default::default()
        }
    }

    /// Get a metadata value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Set a metadata value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get the authenticated user ID
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Set the authenticated user ID
    pub fn set_user_id(&mut self, user_id: impl Into<String>) {
        self.user_id = Some(user_id.into());
    }

    /// Get the request ID
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Set the request ID
    pub fn set_request_id(&mut self, request_id: impl Into<String>) {
        self.request_id = Some(request_id.into());
    }

    /// Get all metadata
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    // HTTP-specific helpers

    /// Get an HTTP header value (case-insensitive lookup)
    pub fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.metadata
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }

    /// Get the Authorization header
    pub fn authorization(&self) -> Option<&str> {
        self.header("authorization")
    }

    /// Get the Content-Type header
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    // CLI-specific helpers

    /// Get an environment variable
    pub fn env(&self, name: &str) -> Option<&str> {
        self.get(&format!("env:{name}"))
    }
}

/// Marker type for parameters that should be extracted from the URL path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<T>(pub T);

impl<T> Path<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Marker type for parameters that should be extracted from query string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query<T>(pub T);

impl<T> Query<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Marker type for parameters that should be extracted from request body
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_metadata() {
        let mut ctx = Context::new();
        ctx.set("Content-Type", "application/json");
        ctx.set("X-Request-Id", "abc123");

        assert_eq!(ctx.get("Content-Type"), Some("application/json"));
        assert_eq!(ctx.header("content-type"), Some("application/json"));
    }

    #[test]
    fn test_context_user() {
        let mut ctx = Context::new();
        assert!(ctx.user_id().is_none());

        ctx.set_user_id("user_123");
        assert_eq!(ctx.user_id(), Some("user_123"));
    }
}
