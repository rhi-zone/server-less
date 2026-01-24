# Server-less Roadmap

This document outlines the development roadmap for server-less.

---

## Current Status - Foundation ✅

**18 macros implemented, 195+ tests passing**

### What's Working
- ✅ Core runtime protocols (HTTP, CLI, MCP, WebSocket, JSON-RPC, GraphQL)
- ✅ Schema generators (gRPC, Cap'n Proto, Thrift, Smithy, Connect)
- ✅ Specification generators (OpenRPC, AsyncAPI, JSON Schema, Markdown)
- ✅ OpenAPI standalone macro and composition (OpenApiBuilder)
- ✅ Per-protocol OpenAPI paths methods (Phase 2)
- ✅ Error derive macro with HTTP status mapping
- ✅ Serve macro for multi-protocol composition
- ✅ Context injection (HTTP, CLI, JSON-RPC, WebSocket)
- ✅ WebSocket bidirectional patterns (WsSender)
- ✅ SSE streaming (HTTP)
- ✅ Route/response/param attributes
- ✅ Compile-time path validation and duplicate route detection
- ✅ Feature gating (all macros opt-in)
- ✅ Async/sync method support
- ✅ Comprehensive test coverage

### Known Limitations
- ⚠️ Schema generators use panic!() instead of Result types
- ⚠️ GraphQL: Nested types and subscriptions not yet supported
- ⚠️ OpenAPI composition phases 3-4 not yet implemented

---

## Next Phase - OpenAPI & Polish

**Goal:** Complete OpenAPI composition, improve schema generators

### OpenAPI Composition
- [x] **Phase 2: Per-protocol methods** - `http_openapi_paths()`, etc. ✅
- [ ] **Phase 3: Serve integration** - Auto-generate combined spec
- [ ] **Phase 4: Protocol-aware #[openapi]** - Detect sibling protocols

### OpenAPI Improvements
- [ ] Richer parameter schemas
- [ ] Response schemas with examples
- [ ] `#[openapi(hidden)]` attribute

### Error Handling
- [ ] Replace panic!() with Result in schema validation
- [ ] Better error messages with spans

### GraphQL
- [ ] Nested type resolution
- [ ] Custom scalar support (DateTime, UUID)
- [ ] Subscription support

---

## Medium Term - Developer Experience

**Goal:** Make server-less delightful to use

### Streaming
- [ ] MCP streaming responses
- [ ] gRPC streaming exploration

### Better Diagnostics
- [ ] Help hints for common mistakes
- [ ] Suggest fixes for type mismatches
- [ ] Show generated code snippets in errors

### Development Tools
- [ ] Debug mode (`#[http(debug = true)]`)
- [ ] Hot reloading exploration

### Performance
- [ ] Benchmarks vs hand-written code
- [ ] Compile-time overhead measurement

---

## Long Term - Advanced Features

**Goal:** Enterprise-ready features

### Middleware System
- [ ] Before/after request hooks
- [ ] Tower layer integration
- [ ] Async middleware support
- [ ] `#[middleware(auth, logging)]` attribute

### API Versioning
- [ ] URL versioning (`#[http(version = "v1")]`)
- [ ] Header-based versioning
- [ ] Deprecation warnings

### Authentication/Authorization
- [ ] `#[auth(required)]` attribute
- [ ] Bearer token support
- [ ] Role-based access control

### Client Generation
- [ ] TypeScript client from OpenAPI
- [ ] Python client from OpenAPI
- [ ] Rust client from schema

---

## Eventually - Stability & Ecosystem

**Goal:** Production-ready, stable API

### Schema Sharing
- [ ] Common schema representation for MCP/OpenAPI/GraphQL
- [ ] Cross-protocol consistency validation
- [ ] Unified documentation generation

### gRPC Runtime
- [ ] tonic integration
- [ ] All streaming patterns
- [ ] Error code mapping

### "Server" Blessed Preset
```rust
#[derive(Server)]  // Expands to ServerCore + OpenApi + Metrics + HealthCheck + Serve
struct MyServer;
```

### API Stability
- [ ] Lock public API
- [ ] Semver guarantees
- [ ] Migration guides

---

## Future Explorations

- Code-first → Schema-first transition support
- Automatic migration generation between API versions
- Contract testing framework
- Distributed tracing built-in
- GraphQL Federation support
- WebAssembly target support

---

## Contributing

Have ideas? Open an issue with:
- **Feature description**: What problem does it solve?
- **Use case**: Real-world scenario
- **Design sketch**: How might it work?

We prioritize features that:
1. Align with impl-first philosophy
2. Have clear, real-world use cases
3. Don't add complexity to simple scenarios
4. Can be feature-gated if niche
