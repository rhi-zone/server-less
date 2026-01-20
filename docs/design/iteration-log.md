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

## Next Steps

Options for next iteration:
1. **E2E tests** - validate generated code against reference implementations
2. **Async support** - properly handle async methods in MCP/WS
3. **GraphQL** - would test nested types, different query model
4. **Solidify** - documentation, error messages, edge cases

---

## Future Iterations

(To be filled as we go)

- Iteration 2: Add gRPC or WebSocket
- Iteration 3: Composable OpenAPI
- Iteration N: Error derive macro
- Iteration N: "Serve" coordination pattern
