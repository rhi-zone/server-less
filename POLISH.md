# Polish State

Created: bf18ee2a0e903fc4054b38b354405fbcd7263bd9
Last run: 2026-04-26
Applied through: df19851
Round: 3
Project type: Rust proc-macro library (6 crates)

## Lenses
- adversarial _(round 1)_
- api-clarity _(round 1)_
- doc-coverage _(rounds 1, 3)_
- error-surface _(round 1)_
- naming-consistency _(round 1)_
- legacy-debt _(round 1)_
- completeness _(round 2)_
- cross-macro behavioral consistency _(round 2)_
- api-gaps _(round 3)_
- consistency _(round 3)_
- overfit _(round 3)_

## Scope
Full codebase (`crates/`, `docs/`, `README.md`, `CONTEXT_*.md`)

## Findings — Round 1

### HIGH severity

- [APPLIED] `crates/server-less-macros/src/http.rs:830,878,908` — Required body/query/header params silently fall back to `T::default()` when missing or malformed — no HTTP error returned, silent data corruption. Fix: return 400 Bad Request when a required param is absent. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/cli.rs:274,296,321` — Per-method `#[cli(...)]` attribute parse errors silently discarded (`let _ = attr.parse_nested_meta(...)`). Typos accepted with no effect. Fix: propagate `syn::Result` and collect errors. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/cli.rs:471` — `syn::Ident::new(name, ...)` from user-provided `defaults = "fn_name"` string panics on invalid identifiers. Fix: validate with `syn::parse_str::<syn::Ident>` and return `syn::Error`. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/lib.rs:119` — `PROTOCOL_PRIORITY.iter().position(...).unwrap_or(usize::MAX)` then `&PROTOCOL_PRIORITY[..current_pos]` panics OOB if a new protocol is added without updating the list. Fix: use `.expect("BUG: protocol not in PROTOCOL_PRIORITY")`. _(severity: high)_
- [APPLIED] `crates/server-less-rpc/src/lib.rs:15,343,370` — Raw identifier params (e.g. `r#type`) produce JSON key `"r#type"` — schema advertises `"type"`, dispatch silently fails. Fix: strip `r#` prefix when converting param names to strings. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/lib.rs:505` / `cli.rs:80` / `http.rs:114` — Docs list `cli_app()` (never generated; real method is `cli_command()`) and `http_routes()` (never generated). Fix: update all three locations. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/lib.rs:1475` — `#[param]` doc warns "requires nightly `#![feature(register_tool)]`" — false, stable Rust works fine. Fix: remove the nightly note. _(severity: high)_
- [APPLIED] `CONTEXT_INTEGRATION.md:85,121` — "not yet implemented" markers for WebSocket and CLI Context injection that are fully implemented. Fix: update or archive the file. _(severity: high)_
- [APPLIED] `crates/server-less-core/src/extract.rs:39` — Doc comment says "CLI: not yet implemented" for Context injection — it is implemented. Fix: update to "injected via `#[cli]`". _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/lib.rs:493,1727` — `#[cli]` and `#[program]` doc examples show `about = "..."` which was renamed to `description` in v0.4.0. Fix: update examples. _(severity: high)_
- [APPLIED] `crates/server-less-core/src/extract.rs:26` — `Context` doc example uses `ctx.user_id()?` but `user_id()` returns `Option<&str>`, not `Result` — example doesn't compile. Fix: remove `?` or add `.ok_or("...")?`. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/jsonrpc.rs:24` — Module doc says `jsonrpc_handle_async` is generated; both sync/async are named `jsonrpc_handle`. Fix: rename async form to `jsonrpc_handle_async` for consistency with `ws_handle_message_async`. _(severity: high)_

### MEDIUM severity

