# server-less

[![crates.io](https://img.shields.io/crates/v/server-less.svg)](https://crates.io/crates/server-less)
[![docs.rs](https://img.shields.io/docsrs/server-less)](https://docs.rs/server-less)
[![License](https://img.shields.io/crates/l/server-less.svg)](https://github.com/rhi-zone/server-less/blob/master/LICENSE)

**Write less server code.** server-less is a projection system for Rust. You write an impl block — plain methods with plain types — and server-less projects it onto arbitrary protocols: HTTP, CLI, WebSocket, MCP, gRPC, and more.

You're not writing a server, a CLI app, or an API. You're writing your logic. server-less handles the rest.

```rust
impl UserService {
    pub fn create_user(&self, name: String, email: String) -> Result<User, UserError> {
        // This is just your code. No framework, no protocol awareness.
    }
}
```

Add attributes to project it:

```rust
#[http(prefix = "/api")]  // → POST /api/users, GET /api/users/{id}
#[cli(name = "users")]    // → users create-user --name X --email Y
#[mcp]                    // → MCP tools: create_user, get_user
```

Each projection is competitive with hand-written code using the protocol's native library (axum, clap, etc.). That's the quality bar, not the pitch. The pitch is: **annotate once, project anywhere.**

## Installation

```toml
[dependencies]
# Get everything (recommended for getting started)
server-less = "0.5"

# Or select specific features
server-less = { version = "0.5", default-features = false, features = ["http", "cli", "mcp"] }
```

## Quick Start

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

## What's New in 0.5

- **`--manual` whole-tree CLI reference surface.** A single flag emits a complete, machine-readable reference for the entire command tree — every subcommand, parameter, and projection — with content and format treated orthogonally (`--manual --json`, `--manual --jq`). Valid at every node of the tree.
- **`#[derive(Config)]` + config sources.** Layered configuration (TOML files, environment variables) with a generated `config` subcommand.
- **`#[derive(HealthCheck)]`.** A standalone health-check endpoint that composes with the HTTP projection.
- **Shell completions + man pages.** The `completions` feature wires `clap_complete` and `clap_mangen` into the CLI projection.

See the [CHANGELOG](https://github.com/rhi-zone/server-less/blob/master/CHANGELOG.md) for the full history.

## Macros

### Runtime protocol handlers

Generate working server implementations:

| Macro | Protocol | Generated |
|-------|----------|-----------|
| `#[http]` | REST/HTTP | Axum router + OpenAPI spec |
| `#[cli]` | Command line | Clap subcommands |
| `#[mcp]` | Model Context Protocol | Tool schemas + dispatch |
| `#[ws]` | WebSocket | JSON-RPC 2.0 over WebSocket |
| `#[jsonrpc]` | JSON-RPC | Standalone JSON-RPC handler |
| `#[graphql]` | GraphQL | Schema + resolvers (async-graphql) |

### Schema generators

Generate IDL/schema files for cross-language services (no runtime deps):

| Macro | Protocol | Output |
|-------|----------|--------|
| `#[grpc]` | gRPC | `.proto` files |
| `#[capnp]` | Cap'n Proto | `.capnp` schema files |
| `#[thrift]` | Apache Thrift | `.thrift` IDL files |
| `#[smithy]` | AWS Smithy | `.smithy` model files |
| `#[connect]` | Connect RPC | Connect protocol schemas |

### Specification & doc generators

| Macro | Spec type | Output |
|-------|-----------|--------|
| `#[openapi]` | OpenAPI | OpenAPI spec (standalone, no axum) |
| `#[openrpc]` | OpenRPC | JSON-RPC API specification |
| `#[asyncapi]` | AsyncAPI | WebSocket/messaging spec |
| `#[jsonschema]` | JSON Schema | JSON Schema definitions |
| `#[markdown]` | Markdown | Human-readable API docs |

### Blessed presets

Batteries-included bundles — zero config, full progressive disclosure:

| Macro | Bundles |
|-------|---------|
| `#[server]` | HTTP + OpenAPI + serve (the default web server) |
| `#[rpc]` | JSON-RPC over HTTP/WebSocket |
| `#[tool]` | MCP tool surface |
| `#[program]` | CLI program |

### Coordination, metadata & errors

| Macro | Purpose |
|-------|---------|
| `#[serve]` | Compose multiple protocol routers |
| `#[route]` / `#[response]` | Per-method HTTP overrides |
| `#[param]` | Cross-protocol parameter metadata |
| `#[app]` | Application name, description, version, homepage |
| `#[derive(Config)]` | Layered config (TOML, env) + `config` subcommand |
| `#[derive(HealthCheck)]` | Standalone health-check endpoint |
| `#[derive(ServerlessError)]` | Error code inference + protocol status mapping (always available, zero deps) |

## Features

| Category | Features |
|----------|----------|
| **Runtime protocols** | `http`, `cli`, `mcp`, `ws`, `jsonrpc`, `graphql` |
| **Schema generators** | `grpc`, `capnp`, `thrift`, `smithy`, `connect` |
| **Spec generators** | `openapi`, `openrpc`, `asyncapi`, `jsonschema` |
| **Doc generators** | `markdown` |
| **Config & ops** | `config`, `health`, `completions` |
| **Convenience** | `full` (all features, the default) |

The default is `full` — intentionally batteries-included. Opt out with `default-features = false` and pick exactly what you need. `#[derive(ServerlessError)]` is always available regardless of features.

## Highlights

- **Impl-first design** — write methods once, derive protocol handlers.
- **Naming conventions** — `create_*` → POST, `get_*` → GET, `list_*` → collection, and so on.
- **Return-type handling** — `Result<T, E>`, `Option<T>`, `Vec<T>`, `()`, and plain `T` all map correctly.
- **Async support** — sync and async methods both work.
- **SSE streaming** — return `impl Stream<Item = T> + use<>` for Server-Sent Events (Rust 2024).
- **Error mapping** — automatic HTTP status codes and protocol-appropriate error responses.
- **Doc comments** — `///` becomes API documentation across every projection.
- **Parameter extraction** — automatic path/query/body inference.
- **Zero runtime overhead** — pure compile-time code generation; inspect it with `cargo expand`.

## Philosophy

server-less is a **projection system**, not a framework.

- **Frameworks** own your code. You write handlers in their shape, using their types.
- **server-less** projects your code. You write plain Rust methods. Attributes are semantic metadata — `#[param(help = "...")]` becomes CLI help text *and* OpenAPI description *and* MCP tool input docs simultaneously.

**Progressive disclosure.** The zero-config case just works; complexity appears only when you need it. Don't like how server-less handles something? Drop that one derive and write it by hand — everything else still composes.

**Prior art: Serde.** `#[derive(Serialize)]` doesn't compete with hand-written JSON serializers; it's a projection from Rust types onto data formats. server-less does the same thing, from Rust methods onto protocols.

## Documentation

- [Documentation site](https://rhi.zone/server-less/)
- [API docs (docs.rs)](https://docs.rs/server-less)
- [Design documents](https://github.com/rhi-zone/server-less/tree/master/docs/design) — impl-first design, inference vs. configuration, param attributes, error mapping, blessed presets, CLI manual projection, and more.

See the [CHANGELOG](https://github.com/rhi-zone/server-less/blob/master/CHANGELOG.md).

## License

MIT — see [LICENSE](https://github.com/rhi-zone/server-less/blob/master/LICENSE).

---

Part of [RHI](https://rhi.zone/).
