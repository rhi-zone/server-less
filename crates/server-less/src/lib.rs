//! Server-less - Composable derive macros for Rust
//!
//! Server-less takes an **impl-first** approach: write your Rust methods,
//! and derive macros project them into various protocols (HTTP, CLI, MCP, WebSocket).
//!
//! # Quick Start
//!
//! ```ignore
//! use server_less::prelude::*;
//!
//! struct UserService {
//!     // your state
//! }
//!
//! #[http]
//! #[cli(name = "users")]
//! #[mcp]
//! #[ws(path = "/ws")]
//! impl UserService {
//!     /// Create a new user
//!     async fn create_user(&self, name: String, email: String) -> Result<User, UserError> {
//!         // implementation
//!     }
//!
//!     /// Get user by ID
//!     async fn get_user(&self, id: UserId) -> Option<User> {
//!         // implementation
//!     }
//!
//!     /// List all users
//!     async fn list_users(&self, limit: Option<u32>) -> Vec<User> {
//!         // implementation
//!     }
//! }
//! ```
//!
//! This generates:
//! - **HTTP**: `POST /users`, `GET /users/{id}`, `GET /users` (axum router)
//! - **CLI**: `users create-user --name X`, `users get-user <id>` (clap)
//! - **MCP**: Tools `create_user`, `get_user`, `list_users` (Model Context Protocol)
//! - **WebSocket**: JSON-RPC methods over WebSocket (axum)
//!
//! # Available Macros
//!
//! | Macro | Protocol | Generated Methods |
//! |-------|----------|-------------------|
//! | `#[http]` | HTTP/REST | `http_router()`, `openapi_spec()` |
//! | `#[cli]` | Command Line | `cli_command()`, `cli_run()` |
//! | `#[mcp]` | MCP | `mcp_tools()`, `mcp_call()`, `mcp_call_async()` |
//! | `#[ws]` | WebSocket | `ws_router()`, `ws_handle_message()`, `ws_handle_message_async()` |
//!
//! # Naming Conventions
//!
//! Method names infer HTTP methods and CLI subcommand structure:
//!
//! | Prefix | HTTP | CLI |
//! |--------|------|-----|
//! | `create_*`, `add_*` | POST | `<cmd> create-*` |
//! | `get_*`, `fetch_*` | GET (single) | `<cmd> get-*` |
//! | `list_*`, `find_*` | GET (collection) | `<cmd> list-*` |
//! | `update_*`, `set_*` | PUT | `<cmd> update-*` |
//! | `delete_*`, `remove_*` | DELETE | `<cmd> delete-*` |
//!
//! # Return Types
//!
//! | Type | HTTP | CLI | MCP/WS |
//! |------|------|-----|--------|
//! | `T` | 200 + JSON | stdout JSON | JSON result |
//! | `Option<T>` | 200 or 404 | stdout or exit 1 | result or null |
//! | `Result<T, E>` | 200 or error | stdout or stderr | result or error |
//! | `()` | 204 | silent | `{"success": true}` |
//! | `impl Stream<Item=T>` | SSE | N/A | N/A |
//!
//! # Async Methods
//!
//! All macros support async methods:
//!
//! ```ignore
//! #[mcp]
//! impl MyService {
//!     // Sync method - works with mcp_call() and mcp_call_async()
//!     pub fn sync_method(&self) -> String { ... }
//!
//!     // Async method - use mcp_call_async() for proper await
//!     pub async fn async_method(&self) -> String { ... }
//! }
//!
//! // Sync call (errors on async methods)
//! service.mcp_call("sync_method", json!({}));
//!
//! // Async call (awaits async methods properly)
//! service.mcp_call_async("async_method", json!({})).await;
//! ```
//!
//! # SSE Streaming (HTTP)
//!
//! Return `impl Stream<Item=T>` for Server-Sent Events:
//!
//! ```ignore
//! #[http]
//! impl StreamService {
//!     // Note: Rust 2024 requires `+ use<>` to avoid lifetime capture
//!     pub fn stream_events(&self) -> impl Stream<Item = Event> + use<> {
//!         futures::stream::iter(vec![Event { ... }])
//!     }
//! }
//! ```
//!
//! # Feature Flags
//!
//! Enable only what you need:
//!
//! ```toml
//! [dependencies]
//! server-less = { version = "0.1", default-features = false, features = ["http", "cli"] }
//! ```
//!
//! Available features:
//! - `mcp` - MCP macro (no extra deps)
//! - `http` - HTTP macro (requires axum)
//! - `cli` - CLI macro (requires clap)
//! - `ws` - WebSocket macro (requires axum, futures)
//! - `full` - All features (default)