- [APPLIED] `crates/server-less-macros/src/http.rs:1094` — `validate_http_path` error spans the Rust method identifier, not the `#[route(path = "...")]` literal. Fix: pass the `LitStr` span. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/http.rs:397` — Unsupported HTTP verb in `#[route(method = "TRACE")]` silently falls back to name inference. Fix: reject unknown method strings with a clear error. _(severity: medium)_
- [APPLIED] `crates/server-less-parse/src/lib.rs:463` — `#[param(serde)]` without `#[param(nested)]` produces silently inconsistent state. Fix: emit an error or auto-set `nested=true`. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/mcp.rs:351` — `partition_context_params` errors discarded in tool-definition generation but propagated in dispatch — inconsistent. Fix: propagate errors in tool-definition path too. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/cli.rs:344` — `#[cli(display_with = "...")]` with invalid Rust path silently ignored. Fix: propagate the error. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/server_attrs.rs:28` — `has_server_flag` discards parse result; typos in `#[server(skiip)]` accepted silently. Fix: propagate errors. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/openapi_gen.rs:151` — `#[response(header = "X-Foo")]` without `value = "..."` silently drops the header. Fix: check `pending_header_name.is_some()` after loop and emit error. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/http.rs:600` — `struct_name_snake` uses `.to_lowercase()` instead of `.to_snake_case()` — `MyHttpService` → `"myhttpservice"`, distinct types can collide. Fix: use `to_snake_case()`. _(severity: medium)_
- [APPLIED] Multiple files — Two independent `HttpMethod` types (core and openapi_gen) with identical names/methods, no cross-reference. Fix: consolidate into one shared type. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/openapi_gen.rs:256` / `crates/server-less-core/src/lib.rs:444` — Two divergent `infer_path` implementations producing different results for the same input. Fix: consolidate. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/grpc.rs:166,169` — Generated methods are `proto_schema()`/`write_proto()` — breaks `{protocol}_schema()`/`write_{protocol}()` pattern. Fix: rename to `grpc_schema()`/`write_grpc()`. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/mcp.rs:51` — `mcp_tool_names()` inconsistent with `jsonrpc_methods()`/`ws_methods()`. Fix: rename to `mcp_method_names()`. _(severity: medium)_
- [APPLIED] `crates/server-less/src/lib.rs:382` — `#[param]` not included in the prelude. Fix: add to prelude. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/lib.rs:1597` — `#[server(skip)]` dual-role behavior (preset vs method-skip) undocumented. Fix: add doc comment explaining both roles. _(severity: medium)_
- [APPLIED] `crates/server-less/src/lib.rs:342` — `Config as ConfigTrait` alias confusing alongside `#[derive(Config)]`. Fix: rename trait to `ConfigLoad` at source. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/error.rs:128` — `#[error]` attribute clashes with `thiserror`'s `#[error("...")]`. Fix: add a doc warning about the clash. _(severity: medium)_
- [APPLIED] `crates/server-less/src/lib.rs:223` — Feature flags section omits `openrpc`, `asyncapi`, `jsonschema`, `markdown`, `capnp`, `thrift`, `smithy`, `connect`, `openapi`. Fix: add missing features. _(severity: medium)_
- [APPLIED] `README.md:3,120,239` — Contradictory test counts (466/329/171) and version shown as `"0.2"` while crate is at 0.4.9. Fix: update to consistent version and single test count. _(severity: medium)_
- [APPLIED] `docs/design/impl-first.md:30` — `Client` listed as a supported derive but no `#[client]` macro exists. Fix: annotate as "planned". _(severity: medium)_
- [APPLIED] `crates/server-less-parse/src/lib.rs:392` — `ParsedParamAttrs` all fields undocumented despite being public. Fix: add `///` to each field. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/app.rs:171` — `#[allow(dead_code)]` with "used by consuming macros once wired up" — already wired into 4 macros. Fix: remove the attribute. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/http.rs:160` — "backward compatibility" re-exports with no citation; `HttpMethodOverride` alias only used within `http.rs`. Fix: remove re-exports and alias. _(severity: medium)_
- [APPLIED] `crates/server-less/src/lib.rs:345,353` — `#[cfg(feature = "clap")]`/`#[cfg(feature = "axum")]` rely on implicit Cargo dep-features. Fix: use `#[cfg(feature = "cli")]`/`#[cfg(feature = "http")]`. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/lib.rs:662,706` — `ws_methods()`/`jsonrpc_methods()` docs say `Vec<&'static str>` but return `Vec<String>`. Fix: correct return types in docs. _(severity: medium)_
- [APPLIED] `crates/server-less-core/src/error.rs:276` — `ErrorCode` formats as `"INVALIDINPUT"` — should be `"INVALID_INPUT"`. Fix: implement `Display` or use a proper mapping. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/http.rs:830,878,908` (generic impls) — Generic `impl<T>` blocks: `_ty_generics` discarded, generated free functions can't reference `T`, produces confusing downstream compile errors. Fix: detect and emit a clear macro error. _(severity: medium — do last, complex)_
- [APPLIED] Multiple files — Methods with `#[cfg(...)]` attributes included unconditionally in dispatch/route generation. Fix: propagate `#[cfg]` attrs to generated dispatch arms. _(severity: medium — do last, complex)_

