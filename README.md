# Trellis

Composable derive macros for Rust.

## Why "Trellis"?

A **trellis** is a lattice structure that gardeners use to support climbing plants. It gives vines and creepers structure to grow upward while remaining flexible enough to adapt to any shape.

This library does the same for Rust code:

- **Support structure** - Attribute macros provide scaffolding for common patterns
- **Composability** - Macros can be stacked on the same impl block
- **Flexibility** - Configure exactly what you need, nothing more

## Quick Start

Write your business logic once, expose it through multiple protocols:

```rust
use trellis::prelude::*;

struct UserService { /* ... */ }

#[http(prefix = "/api")]
#[cli(name = "users")]
#[mcp(namespace = "users")]
#[ws(path = "/ws")]
impl UserService {
    /// Create a new user
    pub async fn create_user(&self, name: String, email: String) -> Result<User, UserError> {
        // your implementation
    }

    /// Get user by ID
    pub async fn get_user(&self, id: String) -> Option<User> {
        // your implementation
    }

    /// List all users
    pub fn list_users(&self, limit: Option<u32>) -> Vec<User> {
        // your implementation
    }
}
```

This generates:

| Protocol | Generated | Usage |
|----------|-----------|-------|
| **HTTP** | `http_router()` | Axum router with `POST /api/users`, `GET /api/users/{id}` |
| **CLI** | `cli_command()` | Clap commands: `users create-user --name X --email Y` |
| **MCP** | `mcp_call()` | Tool dispatch: `users_create_user`, `users_get_user` |
| **WebSocket** | `ws_router()` | JSON-RPC: `{"method": "create_user", "params": {...}}` |

## Features

- **Impl-first design** - Write methods, derive protocol handlers
- **Method naming conventions** - `create_*` → POST, `get_*` → GET, etc.
- **Return type handling** - `Result`, `Option`, `Vec`, `()` mapped appropriately
- **Async support** - Both sync and async methods supported
- **SSE streaming** - `impl Stream<Item=T>` for Server-Sent Events
- **Feature flags** - Only compile what you need

## Installation

```toml
[dependencies]
trellis = "0.1"

# Or select specific features
trellis = { version = "0.1", default-features = false, features = ["http", "cli"] }
```

Available features: `mcp`, `http`, `cli`, `ws`, `full` (default)

## Development

```bash
nix develop        # Enter dev shell
cargo build        # Build all crates
cargo test         # Run tests (64 tests)
cargo expand       # Inspect macro expansion
```

## Documentation

- [API docs](https://docs.rs/trellis) (once published)
- [Design docs](docs/design/) - Implementation notes and design decisions

## Part of Rhizome

Trellis is part of the [Rhizome](https://rhizome-lab.github.io/) ecosystem - tools for programmable creativity.
