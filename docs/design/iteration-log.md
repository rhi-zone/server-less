# Iteration Log

Living document tracking the evolution of trellis macros.

## Current State (2025-01-20)

### Working Macros
| Macro | Status | Key Features |
|-------|--------|--------------|
| `#[mcp]` | ✅ Solid | Tool schemas, dispatch, optional params |
| `#[http]` | ✅ Working | Axum router, OpenAPI, method inference |
| `#[cli]` | ✅ Working | Clap subcommands, async support, JSON output |

### Shared Patterns Identified

1. **Parameter Extraction** - All three parse params, detect optionality, handle IDs
2. **Return Type Handling** - `Result<T,E>`, `Option<T>`, `Vec<T>`, unit, plain T
3. **Async Detection** - All need to handle sync vs async methods
4. **Doc Comments** - All use `///` docs for descriptions/help text
5. **Method Name Conventions** - `create_*`, `get_*`, `list_*`, etc.

### Current Pain Points

1. **Duplication** - Param extraction logic repeated in each macro
2. **OpenAPI coupling** - Baked into HTTP macro, should be composable
3. **No streaming tests** - `impl Stream<Item=T>` path untested
4. **No error derive** - Manual error code mapping

### Protocols to Explore

| Protocol | Would Test | Complexity |
|----------|-----------|------------|
| gRPC | Streaming, protobuf, error codes | High |
| WebSocket | Bidirectional, connection state | Medium |
| GraphQL | Nested types, query/mutation | Medium |
| SSE | One-way streaming | Low |

---

## Iteration 1: Assess Refactor Opportunity

**Goal:** Determine if shared abstractions would help.

**Analysis:**
Looking at MCP, HTTP, CLI param/return handling:
- Similar structure but different outputs (JSON schema vs axum extractors vs clap args)
- Return handling: MCP → `Result<Value, String>`, HTTP → axum responses, CLI → stdout
- The duplication is conceptual, not copy-paste

**Decision:** Hold on refactor. The patterns aren't converged enough yet. Adding another protocol will give more data points.

---

## Iteration 2: Add WebSocket Support

**Goal:** Add `#[ws]` macro to test bidirectional communication patterns.

**Why WebSocket:**
- Tests connection state (HTTP is stateless, WS has persistent connections)
- Tests bidirectional patterns (send/receive)
- Uses axum, so builds on HTTP infrastructure
- Common need for real-time apps

**Plan:**
- [x] Create `ws.rs` with WebSocket macro
- [x] Handle message types (text, binary, ping/pong)
- [x] Test with simple echo/chat example
- [x] Document patterns that emerge

**Result:**
WebSocket macro implemented using JSON-RPC pattern over WebSocket. Key observations:

1. **WS ≈ MCP**: Both are RPC-over-transport. Method dispatch is nearly identical.
2. **Pattern families emerging**:
   - **RPC**: MCP, WS - dispatch by method name from JSON
   - **REST**: HTTP - dispatch by URL path + verb
   - **CLI**: dispatch by subcommand name
3. **Shared code**: Param extraction and return type handling are copy-paste between WS and MCP
4. **Feature gates needed**: Added features for mcp, http, cli, ws (all enabled by default)

---

## Iteration 3: Refactor RPC Pattern

**Goal:** Extract shared RPC dispatch logic from MCP and WS.

**Observation:** MCP's `generate_dispatch_arm` and WS's `generate_dispatch_arm` are nearly identical:
- Extract params from JSON Value
- Call method with params
- Handle Result/Option/unit/T return types
- Serialize result to JSON

**Plan:**
- [x] Create `rpc.rs` with shared RPC dispatch generation
- [x] Refactor MCP to use shared code
- [x] Refactor WS to use shared code
- [x] Add tests to ensure behavior unchanged
- [x] Add feature gates to lib.rs

**Result:**
Created `rpc.rs` with:
- `generate_param_extraction()` - extract single param from JSON
- `generate_all_param_extractions()` - extract all params
- `generate_method_call()` - call method with async handling options
- `generate_json_response()` - handle Result/Option/unit/T → JSON
- `generate_dispatch_arm()` - complete match arm combining above
- `generate_param_schema()` - JSON schema for params
- `infer_json_type()` - Rust type → JSON schema type

