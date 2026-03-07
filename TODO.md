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

- [x] Wire `ParamInfo::help_text` into `OpenApiParameter::description` in `openapi_gen.rs` ✅ `openapi_gen.rs` reads `param.help_text`; test assertion active in `http_tests.rs::test_param_help_route_in_openapi`.
- [ ] Add richer parameter schemas (description, examples, enum values)
- [x] Add response descriptions via `#[response(description = "...")]` ✅
- [x] Support `#[route(hidden)]` to exclude specific endpoints ✅
- [x] Support `#[route(tags = "users,admin")]` for grouping ✅
- [x] Support `#[route(deprecated)]` for deprecation warnings ✅
- [x] Doc comments → summary (first line) + description (full text) ✅

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

### Mount Points — Per-Protocol Projections

- [x] Shared detection: `is_reference` / `reference_inner` on `ReturnInfo` (parse crate) ✅
- [x] `CliSubcommand` trait + CLI mount support (static & slug) ✅
- [x] `HttpMount` trait — `fn users(&self) -> &Users` → route prefix `/users/...` ✅
- [x] `McpNamespace` trait — `fn users(&self) -> &Users` → tool prefix `users_*` ✅
- [x] `WsMount` trait — WebSocket JSON-RPC namespace delegation ✅
- [x] `JsonRpcMount` trait — JSON-RPC method namespace delegation ✅

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

> **Not planned here** — spun out to `rhizone/normalize`. Client generation (TS/Python/Rust stubs from OpenAPI) lives there.

### gRPC Runtime

> **Server side is in scope** — `#[grpc]` projects an impl block onto a tonic gRPC server, same model as `#[http]` → axum. Client stub generation from proto files lives in `rhizone/normalize`.

- [ ] tonic integration for `#[grpc]`
- [ ] Generate server trait implementation
- [ ] Error code mapping (Rust errors → gRPC status codes)
- [ ] Metadata/header support

### Implementor DX — Capability Discovery

How do implementors know what server-less can do, at the moment they need it?

**Compiler-driven discovery:**
- [x] "Did you mean X?" suggestions for attribute typos ✅ Levenshtein (≤2) across all 10 attribute parsers.
- [ ] "Add `async` to use `.await`" hints
- [ ] Show snippet of generated code in complex errors
- [ ] Contextual hints at expansion time — **not feasible** for cross-item cases (e.g. "add `#[http]` to `Users`") since proc macros only see the item they're applied to. Intra-item patterns (malformed signatures, conflicting attrs) are covered by existing errors.

**Inline examples in diagnostics:**
- [ ] Error messages include short code snippets showing the fix
- [ ] Warnings include "try this:" with corrected attribute usage

**Introspection tooling:**
- [ ] `cargo serverless explain <topic>` CLI — dumps available attributes, inferred behaviors
- [x] `SERVER_LESS_DEBUG=1` env var prints generated code to stderr ✅
- [x] `#[http(debug = true)]` verbose request/response logging ✅
- [x] `#[http(trace = true)]` parameter extraction tracing ✅

**Capability-oriented docs (lower priority):**
- [ ] Capability index page organized by goal, not by macro
- [ ] VitePress "how-to" cookbook layer mapping goals → solutions

**Other diagnostics:**
- [ ] Warn about unused parameters
- > `&mut self` → `&self` warning is clippy's job, not ours.

### Development Tools

> Hot reloading and `cargo serverless` subcommand deferred indefinitely — not a macro concern at this stage.

### Authentication/Authorization

> **Not planned here** — auth is intentionally an escape hatch / third-party extension (e.g. a `server-less-auth` crate). Baking `#[auth]` into core contradicts the design philosophy; users compose their own Tower layers.

### "Server" Blessed Preset

- [x] `#[server]` attribute macro ✅ — `expand_server` in `server.rs`; composes `#[http]` + `#[serve]` + optional OpenAPI; 8 tests in `server_tests.rs`.
- [x] `#[server(openapi = false)]` toggle ✅
- [x] `#[server(health = "/healthz")]` custom health endpoint ✅
- [ ] `ServerCore` trait for base functionality — future; requires design
- [ ] `OpenApi` derive for spec generation — future; `#[openapi]` attr already covers most cases
- [ ] `Metrics` derive for prometheus metrics — future
- [ ] `HealthCheck` derive for `/health` endpoint — future; `/health` already included in `#[server]`

