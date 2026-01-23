# Trellis TODO

Prioritized backlog of pending features, improvements, and ideas.

> **Note:** For completed items, see [CHANGELOG.md](./CHANGELOG.md)

**Current Status:** 167+ tests passing (30 test suites), all clippy checks clean

**Recent:** Context injection implemented for HTTP, CLI, JSON-RPC, and WebSocket protocols.

---

## High Priority

### WebSocket Bidirectional Patterns
**Status:** Blocked - requires architecture changes

Currently WebSocket handlers are stateless request-response. True bidirectional communication requires handlers to have access to a sender/channel to push messages independently of requests.

**Current limitation:**
```rust
#[ws]
impl Service {
    // Can only respond to incoming messages
    async fn handle(&self, data: String) -> String {
        format!("Echo: {}", data)
    }
}
```

**Desired capability:**
```rust
#[ws]
impl Service {
    // Need access to sender to push messages
    async fn handle(&self, data: String, sender: WsSender) -> String {
        // Can respond immediately
        let response = format!("Echo: {}", data);

        // Can also push messages later
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            sender.send("Delayed message").await;
        });

        response
    }
}
```

**Technical considerations:**
- Need to inject `WsSender` into handler signature
- Requires changes to `crates/trellis-macros/src/ws.rs` parameter extraction
- May need new types in `crates/trellis-core/` for sender abstraction
- Must handle sender lifecycle (what if connection closes?)
- Should work with both JSON-RPC and raw WebSocket modes

**Related:** WebSocket server-push examples (depends on this)

### WebSocket Server-Push Examples
**Status:** Blocked by bidirectional patterns

Once bidirectional support is implemented, add examples showing:
- Server-initiated notifications
- Broadcasting to multiple clients
- Subscription patterns (pub/sub)
- Live data feeds (stock prices, metrics, etc.)

**Location:** `examples/websocket-server-push/`

### Improve inline docs
- Add more examples to existing documentation
- Document Rust 2024 `+ use<>` requirement for streaming

## Medium Priority

### Streaming Support
**Status:** Partially implemented

- ✅ SSE for HTTP (implemented - see `examples/streaming_service.rs`)
- Streaming responses for MCP
- Server-push patterns for WebSocket (blocked by bidirectional patterns above)
- Document Rust 2024 `+ use<>` requirement for streaming

### Extract OpenAPI as Standalone Macro
**Status:** Architecture decision needed

Currently OpenAPI spec generation is built into the `#[http]` macro, creating coupling. Should be extracted for:
- Independent customization of OpenAPI generation
- Composition with other schema generators
- Reuse across protocols (not just HTTP)

**Architecture options:**
```rust
// Option 1: Separate derive
#[derive(OpenApi)]
#[http]
impl Service { }

// Option 2: Explicit attribute
#[http]
#[openapi(title = "My API", version = "1.0.0")]
impl Service { }

// Option 3: Manual generation method
let spec = Service::openapi_spec();
```

**Technical considerations:**
- Extract OpenAPI generation code from `crates/trellis-macros/src/http.rs`
- Create new `crates/trellis-macros/src/openapi.rs` module
- Decide on attribute vs derive approach
- Ensure it can still access HTTP routing information
- Consider composition with `#[http]`, `#[jsonrpc]`, `#[graphql]`

**Design questions:**
- Should `#[openapi]` work independently, or require `#[http]`?
- How to handle OpenAPI-specific attributes (`#[route(hidden)]`, etc.)?
- Should other protocols also generate OpenAPI?

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
