# Implementation Notes

Notes from implementing the first set of trellis macros.

## What Works (2025-01-20)

### MCP Macro (`#[mcp]`)

The MCP macro is fully functional:

```rust
#[mcp(namespace = "user")]
impl UserService {
    /// Create a new user
    pub fn create_user(&self, name: String, email: String) -> Result<User, UserError> { ... }
}
```

Generates:
- `mcp_tools()` - Returns JSON tool definitions with schemas
- `mcp_call(name, args)` - Dispatches tool calls to methods
- Proper handling of `Result<T, E>`, `Option<T>`, and plain `T` returns
- Optional parameters via `Option<T>` become non-required in schema

### Core Infrastructure

- **Parsing**: Extracts method signatures, doc comments, parameters, return types
- **Type analysis**: Detects `Result`, `Option`, `Stream`, async, etc.
- **Name inference**: HTTP methods from `create_*`, `get_*`, etc.
- **Error mapping**: `ErrorCode` with HTTP status, gRPC code, CLI exit code

### HTTP Macro (`#[http]`)

The HTTP macro is working:

```rust
#[http(prefix = "/api")]
impl UserService {
    /// List all users
    pub fn list_users(&self) -> Vec<User> { ... }
}
```

Generates:
- `http_router()` - Returns axum Router with all routes
- `openapi_spec()` - Returns OpenAPI 3.0 spec as JSON
- Automatic HTTP method inference from method names
- Path parameter extraction for ID-like parameters
- JSON body extraction for POST/PUT/PATCH
- Query parameter extraction for GET/DELETE

### CLI Macro (`#[cli]`)

The CLI macro is working:

```rust
#[cli(name = "my-cli", version = "1.0.0", about = "My CLI app")]
impl UserService {
    /// List all users
    pub fn list_users(&self) -> Vec<User> { ... }
}
```

Generates:
- `cli_command()` - Returns clap Command with subcommands
- `cli_run()` - Runs the CLI application
- `cli_run_with(args)` - Runs with custom arguments (for testing)
- Async methods handled via tokio runtime
- ID parameters become positional arguments
- JSON output for return values

## Fixed Issues

### Rust 2024 Binding Modes (FIXED)

The 2024 edition has stricter binding mode rules. The issue was in the OpenAPI generation code which used `ref mut` explicitly when matching on `&mut T`:

```rust
// Broke in Rust 2024
if let Value::Object(ref mut map) = path_item { ... }

// Fixed: let Rust infer the binding mode
if let Value::Object(map) = path_item { ... }
```

### CLI Async Support (FIXED)

Async methods now work in CLI via tokio runtime:

```rust
let result = ::tokio::runtime::Runtime::new()
    .expect("Failed to create Tokio runtime")
    .block_on(self.async_method());
```

## Edge Cases Discovered

### Optional Parameters

For `Option<T>` parameters, need to be careful not to double-wrap:
```rust
// Wrong: Option<Option<T>>
let param: Option<T> = args.get("key").and_then(|v| from_value::<Option<T>>(v).ok());

// Right: extract inner, let outer be None if missing
let param: Option<T> = args.get("key").and_then(|v| from_value(v).ok());
```

### ID Parameters

Current heuristic: parameter is "ID-like" if:
- Named `id`
- Ends with `_id`

This affects:
- HTTP: becomes path parameter `/{id}`
- CLI: becomes positional argument

May need to be configurable.

### Rust 2024 Edition

The 2024 edition has stricter binding mode rules. Some patterns that worked in 2021 break:
```rust
// Breaks in 2024 with certain macro expansions
let (x, y) = &some_tuple;  // ref binding modes changed
```

Need to be explicit about references in generated code.

### Attribute Stacking

Multiple macros on same impl:
```rust
#[http]
#[cli]
#[mcp]
impl Service { ... }
```

Each macro processes independently. They don't conflict because each generates different methods. But if they generated methods with same names, there would be conflicts.

## Design Decisions Made

### Attribute Macros vs Derive Macros

Used attribute macros (`#[http]`) on impl blocks instead of derive macros on structs because:
- Methods are the source of truth, not struct fields
- Impl-first design: write methods, project to protocols
- Derive macros on structs would need a different model

### JSON Values for Parameter Passing

MCP uses `serde_json::Value` for arguments because:
- MCP protocol uses JSON
- Flexibility for unknown schemas
- Easy serialization/deserialization

For HTTP, considered generating typed body structs but fell back to `Value` for simplicity. May revisit.

### Error Handling in Generated Code

Generated code uses `.unwrap_or_default()` liberally for missing parameters. This is intentional:
- Keeps generated code simple
- Parameters have defaults if missing
- Explicit errors are returned for required parameters

May want to add stricter validation modes later.

## TODO

### Completed
- [x] HTTP macro: Fixed parameter extraction, works with axum
- [x] CLI macro: Handle async via tokio runtime
- [x] CLI macro: Test with clap's actual behavior
- [x] Tests: Unit and integration tests for all macros

### Remaining
1. **OpenAPI**: Currently inline in HTTP macro, should be separate and more complete
2. **Streaming**: SSE for HTTP, streaming for MCP
3. **Tests**: trybuild tests for compile-fail cases
4. **Error derive**: `#[derive(TrellisError)]` for error code mapping
5. **Attribute customization**: Allow overriding inferred HTTP methods, paths, etc.