### IDE Integration

> **Not planned here** — requires rust-analyzer plugin / LSP work outside this repo. Server-less improves span quality (which helps IDEs) but can't drive IDE integration itself.

### Polish & Hardening

- [x] Add `trybuild` compile-fail tests (3 fixtures: missing_self, invalid_http_arg, duplicate_route)
- [x] Add unit tests for `server-less-parse` (35 tests) and `server-less-rpc` (39 tests)
- [x] Add HTTP round-trip tests via `axum::TestClient` ✅
- [x] Implement `http_mount_openapi_paths()` (populates from method info at macro expansion time)
- [x] Wire up CLI context injection (was already wired — removed stale `#[allow(dead_code)]`)
- [x] Replace `.unwrap()` with `.expect("BUG: ...")` in generated code (ws.rs, openapi_gen.rs)
- [x] Fix `strip_first_impl()` silently dropping code on parse failure (now emits `compile_error!`)
- [x] Add `--all-features` and `--no-default-features` CI checks (already present in CI)
- [x] Add MSRV CI job (rust-version = 1.85, separate `msrv` job with `cargo check`)
- [x] Add `cargo doc --no-deps` CI check for doc warnings (already present in CI)
- [x] Add examples for blessed presets (`server_preset.rs`, `rpc_preset.rs`, `tool_preset.rs`, `program_preset.rs`)
- [x] Add examples for `ServerlessError`, `OpenApiBuilder`, and mount points
- [x] Create missing `examples/param_service.rs` (stable Rust version demonstrating inference)
- [x] Feature-gate `server-less-openapi` (optional dep, pulled in by http/ws/jsonrpc/graphql/openapi features)
- [x] Document relationship between `MethodInfo`/`ParamInfo` in core vs parse crates

---

## Audit Findings (2026-03-07)

Six-agent audit of the codebase. Items are new discoveries — not duplicates of existing queue items.

### CRITICAL — pre-publish blockers

- [x] **Generic impl blocks broken** ✅ Fixed: split_for_impl() pattern propagated to all 18 macro expanders; __CliI/__CliArg rename avoids collision with user generics; regression test added. (`server-less-parse/src/lib.rs` `get_impl_name`): Discards type parameters — `impl<T> MyService<T>` generates `impl MyService { }`, breaking all generic services with confusing compiler errors.

- [x] **Substring type inference produces wrong schemas** ✅ Fixed: AST-based type inspection using syn::Type pattern matching on the outermost type name. Vec<String>→array, HashMap<K,V>→object, Option<T> recurses into T.

### HIGH — embarrassing on crates.io

- [x] **`__trellis_` naming in generated code** ✅ Renamed to `__server_less_` in http.rs, ws.rs, jsonrpc.rs.

- [x] **`Box::leak` on every call in tool/method name fns** ✅ Changed return type to Vec<String>, push owned strings directly. No more Box::leak.

- [x] **`#[server(skip)]` ignored by GraphQL** ✅ partition_methods with has_server_skip predicate now called in expand_graphql. (`graphql.rs`): Methods marked `#[server(skip)]` still appear in GraphQL schema. GraphQL doesn't call `partition_methods` at all.

- [x] **HTTP `partition_methods(|_| false)` skip bug** ✅ Replaced hardcoded never-skip predicate with has_server_skip. (`http.rs`): Passes hardcoded never-skip predicate to `partition_methods`, then manually checks `has_server_skip` afterwards — means skipped `&T`-returning methods still get classified as mount points.

- [x] **Context injection missing from MCP and GraphQL** ✅ MCP and GraphQL now use partition_context_params and inject Context::default() instead of exposing ctx as input. (`mcp.rs`, `graphql.rs`): `server_less::Context` parameters treated as regular tool inputs/arguments instead of being injected. Breaks "annotate once, project anywhere" for any method using Context.

