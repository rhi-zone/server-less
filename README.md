# server-less

[![Tests](https://img.shields.io/badge/tests-187%20passing-brightgreen)](https://github.com/rhizome-lab/server-less)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

**Write less server code.** Composable derive macros for Rust. Write your implementation once, project it into multiple protocols.

## Philosophy

Server-less is about minimizing boilerplate while maximizing flexibility:

- **Convention over configuration** - Sensible defaults that just work
- **Composable** - Stack multiple macros on the same impl block
- **Progressive disclosure** - Simple by default, powerful when needed
- **Escape hatches** - Drop to manual code whenever you want

## Quick Start

Write your business logic once, expose it through multiple protocols:

```rust
use server_less::prelude::*;

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
| **HTTP** | `http_router()` | Axum router with `POST /api/users`, `GET /api/users/{id}`, OpenAPI |
| **CLI** | `cli_command()` | Clap commands: `users create-user --name X --email Y` |
| **MCP** | `mcp_call()` | Model Context Protocol tools: `users_create_user`, `users_get_user` |
| **WebSocket** | `ws_router()` | JSON-RPC 2.0: `{"method": "create_user", "params": {...}}` |

## Available Macros

### Runtime Protocol Handlers

Generate working server implementations:

| Macro | Protocol | Generated Code | Status |
|-------|----------|----------------|--------|
| `#[http]` | REST/HTTP | Axum router + OpenAPI spec | ✅ Production Ready |
| `#[cli]` | Command Line | Clap subcommands | ✅ Production Ready |
| `#[mcp]` | Model Context Protocol | Tool schemas + dispatch | ✅ Production Ready |
| `#[ws]` | WebSocket | JSON-RPC 2.0 over WebSocket | ✅ Stable |
| `#[jsonrpc]` | JSON-RPC | Standalone JSON-RPC handler | ✅ Stable |
| `#[graphql]` | GraphQL | Schema + resolvers (async-graphql) | ✅ Working* |

*GraphQL has minor type mapping limitations for complex types

### Schema Generators

Generate IDL/schema files for cross-language services:

| Macro | Protocol | Output | Status |
|-------|----------|--------|--------|
| `#[grpc]` | gRPC | `.proto` files (Protocol Buffers) | ✅ Working |
| `#[capnp]` | Cap'n Proto | `.capnp` schema files | ✅ Working |
| `#[thrift]` | Apache Thrift | `.thrift` IDL files | ✅ Working |
| `#[smithy]` | AWS Smithy | `.smithy` model files | ✅ Working |
| `#[connect]` | Connect RPC | Connect protocol schemas | ✅ Working |

### Specification Generators

Generate API documentation and contracts:

| Macro | Spec Type | Output | Status |
|-------|-----------|--------|--------|
| `#[openrpc]` | OpenRPC | JSON-RPC API specification | ✅ Working |
| `#[asyncapi]` | AsyncAPI | WebSocket/messaging spec | ✅ Working |
| `#[jsonschema]` | JSON Schema | JSON Schema definitions | ✅ Working |
| `#[markdown]` | Markdown | Human-readable API docs | ✅ Working |

### Error Handling

| Macro | Purpose | Status |
|-------|---------|--------|
| `#[derive(TrellisError)]` | Error code inference + HTTP status mapping | ✅ Working |

### Coordination

| Macro | Purpose | Status |
|-------|---------|--------|
| `#[serve]` | Compose multiple protocol routers | ✅ Working |
| `#[route]` | Per-method attribute overrides | ✅ Working |

**Total: 18 macros, 171 passing tests, 0 failures**

## Features

- **Impl-first design** - Write methods once, derive protocol handlers
- **Method naming conventions** - `create_*` → POST, `get_*` → GET, `list_*` → collection, etc.
- **Return type handling** - `Result<T,E>`, `Option<T>`, `Vec<T>`, `()`, plain `T` all mapped correctly
- **Async support** - Both sync and async methods work seamlessly
- **SSE streaming** - `impl Stream<Item=T>` for Server-Sent Events (Rust 2024 `+ use<>`)
- **Error mapping** - Automatic HTTP status codes and error responses
- **Doc comments** - `///` comments become API documentation
- **Parameter extraction** - Automatic path/query/body inference
- **Feature gated** - Only compile what you need
- **Zero runtime overhead** - Pure compile-time code generation

## Installation

```toml
[dependencies]
# Get everything (recommended for getting started)
server-less = { git = "https://github.com/rhizome-lab/server-less" }

# Or select specific features
server-less = { git = "https://github.com/rhizome-lab/server-less", default-features = false, features = ["http", "cli", "mcp"] }
```

### Available Features

| Category | Features |
|----------|----------|
| **Runtime protocols** | `http`, `cli`, `mcp`, `ws`, `jsonrpc`, `graphql` |
| **Schema generators** | `grpc`, `capnp`, `thrift`, `smithy`, `connect` |
| **Spec generators** | `openrpc`, `asyncapi`, `jsonschema`, `markdown` |
| **Convenience** | `full` (all features, default) |

**Note:** `TrellisError` derive is always available (zero deps).

## Examples

Check out [examples/](crates/server-less/examples/) for working code:

- **[http_service.rs](crates/server-less/examples/http_service.rs)** - REST API with Axum + OpenAPI
- **[cli_service.rs](crates/server-less/examples/cli_service.rs)** - CLI application with Clap
- **[user_service.rs](crates/server-less/examples/user_service.rs)** - Multi-protocol (HTTP + CLI + MCP + WS)
- **[ws_service.rs](crates/server-less/examples/ws_service.rs)** - WebSocket JSON-RPC server
- **[streaming_service.rs](crates/server-less/examples/streaming_service.rs)** - SSE streaming over HTTP

## Server-Sent Events (SSE) Streaming

Server-less supports SSE streaming by returning `impl Stream<Item = T>`:

```rust
use futures::stream::{self, Stream};

#[http]
impl Service {
    /// Stream events to the client
    pub fn stream_events(&self, count: u64) -> impl Stream<Item = Event> + use<> {
        stream::iter((0..count).map(|i| Event { id: i }))
    }
}
```

**Important:** The `+ use<>` syntax is **required** for Rust 2024 edition when using `impl Trait` in return position with streaming. This tells the compiler to capture all generic parameters in scope. Without it, you'll get compilation errors about lifetime capture.

```rust
// ✅ Correct - Rust 2024
pub fn stream(&self) -> impl Stream<Item = T> + use<> { ... }

// ❌ Error - Missing use<> in Rust 2024
pub fn stream(&self) -> impl Stream<Item = T> { ... }
```

The generated code automatically wraps your stream in SSE format with proper event handling.

## Roadmap

### Current - Foundation ✅
- [x] Core runtime protocols (HTTP, CLI, MCP, WebSocket, JSON-RPC, GraphQL)
- [x] Schema generators (gRPC, Cap'n Proto, Thrift, Smithy, Connect)
- [x] Specification generators (OpenRPC, AsyncAPI, JSON Schema, Markdown)
- [x] Error derive macro with HTTP status mapping
- [x] Serve macro for multi-protocol composition
- [x] **187 passing integration tests** ✨
- [x] Complete design documentation and tutorials
- [x] Working examples for all major protocols

### Recently Completed - Polish & Refinement ✅
- [x] **GraphQL improvements**: Array/object type mapping fixed
- [x] **Error handling**: Proper Result types in schema validation
- [x] **SSE streaming**: Server-Sent Events support
- [x] **Documentation**: Inline docs for all macros with examples
- [x] **Attribute customization**: `#[route(method="POST", path="/custom", skip, hidden)]`
- [x] **Helpful error messages**: All macros provide actionable hints
- [x] **Compile-time validation**: HTTP path validation, duplicate route detection

### Next - Advanced Features
- [ ] **Response customization**: `#[response(status = 201)]`
- [ ] **Parameter customization**: `#[param(query, name = "q")]`
- [ ] **WebSocket bidirectional**: Server-push patterns
- [ ] **OpenAPI separation**: Extract as standalone macro

### Medium Term - Developer Experience
- [ ] Improved error messages with better span information
- [ ] Code action support (IDE integration hints)
- [ ] Middleware/hooks pattern for cross-cutting concerns
- [ ] Hot reloading support for development
- [ ] Schema validation at compile time
- [ ] Performance benchmarks vs hand-written code

### Long Term - Advanced Features
- [ ] API versioning support
- [ ] Rate limiting derive macro
- [ ] Authentication/authorization hooks
- [ ] Request/response transformation layers
- [ ] Schema sharing across protocols
- [ ] Multi-language client generation (TypeScript, Python)

### Eventually - Stability & Ecosystem
- [ ] API stability guarantees
- [ ] Production battle-testing
- [ ] Performance optimization
- [ ] Long-term support commitment
- [ ] Extension ecosystem

## Philosophy

Trellis follows an **impl-first design** approach:

1. **Write your implementation** - Focus on business logic
2. **Add protocol macros** - Derive handlers from methods
3. **Customize as needed** - Progressive disclosure of complexity
4. **Escape hatch available** - Drop to manual code when needed

### Design Principles

- **Minimize barrier to entry** - The simple case should be trivial
- **Progressive disclosure** - Complexity appears only when you need it
- **Gradual refinement** - Start simple, incrementally add control
- **Not here to judge** - Support multiple workflows, don't prescribe
- **Silly but proper** - Simple things done right (good errors, readable code)

See [docs/design/](docs/design/) for detailed design philosophy.

## Development

```bash
nix develop        # Enter dev shell (optional)
cargo build        # Build all crates
cargo test         # Run all tests (171 passing)
cargo clippy       # Lint checks
cargo expand       # Inspect macro expansion
```

### Project Structure

```
trellis/
├── crates/
│   ├── trellis/          # Main crate (re-exports)
│   ├── trellis-macros/   # Proc macro implementations (18 macros, 5,142 LOC)
│   ├── trellis-core/     # Core traits & error types
│   ├── trellis-parse/    # Shared parsing utilities
│   └── trellis-rpc/      # RPC dispatch utilities
└── docs/
    ├── design/           # Design documents
    └── .vitepress/       # Documentation site
```

## Documentation

- **[Design Philosophy](docs/design/impl-first.md)** - Impl-first approach and naming conventions
- **[Extension Coordination](docs/design/extension-coordination.md)** - How macros compose
- **[Implementation Notes](docs/design/implementation-notes.md)** - Technical decisions
- **[Iteration Log](docs/design/iteration-log.md)** - Evolution and design decisions
- **[CLAUDE.md](CLAUDE.md)** - Development guidelines for AI assistants

## Part of Rhizome

Trellis is part of the [Rhizome](https://rhizome-lab.github.io/) ecosystem - tools for building composable systems.

Related projects:
- **Lotus** - Object store (uses Trellis for server setup)
- **Spore** - Lua runtime with LLM integration
- **Hypha** - Async runtime primitives

## Contributing

Contributions welcome! Please check:

1. Run tests: `cargo test`
2. Run clippy: `cargo clippy --all-targets --all-features -- -D warnings`
3. Format code: `cargo fmt --all`
4. Follow [conventional commits](https://www.conventionalcommits.org/)

See [CLAUDE.md](CLAUDE.md) for development guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Inspired by the composability of Serde and the "just works" experience of Clap.