MCP and WS now both use `rpc::generate_dispatch_arm()` - significant code reduction.

Added feature gates:
- `mcp` - MCP macro (no extra deps)
- `http` - HTTP macro (requires axum)
- `cli` - CLI macro (requires clap)
- `ws` - WebSocket macro (requires axum+futures)
- `full` - all of the above (default)

---

## Current Status

| Macro | Status | Tests | Shared Code |
|-------|--------|-------|-------------|
| MCP | ✅ | 8 | rpc.rs |
| HTTP | ✅ | 3 | - |
| CLI | ✅ | 6 | - |
| WS | ✅ | 10 | rpc.rs |
| **Total** | | **27** | |

**Pattern families:**
1. **RPC** (MCP, WS): JSON dispatch, shared via `rpc.rs`
2. **REST** (HTTP): URL routing, response types
3. **CLI**: Subcommand dispatch, argument parsing

---

## Iteration 4: SSE Streaming Support

**Goal:** Verify and fix HTTP macro's SSE streaming support.

**Findings:**
1. SSE streaming code was already in `http.rs` but untested
2. Rust 2024's `impl Trait` lifetime capture rules cause issues:
   - `impl Stream + 'static` still captures `&self` by default
   - Users need `+ use<>` on their method return types
   - Example: `fn stream_data(&self) -> impl Stream + use<>`
3. Added `Box::pin(stream)` to erase concrete type and ensure ownership

**Result:**
- Created `streaming_service.rs` example demonstrating SSE
- SSE works with proper lifetime annotations
- Documented Rust 2024 `+ use<>` requirement

---

## Iteration 5: E2E Tests

**Goal:** Validate generated code produces correct results.

**Approach:**
1. Define `Calculator` with reference implementations (`ref_add`, `ref_divide`, etc.)
2. Wrap in `McpCalculator`, `WsCalculator`, `HttpCalculator`, `CliCalculator`
3. Call through generated handlers, verify results match reference
4. Test error cases (Result::Err), missing cases (Option::None), optional params

**Result:**
- 20 E2E tests added
- Cross-protocol consistency verified (MCP and WS produce identical results)
- Total test count: 53

---

## Iteration 6: Async Method Support

**Goal:** Add proper async method support to MCP and WebSocket macros.

**Problem:**
- Both MCP and WS previously used `AsyncHandling::Error` which rejected async methods
- MCP had a stub `mcp_call_async` that just called the sync version
- Real-world services often need async database calls, HTTP fetches, etc.

**Solution:**
1. Generate both sync and async dispatch arms
2. For sync callers (`mcp_call`, `ws_handle_message`): async methods return error
3. For async callers (`mcp_call_async`, `ws_handle_message_async`): async methods awaited
4. WebSocket connection handler uses async dispatch (supports async methods over real connections)

**Changes:**
- `rpc.rs`: Fixed `generate_dispatch_arm` to handle async error case without generating unreachable code
- `mcp.rs`: Generate both `dispatch_arms_sync` and `dispatch_arms_async`, use in respective methods
- `ws.rs`: Added `ws_handle_message_async` and `ws_dispatch_async`, connection handler uses async
- `ws.rs`: Made `__trellis_ws_connection` unique per struct to avoid conflicts

**Tests Added:**
- MCP: 5 async tests (sync/async methods with sync/async callers)
- WS: 6 async tests (sync/async methods with sync/async handlers)

**Result:**
- Async methods fully supported in MCP and WS
- Backwards compatible: sync callers still work for sync methods
- Real WebSocket connections can call async methods

---

## Current Status Summary

| Component | Status | Tests |
|-----------|--------|-------|
| MCP macro | ✅ Solid | 13 (+ E2E) |
| HTTP macro | ✅ Solid | 3 (+ E2E) |
| CLI macro | ✅ Solid | 6 (+ E2E) |
| WS macro | ✅ Solid | 16 (+ E2E) |
| RPC utilities | ✅ Shared | - |
| Feature gates | ✅ Working | - |
| SSE streaming | ✅ Working | - |
| Async support | ✅ Working | - |
| **Total tests** | | **64** |