### LOW severity

- [APPLIED] `crates/server-less/src/lib.rs:359,363` — `futures`, `async_graphql`, `async_graphql_axum` are non-hidden public re-exports. Fix: add `#[doc(hidden)]`. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/app.rs:27` — `version: Option<Option<String>>` three-state encoding. Fix: introduce `enum VersionSpec { Auto, Disabled, Explicit(String) }`. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/cli.rs:139` — `name_prefix` attribute name is opaque. Fix: rename to `description_prefix`. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/program.rs:134` — `no_sync`/`no_async` not forwarded from `#[program]` — undocumented limitation. Fix: document the limitation. _(severity: low)_
- [APPLIED] `crates/server-less-core/src/error.rs:163` — `HttpStatusFallback`/`HttpStatusHelper` public but are implementation details. Fix: add `#[doc(hidden)]`. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/lib.rs:16` / `crates/server-less-parse/src/lib.rs:427` — `levenshtein`/`did_you_mean` duplicated verbatim. Fix: make parse version `pub`, remove duplicate in macros crate. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/config_derive.rs:41` — Local `is_option_type`/`inner_option_type` re-implement `server_less_parse` equivalents. Fix: use existing parse helpers. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/ws.rs:167` — `partition_ws_params`/`has_qualified_ws_sender` duplicates context detection pattern from `context.rs`. Fix: unify with a generic predicate. _(severity: low — do last, complex)_
- [APPLIED] `crates/server-less-macros/src/http.rs:1216` — `validate_http_path` byte-level slicing on path param names — could misbehave with non-ASCII. Fix: use char-aware slicing. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/openapi_gen.rs:682+` — `.expect("BUG: json!({}) must produce an Object")` in generated user code. Fix: use `unreachable!("BUG: ...")`. _(severity: low)_
- [APPLIED] `crates/server-less/src/lib.rs:54` — Prelude table missing `cli_run_with()`/`cli_run_with_async()` variants. Fix: add to table. _(severity: low)_
- [APPLIED] `crates/server-less-core/src/lib.rs:404` — `HttpMethod::as_str()` public but undocumented. Fix: add `///` doc comment. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/lib.rs` / à-la-carte macros — `title =` (openrpc/asyncapi/markdown/jsonschema) vs `name =` (http/cli/server/program) for same concept. Fix: standardize on `name =` to align with `#[app]`. _(severity: low)_

### DEFERRED — pending design decisions

