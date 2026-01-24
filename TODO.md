# Trellis TODO

Prioritized backlog of pending features, improvements, and ideas.

> **Note:** For completed items, see [CHANGELOG.md](./CHANGELOG.md)

**Current Status:** 167+ tests passing (30 test suites), all clippy checks clean

**Recent:** Context injection implemented for HTTP, CLI, JSON-RPC, and WebSocket protocols.

---

## High Priority

### WebSocket Bidirectional Patterns
**Status:** ✅ Implemented

See commit `feat(ws): add WsSender for bidirectional WebSocket communication` for details.

Methods can now receive a `WsSender` parameter for server push:
```rust
#[ws]
impl Service {
    async fn handle(&self, data: String, sender: WsSender) -> String {
        // Can respond immediately
        let response = format!("Echo: {}", data);

        // Can also push messages later
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            sender.send("Delayed message").await.ok();
        });

        response
    }
}
```

### WebSocket Server-Push Examples
**Status:** ✅ Implemented

Comprehensive examples added to `crates/server-less-macros/src/ws.rs` documentation:
- Chat room with broadcasting
- Background tasks with periodic heartbeats
- Combining Context + WsSender for authenticated subscriptions

### Improve inline docs
**Status:** ✅ Implemented

- ✅ Added more examples to existing documentation
- ✅ Document Rust 2024 `+ use<>` requirement for streaming

## Medium Priority

### Streaming Support
**Status:** Partially implemented

- ✅ SSE for HTTP (implemented - see `examples/streaming_service.rs`)
- Streaming responses for MCP
- Server-push patterns for WebSocket (blocked by bidirectional patterns above)
- Document Rust 2024 `+ use<>` requirement for streaming

### Extract OpenAPI as Standalone Macro
**Status:** ✅ Partially implemented

Two approaches now available:

**1. Opt-out flag on `#[http]`:**
```rust
#[http(openapi = false)]  // No openapi_spec() method generated
impl Service { }

#[http]  // Default: openapi = true, generates openapi_spec()
impl Service { }
```

**2. Standalone `#[openapi]` macro:**
```rust
#[openapi(prefix = "/api/v1")]
impl UserService {
    fn create_user(&self, name: String) -> User { }
    fn get_user(&self, id: String) -> Option<User> { }
}

// Use it:
let spec = UserService::openapi_spec();
```

**Current limitation:** `#[openapi]` requires the `http` feature because it reuses
`generate_openapi_spec` and related types from `http.rs`.

**Future improvements:**

1. **Independent `openapi` feature:** Extract shared OpenAPI generation logic into a
   separate module so `#[openapi]` can work without the full HTTP runtime. Would allow:
   ```toml
   server-less = { features = ["openapi"] }  # Just schema generation, no axum
   ```

2. **Trait-based composable approach:** Define an `OpenApiSpec` trait that protocols
   can implement, allowing generic composition:
   ```rust
   trait OpenApiSpec {
       fn paths() -> Vec<OpenApiPath>;
       fn schemas() -> Vec<OpenApiSchema>;
   }

   // Protocol macros implement this:
   #[http]  // Generates OpenApiSpec impl
   #[jsonrpc]  // Could also generate OpenApiSpec impl
   impl Service { }

   // Then compose:
   let combined_spec = OpenApi::new()
       .merge(HttpService::openapi_spec())
       .merge(JsonRpcService::openapi_spec())
       .build();
   ```
   This would enable cross-protocol schema sharing and composition.

### OpenAPI Improvements (Post-extraction)
- Add parameter schemas
- Add response schemas
- Support for `#[openapi(hidden)]` to exclude endpoints

### gRPC Support
Add `#[grpc]` macro for tonic/protobuf generation. Would test:
- Protocol buffer schema generation
- Streaming (unary, server, client, bidirectional)
- Error code mapping to gRPC status codes

## Low Priority

### GraphQL Improvements
GraphQL macro is implemented. Remaining improvements:
- ✅ Query vs Mutation distinction (done)
- ✅ Array type mapping (done - Vec<T> → [T])
- ✅ Object type mapping (done - custom structs → GraphQL objects)
- Nested type resolution for complex relationships
- N+1 problem considerations and dataloader patterns
- Custom scalar support (DateTime, UUID, etc.)
- Subscription support for real-time updates

### "Serve" Coordination Pattern
Type-safe coordination between derives:
```rust
#[derive(Server)]  // blessed preset
struct MyServer;

// Expands to:
#[derive(ServerCore)]
#[derive(OpenApi)]
#[derive(Metrics)]
#[derive(HealthCheck)]
#[derive(Serve)]
struct MyServer;
```

### Attribute Customization
**Status:** Partially implemented

Route overrides are done:
- ✅ `#[route(method = "POST")]` - override HTTP method
- ✅ `#[route(path = "/custom")]` - override path
- ✅ `#[route(skip)]` - exclude from routing
- ✅ `#[route(hidden)]` - exclude from OpenAPI

Still needed (see "Parameter Customization" and "Response Customization" above):
- `#[param(...)]` - customize parameters
- `#[response(...)]` - customize responses

## Ideas / Research

### Context System Review
**Status:** Deferred design decisions documented

Review deferred Context design decisions for MCP and GraphQL protocols. See [CONTEXT_DECISIONS.md](./CONTEXT_DECISIONS.md) for full analysis.

**Key questions to revisit:**
1. **MCP Context:** Should MCP support conversation context tracking?
   - Currently: Tools get data from LLM arguments only (stateless)
   - Alternative: Track conversation ID, turn number, user across calls
   - When to reconsider: If users request it or MCP gets HTTP transport

2. **GraphQL Context Bridge:** Should we auto-bridge server_less::Context into async-graphql?
   - Currently: async-graphql uses its own ResolverContext
   - Alternative: Auto-extract headers into server_less::Context, insert into async-graphql context
   - Benefit: Consistent header extraction across all HTTP protocols

3. **Extensible Context:** How should users add custom data to Context?
   - Use cases: Auth data (user ID, roles), multi-tenancy, tracing, feature flags
   - Options: Generic Context<T>, with_data(), middleware injection
   - Challenge: Type safety vs flexibility, zero-cost abstractions

**Trigger for review:** User feedback, new use cases, or completion of middleware system

### Hot Reloading
Could macros generate code that supports hot reloading for development?

### Schema Sharing
MCP, OpenAPI, GraphQL all need schemas. Could share a common schema representation?

### Middleware/Hooks
```rust
#[http]
#[middleware(auth, logging)]
impl Service { }
```

**Note:** Extensible Context (see above) may integrate with middleware system.

### Versioning
API versioning support across protocols.