- [x] **`#[server(hidden)]` only respected by CLI** ✅ visible_leaf filter added to MCP, JSON-RPC, WS, HTTP, GraphQL; hidden methods excluded from all discovery outputs but still dispatchable.: Methods hidden from CLI help still appear in MCP tool lists, JSON-RPC method listings, OpenAPI specs, etc.

- [x] **`ServerlessError` → HTTP status uses string matching, not `IntoErrorCode`** ✅ Autoref specialization pattern; HttpStatusHelper calls http_status() on concrete error, fallback to 500.: HTTP handler infers status codes from error message text ("not found" → 404) rather than the `IntoErrorCode` trait. `#[error(code = 409)]` may silently not work.

- [x] **lib.rs doc examples use `#[ignore]` and wrong version** ✅ Changed to no_run, updated version to "0.2".: Main crate docs show `use server_less::prelude::*` with `#[ignore]` examples that don't compile; version shows `"0.1"` instead of current version. Bad on docs.rs.

- [x] **`JsonRpcMount` has no sync dispatch method** ✅ Added jsonrpc_mount_dispatch() sync variant to trait and generated impl; async-only methods return clear error from sync path.: Inconsistent with MCP, WS, and CLI which all have both sync and async variants.

- [x] **`.unwrap()` on `reference_inner` in mount code** ✅ Remaining two sites in `jsonrpc.rs` (`generate_static_mount_dispatch_sync`, `generate_slug_mount_dispatch_sync`) converted to `ok_or_else(syn::Error)` + `syn::Result` return types.

- [x] **Nested tokio runtime panic in `cli_run()`** ✅ Handle::try_current() guard added; returns proper Err if called inside tokio context.: If called from within a `#[tokio::main]` or `#[tokio::test]`, tokio panics with "Cannot start a runtime from within a runtime". Consider `Handle::try_current()` to return a proper `Err`.

### MEDIUM

- [x] **`#[param]` has zero integration tests** ✅ Added 8 tests covering name, query, path, body, default, header, help; fixed strip_http_attrs bug that was blocking compilation. Gap: help_text not wired to OpenAPI description.: `http_tests.rs` comment says it can't be tested on stable. Verify MSRV / edition 2024 claim and add tests.
- [x] **`#[param(help)]` not wired to OpenAPI description** ✅ openapi_gen.rs now reads ParamInfo::help_text; test assertion enabled.: `ParamInfo::help_text` is parsed but `openapi_gen.rs` hardcodes `description: None` for all parameters.

- [x] **`Path<T>`, `Query<T>`, `Json<T>` in `extract.rs` are dead code** ✅ Removed (54 lines); confirmed unused via grep across all crates.: Defined with Deref impls but never referenced in generated code or tests.

- [x] **`ErrorCode` missing `jsonrpc_code() -> i32`** ✅ Added to trait with sensible defaults; ServerlessError derive supports #[error(jsonrpc_code = -32602)]; jsonrpc.rs uses it in error responses.: Has `http_status()`, `grpc_code()`, `exit_code()` but no JSON-RPC error code mapping.

- [x] **HTTP mount OpenAPI composition incomplete** ✅ `http_openapi_paths()` now recursively collects mounted children via `<Child as HttpMount>::http_mount_openapi_paths()` and prefixes paths. `openapi_spec()` rebuilt from `http_openapi_paths()` so mounts appear in the spec. `HttpMountPathInfo` removed (replaced by `OpenApiPath`). `operationId`/`requestBody` serde renames added to `OpenApiOperation`.

- [x] **`panic!` in generated schema file writers** ✅ False positive — write_* methods already return std::io::Result<()>; panic! only in assert_schema_matches() which is intentional test-assertion behavior.: `grpc.rs`, `smithy.rs`, `thrift.rs`, `capnp.rs` use `panic!` in generated `write_*` methods for I/O errors. Should propagate errors.

- [x] **Stacking `#[cli]` + `#[http]` on same impl block doesn't compose** ✅ Priority-based impl emission: each macro checks `is_protocol_impl_emitter(current)` — the highest-priority protocol present (order: cli > http > mcp > jsonrpc > ws > graphql) emits the impl block; others skip it. `strip_protocol_impl_attrs` removed (keeping sibling attrs in output is key for the pipeline to process them). 5 e2e tests added covering 3-protocol and 2-protocol stacks.

