# Changelog

All notable changes to the Trellis project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added - January 2025

#### GraphQL Improvements
- **GraphQL array type mapping**: Vec<T> now properly maps to GraphQL List(T) with inner type extraction
- **GraphQL object type mapping**: Custom structs now convert to proper GraphQL objects instead of JSON strings
- Added 3 comprehensive tests for custom struct handling (single objects, lists, mutations)

#### Validation & Error Handling
- **Schema validation error types**: Replaced panic!() with Result<_, SchemaValidationError> across all schema generators (gRPC, Cap'n Proto, Thrift, Smithy)
- **Compile-time validation**: HTTP path validation with detailed error messages
  - Validates paths start with '/'
  - Checks for invalid characters with context-aware hints
  - Validates brace matching for path parameters
  - Ensures path parameters have names
- **Duplicate route detection**: Catches conflicting routes at compile time with helpful resolution suggestions
- **Helpful error messages**: All macros now provide actionable hints and examples
  - HTTP macro: Enhanced errors for unknown arguments, duplicate routes, invalid paths
  - Error derive: Better hints for error codes, messages, enum requirement
  - CLI macro: Added examples for unknown arguments
  - GraphQL macro: Added examples for unknown arguments
  - Parse crate: Better explanation for unsupported parameter patterns
  - Serve macro: Enhanced unknown protocol errors with examples

#### Route Customization
- **Route override implementation**: Full support for `#[route(...)]` attribute
  - `#[route(method = "POST")]` - override HTTP method
  - `#[route(path = "/custom")]` - override path
  - `#[route(skip)]` - exclude from routing
  - `#[route(hidden)]` - exclude from OpenAPI

#### Response Customization
- **Response override implementation**: Full support for `#[response(...)]` attribute
  - `#[response(status = 201)]` - custom HTTP status code
  - `#[response(content_type = "application/octet-stream")]` - custom content type
  - `#[response(header = "X-Custom", value = "foo")]` - custom headers
  - Multiple `#[response(...)]` attributes can be combined
  - OpenAPI spec generation reflects custom status codes, content types, and headers
  - Added 8 comprehensive tests covering all response customization scenarios

#### Parameter Customization
- **Parameter override implementation**: Full support for `#[param(...)]` attribute
  - `#[param(name = "q")]` - custom wire name for parameters
  - `#[param(default = 10)]` - default values for optional parameters
  - `#[param(query/path/body/header)]` - parameter location override (parsed but not yet fully implemented)
  - Extended ParamInfo with wire_name, location, and default_value fields
  - Updated HTTP parameter extraction to use custom names
  - OpenAPI generation reflects renamed parameters and default values
  - Parameters with defaults marked as not required in OpenAPI
  - Note: Requires nightly Rust due to `#[register_tool(param)]` requirement

#### Documentation
- **Module-level documentation**: Added comprehensive docs to all 17 macro modules
- **Tutorial creation**: Created REST API and Multi-Protocol tutorials (1000+ lines total)
  - `docs/tutorials/rest-api.md` - Complete blog API tutorial with CRUD, error handling, OpenAPI
  - `docs/tutorials/multi-protocol.md` - Exposing services via HTTP, WebSocket, JSON-RPC, GraphQL, CLI, MCP
  - `docs/tutorials/README.md` - Tutorial index with quick start and learning path
- **Attribute examples**: Added examples to all macro attributes showing configuration options
- Updated lib.rs crate docs with all features
- Updated README.md with real examples
- Documented async support, SSE streaming, feature flags

### Added - Earlier

#### Core Features
- **Feature Gates**: Added `#[cfg(feature = "...")]` guards around macro re-exports
  - Features: `http`, `ws`, `jsonrpc`, `graphql`, `cli`, `mcp`, `grpc`, `capnp`, `thrift`, `connect`, `smithy`
  - Schema generators: `openrpc`, `asyncapi`, `jsonschema`
  - Doc generators: `markdown`
  - Type stubs: `typescript`, `python`
  - Default feature: `full` (enables all features)

#### Testing
- **E2E Testing Strategy**: Implemented in `tests/e2e_tests.rs`
  - Reference implementations in `Calculator` struct
  - Protocol wrappers (`McpCalculator`, `WsCalculator`, etc.)
  - Cross-protocol consistency tests ensuring all protocols produce identical results

#### Async Support
- **MCP and WebSocket async methods**:
  - `mcp_call` / `ws_handle_message`: sync callers, error on async methods
  - `mcp_call_async` / `ws_handle_message_async`: async callers, await async methods
  - WebSocket connections use async dispatch (real connections work with async)

#### Error Messages
- Improved all macro error messages with spans
- Unknown attributes now list valid options
- Associated functions without `&self` (constructors) are silently skipped
- Unsupported parameter patterns report errors instead of being silently skipped

### Current Status
- **187 tests passing**
- All clippy checks clean
- Full documentation coverage
- Comprehensive tutorials
