# Context Injection - Implementation Status

## ✅ Implemented

### 1. HTTP (`#[http]`)
**Status:** Complete
**Context Source:** HTTP headers
**Tests:** 6 tests in `context_tests.rs`, 21 tests in `http_tests.rs`

```rust
#[http]
impl UserService {
    async fn create_user(&self, ctx: server_less::Context, name: String) -> User {
        let request_id = ctx.request_id()?;  // From x-request-id header
        let user_id = ctx.user_id()?;        // Can be set by auth middleware
        // All headers available via ctx.header("name")
    }
}
```

**Features:**
- Automatic injection from HTTP headers
- Two-pass collision detection
- Excluded from OpenAPI specs
- Works with path/query/body parameters

### 2. CLI (`#[cli]`)
**Status:** Complete
**Context Source:** Environment variables
**Tests:** 6 tests in `cli_tests.rs`

### 3. JSON-RPC (`#[jsonrpc]`)
**Status:** Complete
**Context Source:** HTTP headers (JSON-RPC runs over HTTP POST)
**Tests:** 2 tests in `context_tests.rs`, all existing jsonrpc_tests pass

```rust
#[jsonrpc]
impl Calculator {
    fn add(&self, ctx: server_less::Context, a: i32, b: i32) -> i32 {
        let request_id = ctx.request_id()?;  // From x-request-id header
        a + b
    }
}
```

**Features:**
- Automatic injection from HTTP request headers
- Two-pass collision detection
- Backward compatible (Context only required if methods use it)
- Works with both sync and async methods

### 4. WebSocket (`#[ws]`)
**Status:** Complete
**Context Source:** HTTP upgrade headers (extracted once per connection)
**Tests:** 2 tests in `context_tests.rs`, all existing ws_tests pass

```rust
#[ws(path = "/chat")]
impl ChatService {
    fn echo(&self, ctx: server_less::Context, message: String) -> String {
        let request_id = ctx.request_id()?;  // From WebSocket upgrade headers
        format!("[{}] {}", request_id, message)
    }
}
```

**Features:**
- Automatic injection from WebSocket HTTP upgrade headers
- Context persists for entire WebSocket connection
- Two-pass collision detection
- Backward compatible (Context only required if methods use it)
- Works with both sync and async message handlers

```rust
#[cli]
impl MyApp {
    fn deploy(&self, ctx: server_less::Context, env: String) {
        let api_key = ctx.env("API_KEY");  // From environment
        let user = ctx.env("USER");
        // All env vars available via ctx.env("VAR_NAME")
    }
}
```

**Features:**
- Automatic injection from environment variables
- Two-pass collision detection
- Excluded from CLI help/args
- Works with required/optional args

### 5. Shared Infrastructure (`context.rs`)
**Status:** Complete
**Tests:** 3 unit tests

**Helpers:**
- `has_qualified_context(methods)` - First-pass detection
- `partition_context_params(params, has_qualified)` - Separate Context from regular params
- `should_inject_context(ty, has_qualified)` - Type checking
- `generate_http_context_extraction()` - HTTP header extraction
- `generate_cli_context_extraction()` - Environment variable extraction

## ⏸️ Deferred (Design Decisions Documented)

**See [CONTEXT_DECISIONS.md](./CONTEXT_DECISIONS.md) for full analysis and rationale.**

### MCP (`#[mcp]`)
**Status:** Deferred - skip Context for now
**Decision:** MCP tools are pure functions; LLM passes data via arguments

**Rationale:**
- MCP tools meant to be stateless
- No inherent "request context" like HTTP
- Users can pass context as explicit tool arguments

**Future consideration:** May add conversation context (conversation ID, turn number) if users request it, or if MCP gets exposed over HTTP transport.

### GraphQL (`#[graphql]`)
**Status:** Deferred - bridge contexts when needed
**Decision:** async-graphql already has powerful ResolverContext

**Recommended approach (not yet implemented):**
- Auto-extract HTTP headers into server_less::Context
- Insert into async-graphql context
- Users access via `ctx.data_unchecked::<server_less::Context>()`

**Rationale:**
- Consistent header extraction across all HTTP protocols
- Both contexts available (async-graphql + server_less)
- No breaking changes to async-graphql patterns

**Future consideration:** Implement bridging if users need cross-protocol header extraction or request ID tracking.

## Summary

**Implemented:** 4/6 protocol macros (HTTP, CLI, JSON-RPC, WebSocket)
**Shared Infrastructure:** Complete and tested
**Coverage:** All request/response protocols + RPC protocols ✅
**Deferred:** MCP (skip for now), GraphQL (bridge when needed)

**See [CONTEXT_DECISIONS.md](./CONTEXT_DECISIONS.md) for full design decisions and future considerations.**

## Integration Path for Remaining Protocols

Remaining protocols have documented design decisions but are not blocking:

**MCP:**
- **Decision:** Skip Context for now - tools are pure functions
- **Rationale:** LLM passes everything in arguments; no HTTP layer
- **Path forward:** Can add conversation context if users request it

**GraphQL:**
- **Decision:** Bridge contexts when needed (not yet implemented)
- **Rationale:** async-graphql has powerful context; avoid confusion
- **Path forward:** Auto-extract headers into server_less::Context, insert into async-graphql context

## Testing

All implemented protocols have:
- ✅ Basic injection tests
- ✅ Collision detection tests
- ✅ Spec generation tests (Context excluded)
- ✅ Integration with existing tests

**Total test count:** 219 tests passing (10 Context-specific)