- [x] **Iterator types silently fail in RPC dispatch** ✅ Added is_iterator branch in generate_json_response; integration tests verify array output. (`server-less-rpc`): `impl Iterator<Item = T>` return type has no handling in `generate_json_response` — falls through to `serde_json::to_value(iterator)` which fails at runtime. CLI handles iterators correctly.

- [x] **`if true { }` / `if false { }` in WS generated code** ✅ Moved uses_injected_params branch into Rust code to avoid dead warnings. (`ws.rs:795`): Produces dead_code/unreachable warnings in user's build output.

- [x] **Module doc in `cli.rs` missing async methods** ✅ Listed cli_run_async, cli_run_with_async, cli_dispatch_async in generated methods section.: `cli_run_async`, `cli_run_with_async`, `cli_dispatch_async` not listed in module-level doc comment.

- [x] **`no_sync`/`no_async` trait semantics undocumented** ✅ Expanded CliArgs field doc comments explaining what is and is not suppressed and why.: These suppress convenience methods only, not `cli_dispatch`/`cli_dispatch_async` on the trait. Surprising to users who expect full suppression.

- [x] **Only 4 compile-fail test fixtures** ✅ Added 5 fixtures: invalid_cli_attribute, invalid_param_attribute, serverless_error_on_struct, cli_on_non_impl, graphql_input_non_named_fields.: Missing coverage for invalid attribute syntax, conflicting attributes, multiple `#[cli(default)]` methods, `ServerlessError` on struct, etc.
- [x] **Multiple `#[cli(default)]` silently ignored** ✅ Now emits syn::Error::new_spanned pointing at second default, naming the first. Compile-fail fixture added.: When two methods are marked default, the macro takes the first silently. Should emit a compile error.

### MEDIUM — async CLI (from targeted audit)

- [x] **Async return types untested** ✅ Added tests for Result<T,E> ok path, Option<T> some/none (via --json), and () unit return.

- [x] **Async + output flags untested** ✅ Added 4 tokio tests for --json, --jq, --output-schema, --params-json via cli_run_with_async.: `--json`, `--jq`, `--params-json`, `--input-schema`/`--output-schema` untested through async dispatch path.

- [x] **Async slug mount dispatch untested** ✅ Added SlugParent/SlugChild test exercising generate_slug_mount_arm_async at runtime.

- [x] **`no_sync`/`no_async` compile-fail tests missing** ✅ Added trybuild fixtures: no_sync_missing_cli_run_with.rs, no_async_missing_cli_run_with_async.rs.

### LOW

- [x] **Pluralization produces "indexs", "statuss"** ✅ Added pluralize() helper with es/ies/s rules; infer_path now uses it. (`server-less-core` `infer_path`): Naive `+ "s"` heuristic.

- [x] **CLI default output documented as `Display`, actually pretty-printed JSON** ✅ cli_format_output is only called on JSON-flagged paths; default Display path is in macro. Updated doc comment to reflect this.: `cli_format_output` default contradicts the design doc.

- [x] **`camel_to_sentence` unwrap** ✅ Replaced .next().unwrap() with for-loop over ToLowercase iterator. (`error.rs:255`): Safe in practice but should use explicit char handling.

- [x] **GraphQL: no mount point / composition tests** ✅ 11 tests added; field-merging helpers generated on each #[graphql] service; child fields inlined into parent schema.

- [x] **`serve` macro never tested with GraphQL** ✅ 4 integration tests: POST /graphql responds, query executes, /health works, openapi_spec documents the endpoint.

---

## Ideas / Research

These need more design work before implementation:

- [ ] Distributed tracing (OpenTelemetry integration) — inject trace context via middleware layer
- [ ] GraphQL Federation support — far future; requires significant schema stitching work

> Removed as out-of-scope: Code-first→schema-first tooling, auto migration gen, contract testing (separate products), WASM/no_std (different target universe), performance benchmarks (pre-mature at this stage).

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
