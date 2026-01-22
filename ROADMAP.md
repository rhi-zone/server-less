# Trellis Roadmap

This document outlines the development roadmap for Trellis. Last updated: 2026-01-23

## Current Status - Foundation ✅

**18 macros implemented, 187 tests passing, 0 failures**

### What's Working
- ✅ Core runtime protocols (HTTP, CLI, MCP, WebSocket, JSON-RPC, GraphQL)
- ✅ Schema generators (gRPC, Cap'n Proto, Thrift, Smithy, Connect)
- ✅ Specification generators (OpenRPC, AsyncAPI, JSON Schema, Markdown)
- ✅ Error derive macro with HTTP status mapping
- ✅ Serve macro for multi-protocol composition
- ✅ Comprehensive test coverage (171 tests)
- ✅ Design documentation
- ✅ Working examples for all major protocols
- ✅ Feature gating (all macros opt-in)
- ✅ Async/sync method support
- ✅ SSE streaming (HTTP)
- ✅ Method naming conventions
- ✅ Return type handling (Result/Option/Vec/T/())

### Known Limitations
- ⚠️ GraphQL: Array/object type mapping returns "String" instead of proper GraphQL types
- ⚠️ Schema validation: Uses panic!() instead of Result types
- ⚠️ Streaming: SSE works but bidirectional WebSocket patterns not fully explored

---

## Next Phase - Polish & Refinement

**Goal:** Fix known issues, improve documentation, prepare for wider use

### Critical Fixes
- [ ] **GraphQL Type Mapping** (#1)
  - Fix array types: `Vec<T>` → `GraphQL::List(T)` not `"String"`
  - Fix object types: Custom structs → proper GraphQL objects
  - Add scalar type registry for custom types

- [ ] **Error Handling** (#2)
  - Replace `panic!()` with `Result` in schema validation
  - Add proper error types for proto/capnp/thrift/smithy validation
  - Better error messages with spans

- [ ] **Streaming Enhancements** (#3)
  - Test bidirectional WebSocket patterns
  - Add WebSocket server-push examples
  - Document Rust 2024 `+ use<>` requirement clearly
  - Add gRPC streaming support exploration

### Documentation
- [ ] **Inline Documentation** (#4)
  - Add module-level docs for all macros
  - Add examples to all macro attributes
  - Document all configuration options
  - Generate docs.rs documentation

- [ ] **Tutorial Series** (#5)
  - "Building a REST API with Trellis"
  - "Multi-protocol Services"
  - "Schema-First Development"
  - "Error Handling Best Practices"

### Features
- [ ] **Attribute Customization** (#6)
  - `#[route(method="POST", path="/custom")]` per-method overrides
  - `#[param(query, name="q")]` parameter customization
  - `#[response(status=201)]` response customization

- [ ] **OpenAPI Separation** (#7)
  - Extract OpenAPI generation as standalone `#[openapi]` macro
  - Keep HTTP macro focused on routing
  - Allow OpenAPI without HTTP runtime dependency

- [ ] **API Reference** (#8)
  - Complete inline documentation
  - Add search keywords
  - Cross-link examples

---

## Medium Term - Developer Experience

**Goal:** Make Trellis delightful to use

### Error Messages
- [ ] **Better Diagnostics** (#10)
  - Use `proc_macro2::Span` for precise error locations
  - Add "help" hints for common mistakes
  - Suggest fixes for type mismatches
  - Show generated code in error messages

- [ ] **Validation at Compile Time** (#11)
  - Validate HTTP paths are well-formed
  - Check for duplicate routes
  - Warn about unused parameters
  - Validate JSON-RPC method names

### IDE Integration
- [ ] **Code Actions** (#12)
  - "Add macro" code action
  - "Generate implementation" stub
  - "Add parameter documentation"
  - "Generate OpenAPI spec"

- [ ] **rust-analyzer Support** (#13)
  - Macro expansion hints
  - Go-to-definition for generated code
  - Hover documentation

### Development Tools
- [ ] **Hot Reloading** (#14)
  - Watch mode for development
  - Reload routes without restart
  - Preserve state across reloads

- [ ] **Debug Mode** (#15)
  - `#[http(debug = true)]` verbose logging
  - Print generated code to stderr
  - Trace parameter extraction
  - Show dispatch flow

### Performance
- [ ] **Benchmarks** (#16)
  - Compare vs hand-written Axum
  - Compare vs hand-written Clap
  - Measure compile-time overhead
  - Document performance characteristics

- [ ] **Optimization** (#17)
  - Reduce generated code size
  - Minimize monomorphization
  - Optimize dispatch patterns

---

## Long Term - Advanced Features

**Goal:** Enterprise-ready features

### Production Features
- [ ] **API Versioning** (#18)
  - `#[http(version = "v1")]` URL versioning
  - Header-based versioning
  - Content negotiation
  - Deprecation warnings

- [ ] **Rate Limiting** (#19)
  - `#[derive(RateLimit)]` macro
  - Per-method limits
  - Per-user/IP limits
  - Token bucket algorithm

- [ ] **Authentication/Authorization** (#20)
  - `#[auth(required)]` attribute
  - Bearer token support
  - JWT validation
  - Role-based access control

### Middleware System
- [ ] **Middleware Hooks** (#21)
  - Before/after request hooks
  - Error transformation
  - Logging/tracing
  - Request/response modification

- [ ] **Middleware Composition** (#22)
  - Stack middlewares on impl blocks
  - Tower layer integration
  - Async middleware support

### Cross-Language Support
- [ ] **Schema Sharing** (#23)
  - Export schemas for all protocols at once
  - Validate consistency across protocols
  - Generate unified documentation

- [ ] **Client Generation** (#24)
  - TypeScript client from HTTP schema
  - Python client from HTTP schema
  - Rust client from schema
  - Client SDK templates

---

## Eventually - Stability & Ecosystem

**Goal:** Production-ready, stable API

### API Stability
- [ ] **Semver Guarantees** (#25)
  - Lock public API
  - Document stability guarantees
  - Define deprecation policy
  - Version migration guide

### Production Hardening
- [ ] **Battle Testing** (#26)
  - Deploy in production environments
  - Collect feedback from users
  - Fix discovered issues
  - Performance tuning

### Long-term Support
- [ ] **LTS Commitment** (#27)
  - Security updates
  - Bug fixes
  - Documentation maintenance
  - Community support

### Ecosystem
- [ ] **Extension Ecosystem** (#28)
  - Third-party macro registry
  - Plugin system for custom protocols
  - Middleware marketplace
  - Community contributions

---

## Future Explorations

### Research Items
- [ ] **Code-first → Schema-first** transition support
- [ ] **Automatic migration generation** between API versions
- [ ] **Contract testing** framework
- [ ] **Load testing** integration
- [ ] **Distributed tracing** built-in
- [ ] **Observability** macros
- [ ] **GraphQL Federation** support
- [ ] **gRPC bidirectional streaming**
- [ ] **WebAssembly** target support
- [ ] **No-std** support for embedded

### Community Requests
Track community feature requests and prioritize based on:
- Number of requests
- Implementation difficulty
- Alignment with philosophy
- Maintenance burden

---

## Contributing to Roadmap

Have ideas? Open an issue with:
- **Feature description**: What problem does it solve?
- **Use case**: Real-world scenario where you'd use it
- **Design sketch**: How might it work?
- **Alternatives**: What are other ways to solve this?

We prioritize features that:
1. Align with impl-first philosophy
2. Have clear, real-world use cases
3. Don't add complexity to simple scenarios
4. Can be feature-gated if niche

---

## Maintenance

### Regular Tasks
- Weekly: Review issues and PRs
- Monthly: Update dependencies
- Quarterly: Performance benchmarks
- Annually: Security audit

### Release Philosophy
- **Patches**: Bug fixes as needed
- **Features**: New capabilities when ready
- **Breaking changes**: Avoid when possible, batch when necessary
- No fixed schedule - ship when it's ready

---

## Notes

- This roadmap is a living document and will evolve based on feedback
- Features may be reprioritized based on user needs
- We reserve the right to say "no" to features that don't fit our philosophy
- Timelines are intentionally vague - quality over deadlines

Last updated: 2026-01-23 by Claude Sonnet 4.5
