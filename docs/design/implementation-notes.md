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

## Partial / Needs Work

### HTTP Macro (`#[http]`)

Basic structure is there but has issues:
- Parameter extraction generates working but ugly code
- Need to handle the body struct generation better
- Rust 2024 edition has stricter binding mode rules that break some patterns
- SSE streaming not tested

### CLI Macro (`#[cli]`)

Structure is there but:
- Async methods not handled properly
- Type parsing for arguments needs work
- Need to test with clap's actual behavior

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

1. **HTTP macro**: Fix parameter extraction, test with real axum handlers
2. **CLI macro**: Handle async, test with clap
3. **OpenAPI**: Currently inline in HTTP macro, should be separate and more complete
4. **Streaming**: SSE for HTTP, streaming for MCP
5. **Async**: Better async method support across all macros
6. **Tests**: trybuild tests for compile-fail cases
7. **Error derive**: `#[derive(TrellisError)]` for error code mapping