- [DEFERRED] `crates/server-less-macros/src/http.rs` / `error.rs` — Unknown HTTP status codes (e.g. `code = 418`) silently map to `Internal`/500. Decision: reject at compile time, return 400 at runtime, or document the fallback? _(M8)_
- [DEFERRED] `crates/server-less-macros/src/http.rs` — `openapi_spec()` (per-impl) vs `combined_openapi_spec()` (serve-level) confusingly named. Decision: what should the two names be? _(M17)_
- [DEFERRED] `crates/server-less-macros/src/context.rs` — Context qualified/bare detection is impl-wide; one method with qualified form silently disables injection for others. Decision: change behavior (per-method scope) or document the footgun? _(M22)_
- [APPLIED] `crates/server-less-core/src/lib.rs:245` — `SchemaValueParser::variants` populated and leaked but `possible_values()` never implemented — leaked memory serves no purpose. Decision: implement `possible_values()` or remove the field? _(M27)_
- [DEFERRED] `CONTEXT_SUMMARY.md` — Written in "work just completed" style that agents misread as current design. Decision: add stale header, update, or delete? _(M32)_
- [APPLIED] `crates/server-less-core/src/error.rs:7` — `FailedPrecondition` is gRPC vocabulary for 422. Renamed to `UnprocessableEntity` in commit `3d4549e`. _(L5)_

## Findings — Round 2

### HIGH severity

- [APPLIED] `crates/server-less-macros/src/graphql.rs:769` — `value_to_graphql` nested fn emitted once per method inside `quote!` → N duplicate definitions in same scope → compile error for any service with >1 method. Fix: hoist to a single definition outside the per-method loop. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/graphql.rs:795` — All non-primitive Rust return types silently declared as `String` in GraphQL schema with no warning or error. Fix: emit a `syn::Error` for unmapped types (or at minimum a `compile_warning!`). _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/capnp.rs:104` — Missing `id` silently generates `@0x0000000000000000`; Cap'n Proto toolchain rejects zero IDs at schema compile time. Fix: emit a `syn::Error` requiring `id` or generate a deterministic non-zero ID with a warning. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/thrift.rs:262` — `Option<T>` maps to `"string"` for all `T` — loses both optionality and inner type. `Option<i32>` → Thrift `string`. Fix: unwrap `Option<T>` and map the inner type. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/smithy.rs:334` — `Vec<T>` produces a bare `List` shape name with no member type — invalid Smithy model. Fix: emit `List { member: { target: <inner_type> } }`. _(severity: high)_
- [APPLIED] `grpc.rs`, `capnp.rs`, `thrift.rs`, `smithy.rs`, `connect.rs`, `asyncapi.rs`, `jsonschema.rs` — All 7 unconditionally emit `#impl_block`, ignoring `is_protocol_impl_emitter`. Stacking with `#[http]`/`#[jsonrpc]`/`#[ws]`/`#[graphql]` produces duplicate `impl` blocks. Fix: gate on `is_protocol_impl_emitter` as `jsonrpc.rs`, `ws.rs`, `graphql.rs`, `openrpc.rs`, `markdown.rs` already do. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/asyncapi.rs:163` — `asyncapi_yaml()` is a fake YAML converter (string-replaces `{`, `}`, `[`, `]` on JSON output) — produces malformed non-YAML for any non-trivial schema. Fix: use a real YAML serialization crate, or remove the method and document the limitation. _(severity: high)_

### MEDIUM severity

- [APPLIED] `grpc.rs`, `capnp.rs`, `thrift.rs`, `smithy.rs`, `connect.rs`, `openrpc.rs`, `asyncapi.rs`, `markdown.rs`, `jsonschema.rs` — `has_server_skip`/`has_server_hidden` not respected; `#[server(skip)]` methods appear unconditionally in generated schemas/docs. Fix: filter via `has_server_skip`/`has_server_hidden` as reference macros do. _(severity: medium)_
- [APPLIED] `grpc.rs`, `capnp.rs`, `thrift.rs`, `smithy.rs`, `connect.rs`, `openrpc.rs`, `asyncapi.rs`, `jsonschema.rs` — `server_less::Context` params not excluded; appear as schema fields/proto messages/spec params. Fix: call `partition_context_params` and exclude Context params from schema output. _(severity: medium)_
- [APPLIED] All 12 audited macros (`graphql`, `grpc`, `capnp`, `thrift`, `smithy`, `connect`, `jsonrpc`, `ws`, `openrpc`, `asyncapi`, `markdown`, `jsonschema`) — `extract_app_meta` not called; `#[app(name, description, version)]` passthrough has no effect on schema titles, package names, or doc headings. Fix: call `extract_app_meta` and use values as fallbacks. _(severity: medium)_
- [APPLIED] `grpc.rs`, `capnp.rs`, `thrift.rs`, `smithy.rs`, `connect.rs`, `openrpc.rs`, `asyncapi.rs`, `jsonschema.rs` — `Option<T>`, `Vec<T>`, `Result<T,E>` inner types not unwrapped in type-mapping functions; raw outer type string passed directly, producing wrong schema types silently. Fix: extract inner types before mapping. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/graphql.rs:865` — `generate_resolver_arm` always returns `Ok(Value::String("todo"))` — dead stub wired into real generated methods. Fix: remove or replace with a proper dispatch or a clear `unimplemented!`. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/grpc.rs:268` — Custom types silently become `bytes` in proto schema; `Vec<User>` → `repeated string`, `User` → `bytes` with no diagnostic. Fix: emit an error or warning for unmapped types. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/capnp.rs:211` — Method ordinals `@0`, `@1`, ... regenerated from scratch each time; inserting a method reshuffles all ordinals, silently breaking wire compatibility. Fix: emit a warning that ordinal stability is the user's responsibility, or document prominently. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/smithy.rs:105` — Schema path read from a bare `#[schema = "..."]` impl-block attribute, while `grpc`/`capnp` accept `schema = "..."` as a macro argument. Fix: make smithy consistent with grpc/capnp. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/graphql.rs:158` / `jsonrpc.rs:108` / `ws.rs:325` — `#[app]` not extracted in these runtime macros either. Fix: call `extract_app_meta`. _(severity: medium)_