// Re-export macros (feature-gated)
#[cfg(feature = "mcp")]
pub use server_less_macros::mcp;

#[cfg(feature = "http")]
pub use server_less_macros::http;

#[cfg(feature = "http")]
pub use server_less_macros::openapi;

#[cfg(feature = "http")]
pub use server_less_macros::route;

#[cfg(feature = "http")]
pub use server_less_macros::response;

#[cfg(feature = "http")]
pub use server_less_macros::serve;

#[cfg(feature = "cli")]
pub use server_less_macros::cli;

#[cfg(feature = "ws")]
pub use server_less_macros::ws;

#[cfg(feature = "jsonrpc")]
pub use server_less_macros::jsonrpc;

#[cfg(feature = "openrpc")]
pub use server_less_macros::openrpc;

#[cfg(feature = "graphql")]
pub use server_less_macros::graphql;

#[cfg(feature = "grpc")]
pub use server_less_macros::grpc;

#[cfg(feature = "capnp")]
pub use server_less_macros::capnp;

#[cfg(feature = "thrift")]
pub use server_less_macros::thrift;

#[cfg(feature = "connect")]
pub use server_less_macros::connect;

#[cfg(feature = "smithy")]
pub use server_less_macros::smithy;

#[cfg(feature = "markdown")]
pub use server_less_macros::markdown;

#[cfg(feature = "jsonschema")]
pub use server_less_macros::jsonschema;

#[cfg(feature = "asyncapi")]
pub use server_less_macros::asyncapi;

// Error derive macro (always available - no deps, commonly needed)
pub use server_less_macros::ServerlessError;

// Re-export futures for generated WebSocket code
#[cfg(feature = "ws")]
pub use futures;

// Re-export async-graphql for generated GraphQL code
#[cfg(feature = "graphql")]
pub use async_graphql;
#[cfg(feature = "graphql")]
pub use async_graphql_axum;

// Re-export core types
pub use server_less_core::*;

// Re-export serde for generated code
pub use serde;
pub use serde_json;

/// Prelude for convenient imports
pub mod prelude {
    // Runtime protocols
    #[cfg(feature = "cli")]
    pub use super::cli;
    #[cfg(feature = "graphql")]
    pub use super::graphql;
    #[cfg(feature = "http")]
    pub use super::http;
    #[cfg(feature = "jsonrpc")]
    pub use super::jsonrpc;
    #[cfg(feature = "mcp")]
    pub use super::mcp;
    #[cfg(feature = "http")]
    pub use super::openapi;
    #[cfg(feature = "http")]
    pub use super::response;
    #[cfg(feature = "http")]
    pub use super::route;
    #[cfg(feature = "http")]
    pub use super::serve;
    #[cfg(feature = "ws")]
    pub use super::ws;

    // Schema generators
    #[cfg(feature = "capnp")]
    pub use super::capnp;
    #[cfg(feature = "connect")]
    pub use super::connect;
    #[cfg(feature = "grpc")]
    pub use super::grpc;
    #[cfg(feature = "smithy")]
    pub use super::smithy;
    #[cfg(feature = "thrift")]
    pub use super::thrift;

    // Specification generators
    #[cfg(feature = "asyncapi")]
    pub use super::asyncapi;
    #[cfg(feature = "jsonschema")]
    pub use super::jsonschema;
    #[cfg(feature = "openrpc")]
    pub use super::openrpc;

    // Documentation generators
    #[cfg(feature = "markdown")]
    pub use super::markdown;

    // Always available
    pub use super::{Context, ErrorCode, ErrorResponse, IntoErrorCode, ServerlessError};
    pub use serde::{Deserialize, Serialize};

    // WebSocket sender (when ws feature enabled)
    #[cfg(feature = "ws")]
    pub use super::WsSender;
}
