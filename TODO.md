# Server-less TODO

Prioritized backlog of pending features, improvements, and ideas.

> **Note:** For completed items, see [CHANGELOG.md](./CHANGELOG.md)

---

## Queued

### OpenAPI Composition (Phases 2-4)

Phase 1 complete: `OpenApiBuilder` in `server-less-openapi` crate.
See `docs/design/openapi-composition.md` for full design.

- [ ] **Phase 2: Per-protocol OpenAPI methods**
  - Add `http_openapi_paths()` to `#[http]`
  - Add `jsonrpc_openapi_paths()` to `#[jsonrpc]`
  - Add similar methods to `#[graphql]`, `#[ws]`

- [ ] **Phase 3: Serve integration**
  - `#[serve]` auto-generates combined `openapi_spec()` from detected protocols
  - Opt-out via `#[serve(openapi = false)]`

- [ ] **Phase 4: Protocol-aware #[openapi]**
  - `#[openapi]` detects sibling protocol attributes
  - Generates combined spec when multiple protocols present

### OpenAPI Improvements

- [ ] Add richer parameter schemas (beyond just type)
- [ ] Add response schemas with examples
- [ ] Support `#[openapi(hidden)]` to exclude specific endpoints

### GraphQL Improvements

- [ ] Nested type resolution for complex relationships
- [ ] N+1 problem considerations and dataloader patterns
- [ ] Custom scalar support (DateTime, UUID, etc.)
- [ ] Subscription support for real-time updates

### Streaming

- [ ] MCP streaming responses
- [ ] gRPC streaming exploration (unary, server, client, bidirectional)

### Error Handling

- [ ] Replace `panic!()` with `Result` in schema validation (grpc, capnp, thrift, smithy)
- [ ] Better error messages with spans for schema generators

---

## Ideas / Research

### Schema Sharing

MCP, OpenAPI, GraphQL all need schemas. Could share a common schema representation?
- Common intermediate format
- Render to OpenAPI, MCP tool schema, GraphQL schema
- Validate consistency across protocols

### Middleware System

```rust
#[http]
#[middleware(auth, logging)]
impl Service { }
```

Tower layer integration, before/after hooks, async middleware.

### Context Extensions

How should users add custom data to Context?
- Use cases: Auth data (user ID, roles), multi-tenancy, tracing, feature flags
- Options: Generic `Context<T>`, `with_data()`, middleware injection

See [CONTEXT_DECISIONS.md](./CONTEXT_DECISIONS.md) for full analysis.

### Hot Reloading

Could macros generate code that supports hot reloading for development?

### API Versioning

```rust
#[http(version = "v1")]
impl Service { }
```

URL versioning, header-based versioning, deprecation warnings.

### Client Generation

- TypeScript client from OpenAPI spec
- Python client from OpenAPI spec
- Rust client from schema

---

## Low Priority

### gRPC Runtime Support

Currently `#[grpc]` generates proto schema only. Could add:
- tonic integration for actual gRPC server
- Streaming support (all four patterns)
- Error code mapping to gRPC status codes

### "Server" Blessed Preset

Type-safe coordination between derives:
```rust
#[derive(Server)]  // blessed preset
struct MyServer;

// Expands to:
#[derive(ServerCore, OpenApi, Metrics, HealthCheck, Serve)]
struct MyServer;
```

### IDE Integration

- rust-analyzer macro expansion hints
- Go-to-definition for generated code
- Code actions for adding macros

---

## Completed

Moved to [CHANGELOG.md](./CHANGELOG.md):
- ✅ WebSocket bidirectional patterns (WsSender)
- ✅ WebSocket server-push examples
- ✅ Context injection (HTTP, CLI, JSON-RPC, WebSocket)
- ✅ OpenAPI standalone macro and feature flag
- ✅ OpenApiBuilder for spec composition (Phase 1)
- ✅ Route overrides (`#[route]` attribute)
- ✅ Response customization (`#[response]` attribute)
- ✅ Parameter customization (`#[param]` attribute)
- ✅ SSE streaming for HTTP
- ✅ Compile-time path validation
- ✅ Duplicate route detection
- ✅ Inline documentation improvements