**Remaining work:**
1. Documentation improvements
2. GraphQL if more protocol coverage needed
3. Error derive macro
4. "Serve" coordination pattern

---

## Iteration 7: Better Error Messages

**Goal:** Improve macro error messages with proper spans and helpful suggestions.

**Changes:**
1. **Unknown attribute arguments** now list valid options:
   - `unknown argument 'badarg'. Valid arguments: namespace` (MCP)
   - `unknown argument 'badarg'. Valid arguments: prefix` (HTTP)
   - `unknown argument 'badarg'. Valid arguments: name, version, about` (CLI)
   - `unknown argument 'badarg'. Valid arguments: path` (WS)

2. **Associated functions without `&self`** (constructors, etc.) are silently skipped, not errored.

3. **Unsupported parameter patterns** now report errors instead of silently skipping:
   - `unsupported parameter pattern. Use a simple identifier like 'name: String'`

All errors use `syn::Error::new_spanned()` to point to the exact problematic code.

---

## Current Status Summary

| Component | Status | Tests |
|-----------|--------|-------|
| MCP macro | ✅ Solid | 13 (+ E2E) |
| HTTP macro | ✅ Solid | 3 (+ E2E) |
| CLI macro | ✅ Solid | 6 (+ E2E) |
| WS macro | ✅ Solid | 16 (+ E2E) |
| RPC utilities | ✅ Shared | - |
| Feature gates | ✅ Working | - |
| SSE streaming | ✅ Working | - |
| Async support | ✅ Working | - |
| Error messages | ✅ Improved | - |
| **Total tests** | | **64** |

---

## Iteration 8: Documentation Improvements

**Goal:** Update documentation to reflect all implemented features.

**Changes:**
1. **lib.rs crate docs** - Comprehensive overview including:
   - All 4 macros (HTTP, CLI, MCP, WebSocket)
   - Generated methods table
   - Async method support with examples
   - SSE streaming with Rust 2024 `+ use<>` note
   - Feature flags documentation

2. **README.md** - Updated from placeholder to real examples:
   - Quick start with all 4 macros
   - Generated code table showing what each macro produces
   - Feature highlights
   - Installation with feature flags

---

## Current Status Summary

| Component | Status | Tests |
|-----------|--------|-------|
| MCP macro | ✅ Solid | 13 (+ E2E) |
| HTTP macro | ✅ Solid | 3 (+ E2E) |
| CLI macro | ✅ Solid | 6 (+ E2E) |
| WS macro | ✅ Solid | 16 (+ E2E) |
| RPC utilities | ✅ Shared | - |
| Feature gates | ✅ Working | - |
| SSE streaming | ✅ Working | - |
| Async support | ✅ Working | - |
| Error messages | ✅ Improved | - |
| Documentation | ✅ Updated | - |
| **Total tests** | | **64** |

---

## Iteration 9: Error Derive Macro

**Goal:** Add `#[derive(TrellisError)]` for error types with protocol-agnostic codes.

**Features:**
- Implements `IntoErrorCode`, `Display`, and `std::error::Error`
- `#[error(code = NotFound)]` - explicit ErrorCode variant
- `#[error(code = 404)]` - HTTP status (mapped to ErrorCode)
- `#[error(message = "...")]` - custom message
- Code inference from variant name (e.g., `Unauthorized` → `Unauthenticated`)
- Supports unit, tuple, and struct variants

**Example:**
```rust
#[derive(TrellisError)]
enum MyError {
    #[error(code = NotFound, message = "User not found")]
    UserNotFound,
    #[error(code = 400)]
    ValidationFailed(String),
    Unauthorized,  // inferred from name
}
```

**Tests:** 10 new tests, 74 total.

---

## Current Status Summary

| Component | Status | Tests |
|-----------|--------|-------|
| MCP macro | ✅ Solid | 13 (+ E2E) |
| HTTP macro | ✅ Solid | 3 (+ E2E) |
| CLI macro | ✅ Solid | 6 (+ E2E) |
| WS macro | ✅ Solid | 16 (+ E2E) |
| Error derive | ✅ NEW | 10 |
| RPC utilities | ✅ Shared | - |
| Feature gates | ✅ Working | - |
| SSE streaming | ✅ Working | - |
| Async support | ✅ Working | - |
| Error messages | ✅ Improved | - |
| Documentation | ✅ Updated | - |
| **Total tests** | | **74** |

