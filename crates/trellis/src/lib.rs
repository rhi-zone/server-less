//! Trellis - Composable derive macros for Rust
//!
//! Trellis takes an **impl-first** approach: write your Rust methods,
//! and derive macros project them into various protocols (HTTP, CLI, MCP, etc.).
//!
//! # Quick Start
//!
//! ```ignore
//! use trellis::prelude::*;
//!
//! struct UserService {
//!     // your state
//! }
//!
//! #[http]
//! #[cli(name = "users")]
//! #[mcp]
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
//! - HTTP routes: `POST /users`, `GET /users/{id}`, `GET /users`
//! - CLI commands: `users create-user --name X --email Y`, `users get-user <id>`
//! - MCP tools: `create_user`, `get_user`, `list_users`
//!
//! # Naming Conventions
//!
//! Method names are used to infer behavior:
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
//! | Type | HTTP | CLI |
//! |------|------|-----|
//! | `T` | 200 + JSON | stdout JSON |
//! | `Option<T>` | 200 or 404 | stdout or exit 1 |
//! | `Result<T, E>` | 200 or error | stdout or stderr |
//! | `()` | 204 | silent |

// Re-export macros
pub use trellis_macros::{cli, http, mcp};

// Re-export core types
pub use trellis_core::*;

// Re-export serde for generated code
pub use serde;
pub use serde_json;

/// Prelude for convenient imports
pub mod prelude {
    pub use super::{cli, http, mcp};
    pub use super::{Context, ErrorCode, ErrorResponse, IntoErrorCode};
    pub use serde::{Deserialize, Serialize};
}
