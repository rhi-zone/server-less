//! Example: HTTP and MCP mount points — composing services into a parent.
//!
//! Mount points let you nest one service inside another via methods that return
//! `&ChildService`. The parent's generated router / tool list automatically
//! includes everything from the child, scoped under the mount method's name.
//!
//! ## HTTP mounts
//!
//! A `fn users(&self) -> &UsersService` method on the parent causes
//! `ApiService::http_router()` to nest `UsersService`'s routes under `/users/`.
//! The child's `list_users` route (`GET /users`) becomes `GET /users/users` in
//! the parent's router. The parent's `openapi_spec()` includes all child paths
//! with the `/users/` prefix.
//!
//! ## MCP mounts
//!
//! The same `fn users(&self) -> &UsersService` mount method makes
//! `ApiService::mcp_tools()` include the child's tools prefixed with `users_`.
//! `list_users` becomes `users_list_users`. Dispatch via
//! `mcp_call("users_list_users", ...)` forwards to the child automatically.
//!
//! Run: cargo run --example mount_service

use server_less::{http, mcp};

// ============================================================================
// Child service: user management
// ============================================================================

/// A focused service that manages users.
///
/// When mounted into a parent, its HTTP routes appear under `/users/` and its
/// MCP tools are prefixed with `users_`.
#[derive(Clone)]
struct UsersService;

#[http]
#[mcp]
impl UsersService {
    /// List all users
    pub fn list_users(&self) -> Vec<String> {
        vec!["alice".to_string(), "bob".to_string()]
    }

    /// Get a user by ID
    pub fn get_user(&self, user_id: String) -> Option<String> {
        match user_id.as_str() {
            "alice" | "bob" => Some(format!("User: {}", user_id)),
            _ => None,
        }
    }

    /// Create a new user
    pub fn create_user(&self, name: String) -> String {
        format!("Created user: {}", name)
    }
}

// ============================================================================
// Parent service: top-level API gateway
// ============================================================================

/// The top-level API service.
///
/// `fn users(&self) -> &UsersService` is the mount point: returning `&T` tells
/// server-less to compose the child's routes and tools into this service,
/// scoped under the method name.
#[derive(Clone)]
struct ApiService {
    users: UsersService,
}

impl ApiService {
    fn new() -> Self {
        Self {
            users: UsersService,
        }
    }
}

#[http]
#[mcp]
impl ApiService {
    /// Health check
    pub fn get_health(&self) -> String {
        "ok".to_string()
    }

    /// Mount the users service.
    ///
    /// HTTP: child routes appear under `/users/`.
    /// MCP: child tools appear as `users_*`.
    pub fn users(&self) -> &UsersService {
        &self.users
    }
}

// ============================================================================
// main: show generated structure
// ============================================================================

fn main() {
    let api = ApiService::new();

    // --- MCP ---
    // Inspect the tool list before consuming the router (http_router moves self).
    let tools = ApiService::mcp_tools();
    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    println!("MCP tools in ApiService:");
    for name in &tool_names {
        println!("  {}", name);
    }

    // Parent leaf tool
    assert!(
        tool_names.contains(&"get_health"),
        "Expected get_health tool, got: {:?}",
        tool_names
    );
    // Mounted child tools — prefixed with users_
    assert!(
        tool_names.contains(&"users_list_users"),
        "Expected users_list_users tool, got: {:?}",
        tool_names
    );
    assert!(
        tool_names.contains(&"users_get_user"),
        "Expected users_get_user tool, got: {:?}",
        tool_names
    );
    assert!(
        tool_names.contains(&"users_create_user"),
        "Expected users_create_user tool, got: {:?}",
        tool_names
    );

    // Dispatch to a mounted child tool
    let result = api.mcp_call("users_list_users", serde_json::json!({}));
    println!("\nmcp_call(\"users_list_users\") -> {:?}", result.unwrap());

    // --- HTTP ---
    // The parent's OpenAPI spec includes child paths prefixed with /users/.
    let spec = ApiService::openapi_spec();
    let paths = spec["paths"].as_object().unwrap();

    println!("\nHTTP routes in ApiService spec:");
    for (path, methods) in paths {
        let method_list: Vec<_> = methods.as_object().unwrap().keys().collect();
        println!(
            "  {} [{}]",
            path,
            method_list
                .iter()
                .map(|m| m.to_uppercase())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Verify child paths appear under /users/
    let path_keys: Vec<&str> = paths.keys().map(|k| k.as_str()).collect();
    assert!(
        path_keys.iter().any(|p| p.starts_with("/users/")),
        "Expected child routes under /users/, got: {:?}",
        path_keys
    );

    // Build the router — consumes the instance, so do this last.
    let api2 = ApiService::new();
    let _router = api2.http_router();
    println!("\nHTTP router created successfully (child routes nested under /users/)");

    println!("\nAll assertions passed.");
}
