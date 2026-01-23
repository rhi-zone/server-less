# Context Design Decisions

This document records design decisions about Context injection for different protocols, including deferred decisions for future review.

Last updated: 2026-01-23

---

## ✅ Implemented Protocols

### HTTP (`#[http]`)
**Decision:** Full Context support from HTTP headers
**Rationale:** HTTP is inherently request/response with rich header metadata
**Implementation:** Extract all headers into Context, special handling for `x-request-id`

### CLI (`#[cli]`)
**Decision:** Full Context support from environment variables
**Rationale:** CLI tools run in shell environments with available env vars
**Implementation:** All env vars available via `ctx.env("VAR_NAME")`

### JSON-RPC (`#[jsonrpc]`)
**Decision:** Full Context support from HTTP headers (runs over HTTP POST)
**Rationale:** JSON-RPC inherits HTTP transport, headers available
**Implementation:** Same as HTTP - extract headers into Context

### WebSocket (`#[ws]`)
**Decision:** Full Context support from HTTP upgrade headers
**Rationale:** WebSocket starts as HTTP upgrade, headers available
**Implementation:** Extract once during upgrade, persist for connection lifetime

---

## ⏸️ Deferred Protocols

### MCP (`#[mcp]`) - Model Context Protocol

**Status:** Deferred - skip Context for now

**Problem:** MCP tools are pure functions called by LLMs with JSON arguments. No inherent "request context" like HTTP.

**Options Considered:**

1. **Skip Context entirely** ✅ **CHOSEN (for now)**
   - **Pros:**
     - MCP tools meant to be pure functions
     - LLM passes everything needed in arguments
     - Simple, clear semantics
   - **Cons:**
     - Inconsistent with HTTP-based protocols
   - **When to reconsider:** If exposing MCP over HTTP transport

2. **Conversation Context**
   - **What:** Track conversation ID, turn number, user ID across tool calls
   - **Pros:** Enables stateful multi-turn conversations
   - **Cons:** High complexity, need conversation state management
   - **Use case:** Tools that remember previous context

3. **LLM Metadata Context**
   - **What:** Model name, temperature, token usage, provider info
   - **Pros:** Tools can adapt behavior based on LLM
   - **Cons:** Adds coupling to LLM details
   - **Use case:** Tools that need to know which LLM is calling

**Workarounds for users:**
- Pass context as explicit tool arguments (recommended)
- Store conversation state in struct fields
- Add Context later if exposing MCP over HTTP

**TODO:** Review this decision if/when:
- Users request conversation tracking
- MCP gets exposed over HTTP transport
- Standardized MCP context emerges

---

### GraphQL (`#[graphql]`)

**Status:** Deferred - bridge contexts when needed

**Problem:** async-graphql already has powerful `ResolverContext` system. Adding `server_less::Context` creates potential confusion.

**Options Considered:**

1. **Skip server_less::Context**
   - **Pros:** Zero confusion, async-graphql context is powerful
   - **Cons:** Inconsistent with other HTTP-based protocols
   - **Users can:** Insert server_less::Context into async-graphql context manually

2. **Bridge contexts** ✅ **RECOMMENDED (not yet implemented)**
   - **What:** Auto-extract HTTP headers into server_less::Context, insert into async-graphql context
   - **Implementation:**
     ```rust
     #[graphql]
     impl UserService {
         async fn get_user(&self, ctx: &ResolverContext, id: i32) -> User {
             // Access server_less::Context via async-graphql context
             let sl_ctx: &server_less::Context = ctx.data_unchecked();
             let request_id = sl_ctx.request_id()?;
             // Also have full async-graphql context available
         }
     }
     ```
   - **Pros:**
     - Consistent header extraction across all HTTP protocols
     - Both contexts available
     - No breaking changes to async-graphql patterns
   - **Cons:**
     - Indirection to access server_less::Context
     - Need to understand both context systems

3. **Dual context parameters**
   - **What:** Support both context types in method signatures
     ```rust
     async fn get_user(
         &self,
         ctx: server_less::Context,      // Headers
         gql_ctx: &ResolverContext,       // GraphQL query info
         id: i32
     ) -> User
     ```
   - **Pros:** Explicit, both available directly
   - **Cons:** Verbose, unusual API

4. **Replace async-graphql context** ❌ **NOT RECOMMENDED**
   - **Why not:** Loses GraphQL-specific features (parent data, query depth, lookahead, etc.)

**TODO:** Review and potentially implement Option 2 (bridge) if:
- Users need consistent header extraction
- Cross-protocol observability needed
- Request ID tracking across protocols

---

## 🤔 Future Considerations

### Extensible Context

**Idea:** Allow users to extend Context with custom data beyond headers/env vars.

**Potential approaches:**
1. **Generic Context<T>**
   ```rust
   struct MyContext {
       user_id: String,
       tenant_id: String,
   }
   fn handler(&self, ctx: Context<MyContext>) { }
   ```

2. **Context::with_data()**
   ```rust
   let ctx = Context::new()
       .with_data("user_id", user_id)
       .with_data("tenant", tenant);
   ```

3. **Context::extend()**
   ```rust
   ctx.extend(|ctx| {
       ctx.set_user_id(user.id);
       ctx.set_tenant(tenant);
   });
   ```

**Challenges:**
- Type safety vs flexibility tradeoff
- When/how does extension happen? (middleware? decorator?)
- How to access custom data?
- Need to maintain zero-cost abstractions

**Use cases:**
- Authentication data (user ID, roles, permissions)
- Multi-tenancy (tenant ID, org ID)
- Distributed tracing (trace ID, span ID)
- Feature flags
- Custom business context

**TODO:** Explore extensible context designs:
- Research how other frameworks handle this (Tower, actix-web, etc.)
- Prototype different approaches
- Consider integration with middleware system
- Document trade-offs

---

## Summary

**Current Status:** 4/6 protocols fully support Context
- ✅ HTTP - headers
- ✅ CLI - environment variables
- ✅ JSON-RPC - HTTP headers
- ✅ WebSocket - upgrade headers
- ⏸️ MCP - deferred (skip for now)
- ⏸️ GraphQL - deferred (bridge when needed)

**Consistency:** All HTTP-based protocols extract Context from request headers

**Philosophy:** Context should be invisible for users who don't want it, convenient for those who do

**Open Questions:**
1. Should MCP support conversation context?
2. Should GraphQL bridge contexts automatically?
3. How should Context be extensible?
4. Should Context support middleware injection?

**Next Review:** When user feedback suggests different direction or new use cases emerge