---

## Iteration 10: Attribute Customization

**Goal:** Allow per-method HTTP route customization via `#[route(...)]`.

**Features:**
- `#[route(method = "POST")]` - override HTTP method
- `#[route(path = "/custom")]` - override path
- `#[route(skip)]` - exclude from HTTP router and OpenAPI
- `#[route(hidden)]` - include in router but hide from OpenAPI

**Example:**
```rust
#[http(prefix = "/api")]
impl MyService {
    #[route(method = "POST", path = "/custom")]
    fn my_method(&self) { }

    #[route(skip)]
    fn internal_only(&self) { }
}
```

**Implementation:**
- Added `#[route]` passthrough macro (inner attributes on methods)
- `HttpMethodOverride` struct parses method attributes
- Override logic in `generate_route` and `generate_openapi_spec`

**Tests:** 7 new tests, 81 total.

---

## Iteration 11: OpenAPI Improvements

**Goal:** Enhance OpenAPI spec generation with proper parameter schemas and response types.

**Features:**
- **Query parameter schemas**: Type inference for GET/DELETE params (integer, string, boolean)
- **Path parameter schemas**: ID params marked as path params with proper types
- **Request body schemas**: POST/PUT/PATCH get request body with properties and required fields
- **Error responses**: Result return types generate 200/400/500 responses

**Implementation Details:**
1. Added `extract_option_inner()` helper to get inner type from `Option<T>` (avoids double-wrapping)
2. Made handler names unique per struct (`__trellis_http_{struct}_{method}`) to support multiple services
3. Fixed parameter type handling for optional params (parse as inner type, not `Option<T>`)
4. Enhanced `generate_openapi_spec` with proper parameter/body/response generation

**Tests Added:**
- `test_openapi_query_parameters` - verifies page/limit query params
- `test_openapi_path_parameters` - verifies item_id path param
- `test_openapi_request_body` - verifies POST body schema
- `test_openapi_error_responses` - verifies Result generates error codes

**Total:** 4 new OpenAPI tests, 85 total tests.

---

## Current Status Summary

| Component | Status | Tests |
|-----------|--------|-------|
| MCP macro | ✅ Solid | 13 (+ E2E) |
| HTTP macro | ✅ Enhanced | 14 (+ E2E) |
| CLI macro | ✅ Solid | 6 (+ E2E) |
| WS macro | ✅ Solid | 16 (+ E2E) |
| Error derive | ✅ Working | 10 |
| Route attr | ✅ Working | - |
| OpenAPI schemas | ✅ NEW | - |
| RPC utilities | ✅ Shared | - |
| Feature gates | ✅ Working | - |
| SSE streaming | ✅ Working | - |
| Async support | ✅ Working | - |
| Error messages | ✅ Improved | - |
| Documentation | ✅ Updated | - |
| **Total tests** | | **85** |

---

## Iteration 12: Serve Coordination Pattern

**Goal:** Combine multiple protocol handlers into a single server.

**Features:**
- `#[serve(http, ws)]` - combine HTTP and WebSocket routers
- `#[serve(http)]` or `#[serve(ws)]` - single protocol
- `#[serve(http, health = "/healthz")]` - custom health check path
- Generates `serve(addr)` async method and `router()` builder

**Example:**
```rust
#[http]
#[ws]
#[serve(http, ws)]
impl MyService {
    fn list_items(&self) -> Vec<String> { vec![] }
}

// Start server
service.serve("0.0.0.0:3000").await?;

// Or get router for custom setup
let router = service.router();
```

**Implementation:**
- New `serve.rs` module with `ServeArgs` parser
- Handles Clone requirement by cloning before passing to routers
- Auto-adds `/health` endpoint (configurable)
- Both `serve()` and `router()` methods for flexibility

**Tests:** 6 new tests, 91 total.

---

## Current Status Summary

