# Server-less TODO

Prioritized backlog of pending features and improvements.

> **Note:** For completed items, see [CHANGELOG.md](./CHANGELOG.md)

---

## Queue

### OpenAPI Composition

- [x] **Phase 2: Per-protocol OpenAPI methods** ✅
  - [x] Add `http_openapi_paths() -> Vec<OpenApiPath>` to `#[http]`
  - [x] Add `jsonrpc_openapi_paths() -> Vec<OpenApiPath>` to `#[jsonrpc]`
  - [x] Add `graphql_openapi_paths() -> Vec<OpenApiPath>` to `#[graphql]`
  - [x] Add `ws_openapi_paths() -> Vec<OpenApiPath>` to `#[ws]`

- [x] **Phase 3: Serve integration** ✅
  - [x] Parse `openapi` in `#[serve]` args
  - [x] Generate combined `openapi_spec()` from detected protocols
  - [x] Support `#[serve(openapi = false)]` opt-out

- [x] **Phase 4: Protocol-aware #[openapi]** ✅
  - [x] Detect sibling protocol attributes (`#[http]`, `#[jsonrpc]`, etc.)
  - [x] Generate combined spec when multiple protocols present
  - Note: `#[openapi]` must be placed FIRST to detect sibling protocols

### OpenAPI Improvements

- [ ] Add richer parameter schemas (description, examples, enum values)
- [x] Add response descriptions via `#[response(description = "...")]` ✅
- [x] Support `#[route(hidden)]` to exclude specific endpoints ✅
- [x] Support `#[route(tags = "users,admin")]` for grouping ✅
- [x] Support `#[route(deprecated)]` for deprecation warnings ✅
- [x] Support `#[route(description = "...")]` for extended descriptions ✅

### GraphQL Improvements

- [x] Nested type resolution for complex relationships ✅
- [x] Custom scalar support (DateTime, UUID, Url, JSON) ✅
- [ ] Subscription support for real-time updates (requires WebSocket integration)
- [x] Input types for mutations (`#[graphql_input]` + `#[graphql(inputs(...))]`) ✅
- [x] Enum type support (`#[graphql_enum]` + `#[graphql(enums(...))]`) ✅
- [ ] Interface/union type support

### Error Handling

- [x] Schema validation returns `Result<(), SchemaValidationError>` ✅
  - `validate_schema()` returns Result (grpc, capnp, thrift, smithy)
  - `assert_schema_matches()` panics for test convenience (intentional)
- [x] Add "help" hints to `SchemaValidationError` ✅
- [x] Add span information to HTTP path validation errors ✅

### Streaming

- [ ] MCP streaming responses (progressive tool output)
- [x] gRPC server streaming (proto3 `stream` keyword generation) ✅
- [ ] gRPC client streaming
- [ ] gRPC bidirectional streaming

### Schema Sharing

- [ ] Define common `SchemaType` enum (String, Int, Bool, Array, Object, etc.)
- [ ] Add `fn schema() -> SchemaType` to method return types
- [ ] Render `SchemaType` to OpenAPI JSON Schema
- [ ] Render `SchemaType` to MCP tool input schema
- [ ] Render `SchemaType` to GraphQL type
- [ ] Validate schema consistency across protocols

### Middleware System

- [ ] Design middleware trait/interface
- [ ] `#[middleware(name)]` attribute on impl blocks
- [ ] Before-request hook
- [ ] After-request hook
- [ ] Error transformation hook
- [ ] Tower layer integration
- [ ] Async middleware support

### Context Extensions

- [ ] Design extensible Context API
- [ ] `Context::get::<T>()` for typed access
- [ ] `Context::insert()` for middleware to add data
- [ ] Extract user ID from JWT in middleware, access via `ctx.user_id()`
- [ ] Document patterns for auth, multi-tenancy, tracing

### API Versioning

- [ ] `#[http(version = "v1")]` for URL prefix versioning
- [ ] `#[http(version_header = "X-API-Version")]` for header versioning
- [ ] `#[deprecated(since = "v2", note = "Use X instead")]` warnings
- [ ] Version-aware OpenAPI spec generation

### Client Generation

- [ ] TypeScript client generator from OpenAPI spec
- [ ] Python client generator from OpenAPI spec
- [ ] Rust client generator from OpenAPI spec
- [ ] CLI tool: `server-less generate-client --lang ts --output ./client`

### gRPC Runtime

- [ ] tonic integration for `#[grpc]`
- [ ] Generate server trait implementation
- [ ] Generate client stub
- [ ] Error code mapping (Rust errors → gRPC status codes)
- [ ] Metadata/header support

### Better Diagnostics

- [ ] "Did you mean X?" suggestions for typos
- [ ] "Add `async` to use `.await`" hints
- [ ] Show snippet of generated code in complex errors
- [ ] Warn about unused parameters
- [ ] Warn about methods that could be `&self` instead of `&mut self`

### Development Tools

- [ ] `#[http(debug = true)]` verbose request/response logging
- [ ] `#[http(trace = true)]` parameter extraction tracing
- [ ] Print generated code to stderr with env var `SERVER_LESS_DEBUG=1`
- [ ] Hot reloading exploration

### Performance

- [ ] Benchmark HTTP macro vs hand-written axum
- [ ] Benchmark CLI macro vs hand-written clap
- [ ] Measure compile-time overhead
- [ ] Optimize generated code size
- [ ] Reduce monomorphization where possible

### Authentication/Authorization

- [ ] `#[auth(required)]` attribute
- [ ] `#[auth(roles = ["admin"])]` for role-based access
- [ ] Bearer token extraction to Context
- [ ] JWT validation middleware
- [ ] API key validation middleware

### "Server" Blessed Preset

- [ ] Design `#[derive(Server)]` expansion
- [ ] `ServerCore` trait for base functionality
- [ ] `OpenApi` derive for spec generation
- [ ] `Metrics` derive for prometheus metrics
- [ ] `HealthCheck` derive for `/health` endpoint
- [ ] `#[server(openapi = false)]` toggle

### IDE Integration

- [ ] rust-analyzer proc-macro expansion hints
- [ ] Go-to-definition for generated methods
- [ ] Hover documentation for attributes
- [ ] Code action: "Add #[http] to impl block"

---

## Ideas / Research

These need more design work before implementation:

- [ ] Code-first → Schema-first transition tooling
- [ ] Automatic API migration generation between versions
- [ ] Contract testing framework
- [ ] Distributed tracing (OpenTelemetry integration)
- [ ] GraphQL Federation support
- [ ] WebAssembly target support
- [ ] no_std support for embedded

---

## Completed

Moved to [CHANGELOG.md](./CHANGELOG.md):
- ✅ WebSocket bidirectional patterns (WsSender)
- ✅ Context injection (HTTP, CLI, JSON-RPC, WebSocket)
- ✅ OpenAPI standalone macro and feature flag
- ✅ OpenApiBuilder for spec composition (Phase 1)
- ✅ Route/response/param attributes
- ✅ SSE streaming for HTTP
- ✅ Compile-time path validation
- ✅ Duplicate route detection