### LOW severity

- [APPLIED] `grpc.rs`, `capnp.rs`, `thrift.rs`, `smithy.rs`, `connect.rs`, `openrpc.rs`, `asyncapi.rs`, `markdown.rs`, `jsonschema.rs` — Unknown attribute key errors have no `did_you_mean` suggestion. Fix: add `did_you_mean` as reference macros do. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/graphql.rs:797` — Type detection uses `contains("DateTime")`, `contains("Uuid")`, `contains("Url")` substring matching — `UserUrl`, `UpdateDateTime` etc. falsely classified as custom scalars. Fix: match on the final path segment only. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/thrift.rs` — No `f32` mapping; falls through to `binary`. Fix: add `f32 → double` branch. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/smithy.rs:314` — Unsigned types (`u8`→`Byte`, `u16`→`Short`, etc.) silently mapped to signed Smithy types. Fix: document or add unsigned wrappers. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/ws.rs` — No path validation; `path = ""` or missing `/` prefix silently produces invalid axum route. Fix: validate in `expand_ws` as `http.rs` does. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/markdown.rs` — `#[param(help = "...")]` not read; parameter descriptions come only from type info. Fix: read `help` from `ParsedParamAttrs` when generating param table. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/thrift.rs:200` — Generated Thrift IDL uses a grouped-args-struct style that may not parse with standard Thrift IDL tooling. Fix: verify against Thrift reference parser or switch to inline param list style. _(severity: low)_
- [APPLIED] `openrpc.rs:132`, `asyncapi.rs:131`, `jsonschema.rs:123` — `serde_json::from_str(...).unwrap_or_default()` silently returns empty on parse failure, producing a spec with no methods. Fix: propagate the error. _(severity: low)_

