---
layout: home

hero:
  name: Server-less
  text: Composable Derive Macros for Rust
  tagline: Write your implementation once, project it into multiple protocols
  actions:
    - theme: brand
      text: Get Started
      link: /design/impl-first
    - theme: alt
      text: View on GitHub
      link: https://github.com/rhi-zone/server-less

features:
  - icon: 🏗️
    title: Impl-First Design
    details: Write your business logic once as Rust methods. Derive protocol handlers automatically.

  - icon: 🔌
    title: 18 Macros Available
    details: HTTP, CLI, MCP, WebSocket, GraphQL, gRPC, and more. All feature-gated and composable.

  - icon: ✅
    title: 450 Tests Passing
    details: Comprehensive test coverage with zero failures. Production-ready core protocols.

  - icon: 📦
    title: Zero Runtime Overhead
    details: Pure compile-time code generation. No runtime magic, just efficient Rust code.

  - icon: 🎯
    title: Progressive Disclosure
    details: Simple case is trivial. Complexity appears only when you need it.

  - icon: 🔧
    title: Escape Hatch Ready
    details: Don't like how a macro works? Drop to manual Tower layers. Full control when needed.
---

## Quick Example

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
}
```

This generates:
- **HTTP**: Axum router with `POST /api/users`, `GET /api/users/{id}` + OpenAPI spec
- **CLI**: Clap commands: `users create-user --name X --email Y`
- **MCP**: Model Context Protocol tools: `users_create_user`, `users_get_user`
- **WebSocket**: JSON-RPC 2.0 over WebSocket

## Available Macros

### Runtime Protocols (6)
Generate working server implementations:
- `#[http]` - REST/HTTP with Axum + OpenAPI ✅ Production Ready
- `#[cli]` - Command-line with Clap ✅ Production Ready
- `#[mcp]` - Model Context Protocol ✅ Production Ready
- `#[ws]` - WebSocket JSON-RPC 2.0 ✅ Stable
- `#[jsonrpc]` - Standalone JSON-RPC ✅ Stable
- `#[graphql]` - GraphQL schema + resolvers ✅ Working*

### Schema Generators (5)
Generate IDL/schema files:
- `#[grpc]` - Protocol Buffers `.proto` files
- `#[capnp]` - Cap'n Proto `.capnp` schemas
- `#[thrift]` - Apache Thrift `.thrift` IDL
- `#[smithy]` - AWS Smithy `.smithy` models
- `#[connect]` - Connect RPC schemas

### Specification Generators (4)
Generate API documentation:
- `#[openrpc]` - OpenRPC specs (JSON-RPC docs)
- `#[asyncapi]` - AsyncAPI specs (WebSocket/messaging)
- `#[jsonschema]` - JSON Schema definitions
- `#[markdown]` - Human-readable API docs

### Blessed Presets (4)
Batteries-included shortcuts:
- `#[server]` → `#[http]` + `#[serve(http)]`
- `#[rpc]` → `#[jsonrpc]` + `#[openrpc]` + `#[serve(jsonrpc)]`
- `#[tool]` → `#[mcp]` + `#[jsonschema]`
- `#[program]` → `#[cli]` + `#[markdown]`

### Utilities
- `#[derive(ServerlessError)]` - Error code inference + HTTP status mapping
- `#[serve]` - Compose multiple protocol routers
- `#[route]` - Per-method HTTP overrides
- `#[param]` - Per-parameter cross-protocol customization

## Project Status

**Current: v0.1.0 — Foundation ✅**
- 18 macros implemented across 6 crates
- 450 tests passing, 0 failures
- Blessed presets: `#[server]`, `#[rpc]`, `#[tool]`, `#[program]`
- Mount points for nested subcommand composition
- CLI output formatting with `--json`, `--jq`, `--output-schema`
- Published on [crates.io](https://crates.io/crates/server-less)

See [TODO.md](https://github.com/rhi-zone/server-less/blob/master/TODO.md) for the backlog.

## Design Philosophy

Server-less follows four core principles:

1. **Minimize Barrier to Entry** - The simple case should be trivial: `#[derive(Server)]`
2. **Progressive Disclosure** - Complexity appears only when you need it
3. **Gradual Refinement** - Start simple, incrementally add control
4. **Not Here to Judge** - Support multiple workflows, don't prescribe

Read more: [Impl-First Design](/design/impl-first)

## Part of RHI

Server-less is part of the [RHI](https://rhi.zone/) ecosystem - tools for building composable systems.

Related projects:
- **Lotus** - Object store (uses Server-less for server setup)
- **Spore** - Lua runtime with LLM integration
- **Hypha** - Async runtime primitives

## Getting Started

1. **[REST API Tutorial](/tutorials/rest-api)** - Build a blog API in 30 minutes
2. **[Multi-Protocol Tutorial](/tutorials/multi-protocol)** - Expose one service over HTTP, CLI, MCP, and more
3. **[Design Philosophy](/design/impl-first)** - Understand the impl-first approach
4. **[Param Attributes](/design/param-attributes)** - `#[param]` cross-protocol customization

## Installation

```toml
[dependencies]
# Get everything (recommended for getting started)
server-less = "0.1"

# Or select specific features
server-less = { version = "0.1", default-features = false, features = ["http", "cli", "mcp"] }
```

## Contributing

Contributions welcome! See [CLAUDE.md](https://github.com/rhi-zone/server-less/blob/master/CLAUDE.md) for development guidelines.

## License

MIT License - see [LICENSE](https://github.com/rhi-zone/server-less/blob/master/LICENSE) for details.
