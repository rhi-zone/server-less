# Trellis Backlog

Prioritized backlog of features, improvements, and ideas.

## High Priority

### Feature Gates in lib.rs
Add `#[cfg(feature = "...")]` guards around macro re-exports so users only pay for what they use.

```rust
#[cfg(feature = "mcp")]
pub use trellis_macros::mcp;
```

### E2E Testing Strategy
Create end-to-end tests that:
1. Define a service with known behavior (the "reference implementation")
2. Apply macros to generate protocol handlers
3. Actually call the handlers and verify results match reference

Example structure:
```rust
// Reference implementation
struct Calculator {
    fn add(&self, a: i32, b: i32) -> i32 { a + b }
}

// Test MCP
let calc = Calculator;
let result = calc.mcp_call("add", json!({"a": 2, "b": 3}));
assert_eq!(result, Ok(json!(5)));

// Test HTTP (with test client)
let app = calc.http_router();
let response = app.oneshot(Request::get("/add?a=2&b=3")).await;
assert_eq!(response.json::<i32>(), 5);

// Test CLI
let output = calc.cli_run_with(["calc", "add", "--a", "2", "--b", "3"]);
assert!(output.contains("5"));
```

### Async Method Support
Currently MCP and WS return errors for async methods. Need to:
- Detect when we're in async context
- Use `block_on` or require async dispatch methods
- Consider feature-gating tokio dependency

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