### DEFERRED — pending design decisions

- [DEFERRED] `crates/server-less-macros/src/markdown.rs` — Should `server_less::Context` params be excluded from generated Markdown docs, or documented with a note? Decision: depends on whether a doc reader should see Context as an implementation detail or a visible concept. _(R2-M2 for markdown)_

## Findings — Round 3

### HIGH severity

- [APPLIED] `crates/server-less/src/lib.rs` — `#[param]` re-exported only under `#[cfg(feature = "http")]`; cli/mcp-only users get "undefined macro". Fix: re-export under `any(feature = "http", feature = "cli", feature = "mcp")`. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/mcp.rs` — `expand_mcp` never calls `extract_app_meta`; `#[app(name, description, version)]` silently ignored for MCP. Fix: add `extract_app_meta` call. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/grpc.rs:301–321`, `capnp.rs:307–342`, `smithy.rs:357–381` — Type mappers use `type_str.contains("i32")` substring matching; user types like `MyI32Wrapper` silently map to wrong wire type. Fix: compare on final path-segment ident. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/http.rs:836–845` — Multiple path params each generate their own `Path<T>` extractor; axum requires a single `Path<(T1,T2,...)>` tuple. Non-first params silently bind the wrong segment. Fix: collect path params and emit a single tuple extractor. _(severity: high)_
- [APPLIED] `crates/server-less-macros/src/openapi.rs:236` — `#[server(skip)]` / `#[server(hidden)]` silently ignored in `#[openapi]` standalone mode; only `#[route(skip/hidden)]` checked. Fix: add `has_server_skip` / `has_server_hidden` to filter condition. _(severity: high)_
- [APPLIED] `docs/design/error-mapping.md:94` — `FailedPrecondition` still in HTTP→ErrorCode table; renamed to `UnprocessableEntity` in commit `3d4549e`. Fix: update. _(severity: high)_
- [APPLIED] `docs/design/iteration-log.md:551,552,566,569,648`, `docs/tutorials/multi-protocol.md:361` — Stale references to `proto_schema()` / `write_proto()` (now `grpc_schema()` / `write_grpc()`). Fix: update all. _(severity: high)_
- [APPLIED] `POLISH.md:90,92` — M27 says `possible_values()` "never implemented" (it is, at `server-less-core/src/lib.rs:315`); L5 still `[DEFERRED]` (rename was applied in `3d4549e`). Fix: mark both `[APPLIED]`. _(severity: high)_

### MEDIUM severity

