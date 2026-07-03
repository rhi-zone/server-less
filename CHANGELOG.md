# Changelog

All notable changes to the server-less project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

## [0.7.0] - 2026-07-03

### Added

- **`#[cli(alias = "...")]` — hidden command aliases.** A leaf or mount method may now
  carry one or more hidden clap aliases: `#[cli(alias = "old-name")]` (repeatable) or
  `#[cli(aliases = ["a", "b"])]`. The command is invocable under each alias, but the
  alias is **not** shown in `--help` (clap's `.alias(...)` is hidden by default, unlike
  `.visible_alias(...)`). Intended as migration scaffolding: when a verb is renamed or
  moved, its old command path can be kept as a hidden alias for one release so existing
  invocations keep working without advertising the deprecated spelling. Additive.

## [0.6.0] - 2026-06-29

### Added

- **`CliGlobals` trait — the CLI capability-wiring invariant for `global = [...]`.**
  Declaring `#[cli(global = [...])]` now requires implementing `CliGlobals`; the macro
  delivers each declared global flag's parsed value to `set_global_flag(&self, name, value)`
  before the matched method runs. The delivery call names the sink by trait, so omitting
  the impl is a **compile error** (`E0277`) instead of a silently-inert flag. There is
  deliberately **no blanket default impl** — a default no-op would itself be a silent sink,
  re-creating the footgun. Flag names are delivered kebab-cased (`dry_run` → `"dry-run"`).
  Resolution policy (TTY/config) lives in the one sink method, not duplicated per command.
  See `docs/design/cli-capability-wiring-invariant.md`.
- **`#[param(default = ...)]` honored by the CLI projection.** A value-bearing parameter
  carrying a default is now wired to clap's `.default_value(...)`, so omitting it no longer
  errors "missing required argument" — clap supplies the default. Previously the field was
  parsed and discarded. (Additive.)

### Changed

- **BREAKING: `#[cli(global = [...])]` now requires `impl CliGlobals`.** Any service
  declaring global flags must implement the new `CliGlobals` trait or it will fail to
  compile. This is the intended forcing function that converts silently-inert global flags
  into loud build errors. Migration: add
  `impl CliGlobals for YourService { fn set_global_flag(&self, name: &str, value: bool) { /* stash/resolve */ } }`.
- **BREAKING: `CliGlobals` is the *sole* way a declared global flag is received — the
  legacy receive-via-matching-param path is removed.** Previously a global's value could
  *also* reach a method body through a method parameter whose name matched the flag (an
  implicit, convention-referenced wiring). That path is gone: globals are delivered only to
  `set_global_flag`. A method parameter that shares a declared global's flag name is now a
  **compile error** (it would collide with the root `.global(true)` flag at clap-build time
  and would silently never be auto-filled), consistent with the existing collision guard for
  built-in global flags. Migration: drop the matching parameter and read the value from your
  `CliGlobals` impl (e.g. stash it in a `Cell`).
- **`#[param(name = "...")]` now honored by the CLI projection.** The wire-name override
  renames the clap arg id, the `--long` flag, and the extraction key (previously the
  kebab-cased Rust identifier was used and the override silently dropped). This can rename
  a flag for any consumer that carried a `#[param(name)]` it expected to be ignored.

### Fixed

- **CLI errors are now structured under `--json`/`--jsonl`/`--jq`.** When a `Result`-returning
  command returns `Err`, the generated CLI dispatch previously always printed the error as plain
  text on stderr regardless of the active output format. It now emits `{"error": "<message>"}`
  on stdout (still exiting non-zero) when a structured-output flag is active, so programmatic
  consumers get a parseable error object instead of plain text on the wrong stream. Plain-text
  stderr is unchanged for the default (human) format.

## [0.5.0] - 2026-06-19

### Added

- **`--manual` whole-tree reference surface for `#[cli]`.** One invocation emits the
  entire command tree as a single reference document — the tool's "manual" — keyed by
  command path, each node carrying its description, input schema, and output schema.
  Works at every node (`tool --manual`; `tool sub --manual` scopes to that subtree),
  and composes with the existing `--json` / `--jsonl` / `--jq` format flags. Text by
  default, structured under `--json`. New core primitives: `CliManualNode`,
  `cli_manual_to_json`, `cli_manual_to_text`, and the `CliSubcommand::cli_manual_nodes`
  trait method (default impl, so hand-written impls keep compiling).
- **Meta-surface toggles for `#[cli]`.** `manual` / `input_schema` / `output_schema`
  (default-on) disable the injected `--manual` / `--input-schema` / `--output-schema`
  flags globally (`#[cli(manual = false)]`); per-command `#[cli(manual = false)]`
  hides one leaf from the aggregated manual while keeping the command runnable.