| Component | Status | Tests |
|-----------|--------|-------|
| MCP macro | ✅ Solid | 13 (+ E2E) |
| HTTP macro | ✅ Enhanced | 14 (+ E2E) |
| CLI macro | ✅ Solid | 6 (+ E2E) |
| WS macro | ✅ Solid | 16 (+ E2E) |
| Serve macro | ✅ NEW | 6 |
| Error derive | ✅ Working | 10 |
| Route attr | ✅ Working | - |
| OpenAPI schemas | ✅ Working | - |
| RPC utilities | ✅ Shared | - |
| Feature gates | ✅ Working | - |
| SSE streaming | ✅ Working | - |
| Async support | ✅ Working | - |
| Error messages | ✅ Improved | - |
| Documentation | ✅ Updated | - |
| **Total tests** | | **91** |

---

## Iteration 13: GraphQL Macro

**Goal:** Add GraphQL support using async-graphql's dynamic schema API.

**Features:**
- `#[graphql]` attribute on impl blocks
- Query/Mutation inference from method names (same as HTTP)
- Dynamic schema generation (no nested proc macros)
- Generates `graphql_schema()`, `graphql_router()`, `graphql_sdl()`

**Example:**
```rust
#[graphql]
impl MyService {
    fn get_user(&self, id: String) -> User { }      // Query
    fn create_user(&self, name: String) -> User { } // Mutation
}

let schema = service.graphql_schema();
let sdl = service.graphql_sdl();
let router = service.graphql_router(); // serves /graphql with playground
```

**Implementation Notes:**
- Uses async-graphql's dynamic schema API (not derive macros) to avoid proc macro nesting issues
- Type mapping simplified for now (returns String) - proper type registration TBD
- Handles query-only services (no empty mutation type)

**Tests:** 4 new tests, 95 total.

---

## Current Status Summary

| Component | Status | Tests |
|-----------|--------|-------|
| MCP macro | ✅ Solid | 13 (+ E2E) |
| HTTP macro | ✅ Enhanced | 14 (+ E2E) |
| CLI macro | ✅ Solid | 6 (+ E2E) |
| WS macro | ✅ Solid | 16 (+ E2E) |
| Serve macro | ✅ Working | 6 |
| GraphQL macro | ✅ Basic | 4 |
| Error derive | ✅ Working | 10 |
| Route attr | ✅ Working | - |
| OpenAPI schemas | ✅ Working | - |
| RPC utilities | ✅ Shared | - |
| Feature gates | ✅ Working | - |
| SSE streaming | ✅ Working | - |
| Async support | ✅ Working | - |
| Error messages | ✅ Improved | - |
| Documentation | ✅ Updated | - |
| **Total tests** | | **95** |

---

## Iteration 14: gRPC Proto Generation

**Goal:** Impl-first protobuf schema generation for gRPC.