- [APPLIED] `crates/server-less-macros/src/openapi.rs:170,218` — `expand_openapi` never calls `extract_app_meta`; also hardcodes `.version("0.1.0")` instead of `env!("CARGO_PKG_VERSION")`. Fix: call `extract_app_meta`; use `CARGO_PKG_VERSION` as default. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/grpc.rs:111` et al. — `validate_server_attrs` not called in any of the 9 schema-generator macros (grpc, openrpc, jsonschema, asyncapi, capnp, thrift, smithy, connect, markdown); typos silently accepted. Fix: add call before skip/hidden filter. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/grpc.rs:232` et al. — `partition_context_params` not called in schema-generator macros; `Context` appears as a proto/schema field. Marked `[APPLIED]` in Round 2 — verify completeness before reapplying. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/grpc.rs` et al. — `#[cfg(...)]` attrs on methods not propagated in schema-generator macros. Marked `[APPLIED]` in Round 2 — verify completeness before reapplying. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/tool.rs` — `#[tool]` preset does not call `extract_app_meta`; `#[app]` silently ignored. Fix: add call. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/rpc_preset.rs` — `#[rpc]` preset missing `name`, `description`, `version`, `homepage` shorthand; `#[server]` supports all four. Fix: add to `RpcArgs`. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/http.rs:113` — Module doc says `http_routes()` is generated; no such method is emitted. Fix: implement it or remove from doc. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/openrpc.rs:200` — `generate_param_spec` ignores `help_text`; `#[param(help)]` has no effect on OpenRPC parameter descriptions. Fix: include in `description` field. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/openrpc.rs:115–119` — `expand_openrpc` discards `description` and `homepage` from `app_meta`; design doc lists both as consumers. Fix: add to `OpenRpcArgs` and generated spec `info`. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/thrift.rs:108` — `_app_meta` discarded entirely; `#[grpc]` uses `name` as service name fallback. Fix: use `app_meta.name` in Thrift service name. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/openrpc.rs:229`, `asyncapi.rs:294`, `jsonschema.rs:271` — `Option<T>` generates `{"anyOf": [null, ...]}` — bare `null` is invalid JSON Schema; correct form is `{"anyOf": [{"type": "null"}, ...]}`. Fix: replace `null` literal. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/openapi_gen.rs:279` — `infer_path` hardcodes `/{id}` regardless of actual parameter names. Fix: use actual param names in path template. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/openapi_gen.rs:232–235` — Unrecognized method name prefixes silently default to `POST` with no diagnostic. Fix: emit a `compile_note!` or proc-macro note when defaulting. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/thrift.rs:300,308` — `type_str.contains("Vec<u8>")` (no spaces) never matches `quote!` output; `contains("HashMap")` falsely matches `HashMapWrapper`. Fix: use AST-based checks. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/lib.rs:150–162` — Generic impl hard-rejected; error message doesn't suggest concrete-type workaround. Fix: improve message. _(severity: medium)_
- [APPLIED] `crates/server-less-macros/src/grpc.rs:153–168`, `thrift.rs:163–176`, `capnp.rs:176–192` — Schema validation uses unordered line-set comparison; reordering proto fields passes silently. Fix: document limitation prominently in the generated comment. _(severity: medium)_
- [APPLIED] `crates/server-less/src/lib.rs:53–57` — Prelude table missing: `http_openapi_paths()`, `mcp_method_names()`, `ws_methods()`, `jsonrpc_handle_async()`. Fix: add all four. _(severity: medium)_
- [APPLIED] `crates/server-less/src/lib.rs:351–352` — `ConfigLoad`, `ConfigError`, `ConfigFieldMeta`, `ConfigSource` all `#[doc(hidden)]` but used in module-level doc examples. Fix: un-hide. _(severity: medium)_
- [APPLIED] `docs/design/open-questions.md:5–13` — "Default Action on Subcommand Groups" marked Resolved but sits above the `## Resolved Questions` heading. Fix: move to resolved section. _(severity: medium)_

### LOW severity

