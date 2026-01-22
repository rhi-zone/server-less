# Trellis TODO

Prioritized backlog of pending features, improvements, and ideas.

> **Note:** For completed items, see [CHANGELOG.md](./CHANGELOG.md)

**Current Status:** 187 tests passing, all clippy checks clean

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

### Parameter Customization
**Feature:** `#[param(query, name="q")]` and related attributes

Currently, parameter names in URLs must match function parameter names. Users should be able to customize:
- Parameter location (query, path, body, header)
- Parameter name in the protocol vs code
- Default values and required/optional overrides

**Desired usage:**
```rust
#[http]
impl SearchService {
    // Query param "q" maps to function param "query"
    async fn search(
        &self,
        #[param(query, name = "q")] query: String,
        #[param(query, name = "limit", default = 10)] max_results: i32,
        #[param(header, name = "X-API-Key")] api_key: String,
    ) -> Vec<SearchResult> {
        // ...
    }
}
```

**Technical considerations:**
- Extend `ParamInfo` in `crates/trellis-parse/src/lib.rs` with location, wire_name, default_value
- Update parameter extraction in all protocol macros (http, jsonrpc, graphql, cli)
- Parse `#[param(...)]` attributes from function parameters
- Update OpenAPI generation to reflect custom names

### Response Customization
**Feature:** `#[response(status = 201)]` and related attributes

Currently response handling is inferred from return types. Users should be able to customize:
- HTTP status codes (201 Created, 202 Accepted, 204 No Content, etc.)
- Response headers
- Content-Type overrides

**Desired usage:**
```rust
#[http]
impl UserService {
    /// Create a new user (returns 201 Created)
    #[response(status = 201)]
    async fn create_user(&self, user: User) -> User {
        // ...
    }

    /// Delete user (returns 204 No Content)
    #[response(status = 204)]
    async fn delete_user(&self, id: String) {
        // ...
    }

    /// Download file
    #[response(content_type = "application/octet-stream")]
    #[response(header = "Content-Disposition", value = "attachment")]
    async fn download(&self, id: String) -> Vec<u8> {
        // ...
    }
}
```

**Technical considerations:**
- Parse `#[response(...)]` attributes from methods
- Extend `MethodInfo` or create `ResponseOverride` in trellis-parse
- Update `generate_response_handling()` in `crates/trellis-macros/src/http.rs`
- Update OpenAPI generation with correct status codes

### Error Derive Macro
```rust
#[derive(TrellisError)]
enum MyError {
    #[error(code = 404)]
    NotFound,
    #[error(code = 401)]
    Unauthorized,
}
```

### Streaming Support
- SSE for HTTP (already partially there)
- Streaming responses for MCP
- WebSocket already handles bidirectional, but could support server-push patterns

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

### Versioning
API versioning support across protocols.