**Approach:**
Since tonic is compile-time focused (unlike async-graphql's dynamic API), we generate
.proto schema files that users can then use with standard tonic-build tooling.

**Features:**
- `#[grpc]` or `#[grpc(package = "my.package")]`
- Generates `proto_schema() -> &'static str`
- Generates `write_proto(path)` helper
- Maps Rust types to protobuf (string, int32, bool, etc.)
- Optional parameters become `optional` fields
- Doc comments preserved in schema

**Example:**
```rust
#[grpc(package = "users.v1")]
impl UserService {
    /// Get user by ID
    fn get_user(&self, id: String) -> User { }
}

// Get proto schema
let proto = UserService::proto_schema();

// Write to file
UserService::write_proto("proto/users.proto")?;
```

**Generated proto:**
```protobuf
syntax = "proto3";
package users.v1;

service UserService {
  // Get user by ID
  rpc GetUser(GetUserRequest) returns (GetUserResponse);
}

message GetUserRequest {
  string id = 1;
}
message GetUserResponse {
  string result = 1;
}
```

**Tests:** 8 new tests, 103 total.

---

## Iteration 15: GraphQL Type Improvements

**Goal:** Fix GraphQL type mapping and make resolvers actually call methods.

**Before:**
- All return types mapped to String
- Resolvers returned "todo" placeholder

**After:**
- Proper scalar type mapping (String, Int, Float, Boolean)
- List types supported (Vec<T> -> [T])
- Resolvers extract arguments from context
- Resolvers call actual service methods
- Results converted to GraphQL values

**Example resolver now generates:**
```rust
FieldFuture::new(async move {
    let name: String = ctx.args.try_get("name")?.deserialize()?;
    let result = service.create_item(name);
    Ok(Some(value_to_graphql(result)))
})
```

**Tests:** 5 new execution tests, 108 total.

---

## Iteration 16: gRPC Schema Validation

**Goal:** Schema-first mode - validate impl against expected .proto file.

**Features:**
- `#[grpc(schema = "path/to/expected.proto")]` - validate against expected schema
- `validate_schema() -> Result<(), String>` - returns diff if mismatch
- `assert_schema_matches()` - panics on mismatch

**Example:**
```rust
// Lock down your API contract
#[grpc(package = "users.v1", schema = "proto/users.proto")]
impl UserService {
    fn get_user(&self, id: String) -> User { }
}

// In tests
#[test]
fn api_contract_stable() {
    UserService::assert_schema_matches();
}
```

**Workflow:**
1. Start impl-first: `#[grpc]` generates .proto
2. Write proto to file: `Service::write_proto("proto/service.proto")?`
3. Lock it down: add `schema = "proto/service.proto"`
4. Now changes to impl that break the schema will fail validation

**Tests:** 3 new validation tests, 111 total.

---

## Iteration 17: Cap'n Proto Support

**Goal:** Generate Cap'n Proto `.capnp` schemas from impl blocks.

**Features:**
- `#[capnp]` - generate schema from impl
- `#[capnp(id = "0x...")]` - set schema ID (required for production)
- `capnp_schema() -> &'static str` - get schema string
- `write_capnp(path)` - write to file
- Schema validation like gRPC: `schema = "path.capnp"`, `validate_schema()`, `assert_schema_matches()`

**Type Mappings:**
| Rust | Cap'n Proto |
|------|-------------|
| String, &str | Text |
| i8/i16/i32/i64 | Int8/Int16/Int32/Int64 |
| u8/u16/u32/u64 | UInt8/UInt16/UInt32/UInt64 |
| f32/f64 | Float32/Float64 |
| bool | Bool |
| Vec<u8> | Data |
| Vec<T> | List(Text) |
| () | Void |

**Schema Structure:**
```
@0x85150b117366d14b;

interface MyService {
  # Doc comment becomes Cap'n Proto comment
  getUser @0 (GetUserParams) -> (GetUserResult);
}

struct GetUserParams {
  id @0 :Text;
}

struct GetUserResult {
  value @0 :Text;
}
```

**Tests:** 10 new tests, 121 total.

---

## Iteration 18: JSON-RPC over HTTP

**Goal:** JSON-RPC 2.0 over HTTP (same protocol as WS, different transport).

**Features:**
- `#[jsonrpc]` - generate JSON-RPC HTTP handler
- `#[jsonrpc(path = "/rpc")]` - custom endpoint path
- `jsonrpc_router()` - axum Router with POST endpoint
- `jsonrpc_handle(request)` - handle JSON-RPC requests
- `jsonrpc_methods()` - list available methods

**JSON-RPC 2.0 Compliance:**
- Version validation (`"jsonrpc": "2.0"`)
- Proper error codes (-32600, -32603, etc.)
- Batch requests (array of requests)
- Notifications (no `id` field = no response)

**Example:**
```rust
#[jsonrpc]
impl Calculator {
    fn add(&self, a: i32, b: i32) -> i32 { a + b }
}

// POST /rpc
// {"jsonrpc": "2.0", "method": "add", "params": {"a": 1, "b": 2}, "id": 1}
// => {"jsonrpc": "2.0", "result": 3, "id": 1}

// Batch:
// [{"jsonrpc": "2.0", "method": "add", ...}, {"jsonrpc": "2.0", "method": "multiply", ...}]
// => [{"result": 3, ...}, {"result": 12, ...}]
```

**Tests:** 11 new tests, 132 total.

---

## Iteration 19: Extended Serve Coordination

**Goal:** Support more protocols in the `#[serve]` macro.

**Added Protocols:**
- `jsonrpc` - JSON-RPC over HTTP router
- `graphql` - GraphQL router

**Example:**
```rust
#[http]
#[jsonrpc]
#[graphql]
#[serve(http, jsonrpc, graphql)]
impl MyService { ... }

// Combines all three routers + health check
let router = service.router();
```

**Tests:** 2 new tests, 134 total.

---

## Iteration 20: Thrift Schema Generation

**Goal:** Generate Apache Thrift `.thrift` schemas from impl blocks.

**Features:**
- `#[thrift]` - generate schema from impl
- `#[thrift(namespace = "users")]` - set namespace
- `thrift_schema() -> &'static str` - get schema string
- `write_thrift(path)` - write to file
- Schema validation like gRPC/Cap'n Proto

**Type Mappings:**
| Rust | Thrift |
|------|--------|
| String, &str | string |
| i8 | byte |
| i16/i32/i64 | i16/i32/i64 |
| f64 | double |
| bool | bool |
| Vec<u8> | binary |
| Vec<T> | list<string> |
| HashMap | map<string, string> |
| () | void |

**Tests:** 9 new tests, 143 total.

---

## Current Status Summary

| Component | Status | Tests |
|-----------|--------|-------|
| MCP macro | ✅ Solid | 13 (+ E2E) |
| HTTP macro | ✅ Enhanced | 14 (+ E2E) |
| CLI macro | ✅ Solid | 6 (+ E2E) |
| WS macro | ✅ Solid | 16 (+ E2E) |
| JSON-RPC macro | ✅ Working | 11 |
| Serve macro | ✅ Working | 6 |
| GraphQL macro | ✅ Working | 9 |
| gRPC (impl + schema) | ✅ Working | 11 |
| Cap'n Proto (impl + schema) | ✅ Working | 10 |
| Thrift (impl + schema) | ✅ Working | 9 |
| Error derive | ✅ Working | 10 |
| Route attr | ✅ Working | - |
| OpenAPI schemas | ✅ Working | - |
| RPC utilities | ✅ Shared | - |
| Feature gates | ✅ Working | - |
| SSE streaming | ✅ Working | - |
| Async support | ✅ Working | - |
| Error messages | ✅ Improved | - |
| Documentation | ✅ Updated | - |
| **Total tests** | | **111** |

---

## Future Iterations

(To be filled as we go)

### Schema-based Protocols (Design Challenge)

Cap'n Proto and Protobuf/gRPC are **schema-first** protocols, which inverts trellis's impl-first approach:

| Approach | Impl-first (current) | Schema-first |
|----------|---------------------|--------------|
| Flow | Rust impl → protocol | .proto/.capnp → Rust |
| Examples | HTTP, MCP, WS, CLI | gRPC, Cap'n Proto |

**Design decision: Bidirectional**

Support both directions - impl-first AND schema-first:

```rust
// Direction 1: Impl-first (generate schema)
#[grpc]
impl MyService {
    fn get_user(&self, id: String) -> User { }
}
// Generates: service.proto, grpc_router(), etc.

// Direction 2: Schema-first (validate against schema)
#[grpc(schema = "service.proto")]
impl MyService {
    // Macro validates methods match schema
    // Compile error if method signature doesn't match
}

// Direction 3: Schema-first with trait generation
#[derive(GrpcService)]
#[grpc(schema = "service.proto")]
struct MyService;
// Generates: trait MyServiceRpc { fn get_user(...) }
// User implements trait, gets type safety from schema
```

**Why bidirectional:**
- Impl-first for rapid prototyping, internal services
- Schema-first for interop with existing systems, contract-first teams
- Progressive: start impl-first, export schema, switch to schema-first when stabilized

**Philosophy: We're not here to judge, just to help.**
Users have their own workflows, constraints, and preferences. Trellis supports them, not the other way around.

**Protocols to explore:**
- gRPC (protobuf) - streaming, error codes, widely used
- Cap'n Proto - zero-copy, RPC built-in, mentioned in origin story
- Thrift - if there's demand
