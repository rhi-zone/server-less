# Trellis Backlog

Prioritized backlog of features, improvements, and ideas.

## Completed

### ✅ Feature Gates in lib.rs
Added `#[cfg(feature = "...")]` guards around macro re-exports.
Features: `mcp`, `http`, `cli`, `ws`, `full` (default = all).

### ✅ E2E Testing Strategy
Implemented in `tests/e2e_tests.rs`:
- Reference implementations in `Calculator` struct
- Protocol wrappers (`McpCalculator`, `WsCalculator`, etc.)
- Cross-protocol consistency tests

### ✅ Async Method Support
MCP and WS now support async methods:
- `mcp_call` / `ws_handle_message`: sync callers, error on async methods
- `mcp_call_async` / `ws_handle_message_async`: async callers, await async methods
- WebSocket connections use async dispatch (real connections work with async)

## High Priority

### Better Error Messages with Spans
Use `syn::Error` with proper spans for better compiler error messages.

### Documentation
- Improve inline docs
- Add more examples
- Document Rust 2024 `+ use<>` requirement for streaming

## Medium Priority

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

### OpenAPI Improvements
- Extract from HTTP macro into separate composable derive
- Add parameter schemas
- Add response schemas
- Support for `#[openapi(hidden)]` to exclude endpoints

### gRPC Support
Add `#[grpc]` macro for tonic/protobuf generation. Would test:
- Protocol buffer schema generation
- Streaming (unary, server, client, bidirectional)
- Error code mapping to gRPC status codes

## Low Priority

### GraphQL Support
Add `#[graphql]` macro. Would test:
- Query vs Mutation distinction
- Nested type resolution
- N+1 problem considerations

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
Allow overriding inferred behavior:
```rust
#[http]
impl Service {
    #[http(method = "POST", path = "/custom")]
    fn my_method(&self) { }
}
```

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
