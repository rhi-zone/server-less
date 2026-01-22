---
layout: home

hero:
  name: Trellis
  text: Composable Derive Macros for Rust
  tagline: Write your implementation once, project it into multiple protocols
  actions:
    - theme: brand
      text: Get Started
      link: /design/impl-first
    - theme: alt
      text: View on GitHub
      link: https://github.com/rhizome-lab/trellis

features:
  - icon: 🏗️
    title: Impl-First Design
    details: Write your business logic once as Rust methods. Derive protocol handlers automatically.

  - icon: 🔌
    title: 18 Macros Available
    details: HTTP, CLI, MCP, WebSocket, GraphQL, gRPC, and more. All feature-gated and composable.

  - icon: ✅
    title: 171 Tests Passing
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
use rhizome_trellis::prelude::*;

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

### Utilities (3)
- `#[derive(TrellisError)]` - Error code inference + HTTP status mapping
- `#[serve]` - Compose multiple protocol routers
- `#[route]` - Per-method attribute overrides

## Project Status

**Current: Foundation ✅**
- 18 macros implemented
- 171 tests passing, 0 failures
- Complete design documentation
- Working examples for all major protocols

**Next: Polish & Refinement**
- GraphQL type mapping fixes
- Better error handling
- Improved documentation

See [ROADMAP.md](https://github.com/rhizome-lab/trellis/blob/master/ROADMAP.md) for details.

## Design Philosophy

Trellis follows four core principles:

1. **Minimize Barrier to Entry** - The simple case should be trivial: `#[derive(Server)]`
2. **Progressive Disclosure** - Complexity appears only when you need it
3. **Gradual Refinement** - Start simple, incrementally add control
4. **Not Here to Judge** - Support multiple workflows, don't prescribe

Read more: [Impl-First Design](/design/impl-first)

## Part of Rhizome

Trellis is part of the [Rhizome](https://rhizome-lab.github.io/) ecosystem - tools for building composable systems.

Related projects:
- **Lotus** - Object store (uses Trellis for server setup)
- **Spore** - Lua runtime with LLM integration
- **Hypha** - Async runtime primitives

## Getting Started

1. **[Design Philosophy](/design/impl-first)** - Understand the impl-first approach
2. **[Extension Coordination](/design/extension-coordination)** - How macros compose
3. **[Implementation Notes](/design/implementation-notes)** - Technical decisions
4. **[Iteration Log](/design/iteration-log)** - Evolution and design history

## Installation

```toml
[dependencies]
# Get everything (recommended for getting started)
rhizome-trellis = { git = "https://github.com/rhizome-lab/trellis" }

# Or select specific features
rhizome-trellis = { git = "https://github.com/rhizome-lab/trellis", default-features = false, features = ["http", "cli", "mcp"] }
```

## Contributing

Contributions welcome! See [CLAUDE.md](https://github.com/rhizome-lab/trellis/blob/master/CLAUDE.md) for development guidelines.

## License

MIT License - see [LICENSE](https://github.com/rhizome-lab/trellis/blob/master/LICENSE) for details.