- [APPLIED] `crates/server-less-macros/src/rpc_preset.rs:47`, `tool.rs:41` — Unknown-arg errors have no `did_you_mean`; all other macros include it. Fix: add. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/openapi.rs:93` — Unknown-arg error has no `did_you_mean`. Fix: add. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/markdown.rs:107`, `openrpc.rs:104`, `asyncapi.rs:110`, `jsonschema.rs:105`, `openapi.rs:170` — Five macros don't call `reject_generic_impl`; generic `impl<T>` silently proceeds. Fix: add call. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/mcp.rs:443`, `graphql.rs:736` — Injected Context uses `Context::default()`; all ws/jsonrpc sites use `Context::new()`. Fix: standardise on `Context::new()`. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/http.rs:215`, `server.rs:51` — `HttpArgs`/`ServerArgs` require `=` for the `openapi` key (`#[http(openapi)]` fails to parse); `ServeArgs` makes `=` optional. Fix: align to `ServeArgs` pattern or document. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/graphql.rs:163` — `_effective_name` computed from `args.name.or(app_meta.name)` but stored in dead `_`-prefixed local; `app_meta.version`/`description`/`homepage` unconsumed. Fix: wire name into schema or add explaining comment. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/ws.rs:314`, `jsonrpc.rs:110` — `_app_meta` extracted and discarded with no comment; `#[app]` silently does nothing for these macros. Fix: add explaining comment. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/mcp.rs` — `#[param(name = "...")]` wire-rename silently ignored for MCP tool parameter naming. Fix: apply `param.name` override when building tool schema. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/asyncapi.rs` — `generate_param_property` ignores `help_text`; AsyncAPI parameter descriptions always empty. Fix: include as `description`. _(severity: low)_
- [APPLIED] `docs/design/param-attributes.md:100` — "Multiple positional arguments not yet implemented" caveat is stale; `cli.rs:663–666` already tracks `pos_idx`. Fix: remove caveat. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/http.rs:622–623` — `to_snake_case()` on struct name for handler idents; structs differing only in separator style produce name collisions. Fix: document as known constraint. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/cli.rs:459` — `to_kebab_case()` for default app name produces `h-t-t-p-server` for `HTTPServer` with no warning. Fix: document that `name = "..."` is the escape hatch. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/cli.rs:1194` — `Vec<T>` in `type_to_json_schema_ty` emits `items: {}` — inner type discarded. Fix: recurse into `T`. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/cli.rs:1549–1556` — `--params-json` Vec deserialization always uses `Vec<String>` regardless of actual element type. Fix: use actual element type. _(severity: low)_
- [APPLIED] `crates/server-less-macros/src/context.rs:27` — Qualified Context detection hardcodes `"server_less"`; fails silently if crate is aliased. Fix: document limitation. _(severity: low)_
- [APPLIED] `crates/server-less-core/src/lib.rs:264,415`, `crates/server-less-parse/src/lib.rs:150` — `SchemaValueParser::new()` and `HttpMethod::as_str()` (both crates) are undocumented `pub fn`. Fix: add `///` one-liners. _(severity: low)_
- [APPLIED] `README.md:161–165` — Examples section lists only 5 of 13 example files. Fix: expand or add note linking to `examples/`. _(severity: low)_

### DEFERRED — pending design decisions

- [DEFERRED] `crates/server-less-macros/src/jsonrpc.rs` — `#[jsonrpc]` has no sync `jsonrpc_handle()`; `#[ws]` has both sync and async variants. Intentional or gap? _(R3-M7)_
- [DEFERRED] `crates/server-less-macros/src/connect.rs` — `#[connect]` missing `schema = "path"` arg and `validate_schema()` / `assert_schema_matches()` methods present on `#[grpc]`. Feature scope decision. _(R3-M8)_
- [DEFERRED] `crates/server-less-macros/src/jsonschema.rs` — `json_schema()` naming breaks `{proto}_spec()` convention; rename to `jsonschema_spec()` would be a breaking change. _(R3-M9)_
- [DEFERRED] `docs/design/app-metadata.md:69` — `#[cli(no_version)]` documented but not implemented. Implement now or keep as planned? _(R3-M14)_
- [DEFERRED] `crates/server-less-macros/src/openapi_gen.rs:261–265` — Naive pluralization produces `/classs`, `/processs` etc. Fix requires non-trivial logic or inflector dep; `#[route(path)]` is the reliable override. _(R3-M16)_
- [DEFERRED] `crates/server-less-parse/src/lib.rs:1069–1072` — `is_id_param` triggers on `android_id`, `void_id` etc. making them positional/path params. Narrowing is a behavior change with breaking potential. _(R3-M19)_
- [DEFERRED] `docs/design/app-metadata.md:141` — `#[server(homepage = false)]` suppression mechanism not implemented. Design decision. _(R3-L10)_
- [DEFERRED] `openrpc.rs:132`, `asyncapi.rs:131`, `jsonschema.rs:123` — `unwrap_or_default()` silently returns empty on parse failure. Marked `[APPLIED]` in Round 2 — verify whether apply was complete. _(R3-L12)_