- **Reserved-name collision guard for `#[cli]`.** A parameter whose flag name collides
  with a currently-injected global flag (`json`, `jsonl`, `jq`, `params-json`, and the
  enabled meta-surfaces) is now a compile error spanned to the parameter, instead of a
  clap panic at runtime. Disabling a meta-surface frees its name.
- **`#[derive(HealthCheck)]`** (feature `health`): generates a `health_router()`
  returning an axum `Router` with a `GET /health` route (`#[health(path, status)]`
  overrides). Complements the `/health` route `#[server]` already mounts.
- **Shell completions + man page for `#[cli]`** (feature `completions`):
  `cli_completions(shell, out)` and `cli_manpage(out)` via `clap_complete` /
  `clap_mangen`.
- **`docs/design/cli-attributes.md`**: Reference for all `#[cli]` method-level
  attributes: `default`, `hidden`, `skip`, `helper`, `name`, `display_with`.
- **`.await`-without-`async` diagnostic.** A method projected by any server-less macro
  that uses `.await` in its body without being declared `async` now produces a clear,
  projection-framed compile error pointing at the fix, instead of leaning on rustc's
  generic E0728. Sound by construction — a `syn` visitor that ignores macro token
  streams and nested `async` blocks/closures, so there are no false positives.

### Changed

- **Reverted overly-aggressive `#[cli(default)]` suppression.** `#[cli(default)]` again
  registers the method as both the default action AND a named subcommand. To hide the
  default from `--help` while keeping it accessible, combine with `#[cli(hidden)]`.
- **`ErrorCode` and `OpenApiError` are now `#[non_exhaustive]`** — future variants
  will not be breaking changes. Downstream `match`es need a wildcard arm.

### Fixed

- **`--jq` output filtering** repaired by bumping to `jaq-core 3.1.0` / `jaq-std 3.0.1`
  / `jaq-json 2.0.1` and restoring the `jaq_core::defs()/funs()` prefix in the
  three-crate chain (a pre-release beta had silently broken the identity filter `.`).
- **Inferred HTTP path params strip a leading underscore.** `delete_resource(_id)` and
  `get_resource(id)` no longer register conflicting `/{_id}` vs `/{id}` matchit
  patterns (router build panic); an explicit `wire_name` is still taken verbatim.

## [0.4.0] - 2026-03-09

### Added

#### Config Management

- **`#[derive(Config)]`**: Generates `Config::load(sources)` and `Config::field_meta()` for structs.
  Supports `ConfigSource::Defaults`, `Env { prefix }`, and `File(PathBuf)` (TOML).
  Field attributes: `#[param(env = "VAR", file_key = "a.b", default = ..., help = "...")]`.
- **`#[program(config = MyConfig)]`**: Links a config struct to a CLI program. Adds a `config`
  subcommand with `show`, `schema`, `validate`, and `set` sub-subcommands.
  Supports `config_cmd = "custom-name"` and `config_cmd = false` to opt out.
- **`#[server(config = MyConfig)]`**: Links a config struct to an HTTP server. Generates
  `config_subcommand()` and `config_run_subcommand()` methods on the server struct.
  Same `config_cmd` customization options as `#[program]`.
- **`#[app]` attribute**: Protocol-neutral metadata (`name`, `description`, `version`, `homepage`)
  injected via `#[__app_meta]` passthrough; consumed by `#[server]`, `#[cli]`, `#[http]`, `#[program]`
  as fallbacks for unset fields.
- **`server-less-core::config` module** (behind `config` feature): `Config` trait, `ConfigSource`,
  `ConfigError`, `ConfigFieldMeta`, and `load_toml_file` helper.
- **Re-exports**: `server_less::Config` (derive macro), `server_less::ConfigTrait`,
  `server_less::ConfigSource`, `server_less::ConfigError`, `server_less::ConfigFieldMeta`.

## [0.1.0] - 2025-01-25

### Added

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
  - `#[param(query/path/body/header)]` - parameter location override (fully implemented)
  - Extended ParamInfo with wire_name, location, and default_value fields
  - Updated HTTP parameter extraction to use custom names and respect location overrides
  - HTTP handlers group parameters by actual location (path/query/body/header)
  - Added header parameter extraction support
  - OpenAPI generation reflects renamed parameters, default values, and location overrides
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
- **329 tests passing**
- All clippy checks clean
- Full documentation coverage
- Comprehensive tutorials
