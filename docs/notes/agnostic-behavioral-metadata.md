# Agnostic Behavioral Op Metadata — Gap Finding

_Surfaced by the fractal design session, mid-2026._

## The gap

server-less has no protocol-neutral place to declare behavioral properties of an operation:
whether it is safe/read-only, idempotent, destructive, or open-world (can affect resources
not named in the call). Every projection that needs this information currently either
duplicates a name-prefix heuristic, silently falls back to a worst-case default, or omits
the signal entirely. The agnostic key set that should exist in one place, consumed by all
projections, is:

| Key          | Meaning                                                         |
|--------------|-----------------------------------------------------------------|
| `safe`       | Pure read — no side effects (HTTP safe, GraphQL Query)          |
| `idempotent` | Repeating is equivalent to one call (HTTP PUT/DELETE, gRPC IDEMPOTENT) |
| `destructive`| Data loss on success (DELETE, drop, wipe)                       |
| `openWorld`  | May affect resources not named in arguments (MCP openWorldHint) |

## Per-projection evidence (verified)

### gRPC — `idempotency_level` not emitted

`crates/server-less-macros/src/grpc.rs` — `generate_proto_method` emits only the `rpc`
line with an optional `stream` prefix. No `option (idempotency_level) = IDEMPOTENT;` or
`= NO_SIDE_EFFECTS;` annotation is produced anywhere in the file. The proto3 field exists
in the gRPC spec and is consumed by proxies and generated clients to enable safe retries,
but server-less currently has no input from which to derive it.

### MCP — behavioral hints not emitted

`crates/server-less-macros/src/mcp.rs` — `generate_tool_definition` (around line 401)
emits exactly three keys: `name`, `description`, and `inputSchema`. No `annotations` block
is constructed. The MCP 2025-11 spec defines four opt-in hint fields inside `annotations`:
`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`. None are emitted.

### HTTP (OpenAPI) — name-prefix heuristic with silent POST fallback

`crates/server-less-macros/src/openapi_gen.rs:227–249` — `infer_http_method` maps
name prefixes to verbs:

- `get_`, `fetch_`, `read_`, `list_`, `find_`, `search_` → GET
- `create_`, `add_`, `new_` → POST
- `update_`, `set_` → PUT
- `patch_`, `modify_` → PATCH
- `delete_`, `remove_` → DELETE
- anything else → POST (silent fallback, noted in inline comment at lines 245–247)

The silent POST fallback means an operation like `send_email` or `trigger_job` is assigned
POST with no indication that the inference was uncertain. A `safe` annotation would let the
macro emit GET confidently without a name-prefix, and `idempotent` would justify PUT over
POST for methods that do not match an `update_` prefix.

Note: `openapi_gen.rs` does not call `has_server_skip`; it parses `#[route(skip)]` into a
`RouteOverride.skip` field directly (lines 42–44). This is an implementation detail that
does not affect the behavioral-metadata gap.

### GraphQL — parallel name-prefix heuristic, different prefix set

`crates/server-less-macros/src/graphql.rs:658–669` — `is_query_method` classifies a
method as a Query (vs. Mutation) by name prefix:

- Query prefixes: `get_`, `fetch_`, `read_`, `list_`, `find_`, `search_`, `count_`,
  `exists_`, `is_`, `has_`
- Mutation: everything else

The HTTP and GraphQL heuristics are maintained separately and have diverged: the GraphQL
version recognises `count_`, `exists_`, `is_`, `has_` as safe but the HTTP version does
not. Both heuristics have the same underlying failure mode — a method that is safe but
does not carry a recognised prefix is silently treated as a mutating operation.

### CLI — no per-op confirm prompt or `--dry-run` generated

No code in `crates/server-less-macros/src/cli.rs` generates a `--confirm` flag or a
`--dry-run` flag based on any property of a method. The only `dry_run` occurrences in the
codebase are:

- `config_cmd.rs` — the config-set subcommand's own `--dry-run` flag, unrelated to
  per-op generation.
- `cli_tests.rs` — test infrastructure for the `#[cli(global = [dry_run])]` feature, a
  user-defined global rather than an auto-generated safety gate.

A `destructive` annotation would be the natural input for generating a confirmation
prompt or `--dry-run` flag on high-risk CLI operations.

## Recommendation

Introduce a protocol-neutral op-level annotation, tentatively `#[op(...)]` or as arguments
on the existing per-projection attributes, accepting some subset of `safe`, `idempotent`,
`destructive`, `openWorld`. Each projection macro reads these annotations and maps them to
the appropriate protocol signal:

| Annotation    | gRPC                              | MCP                  | HTTP         | GraphQL       | CLI                    |
|---------------|-----------------------------------|----------------------|--------------|---------------|------------------------|
| `safe`        | `NO_SIDE_EFFECTS`                 | `readOnlyHint=true`  | GET          | Query         | (no special output)    |
| `idempotent`  | `IDEMPOTENT`                      | `idempotentHint=true`| PUT/DELETE   | (no mapping)  | (no special output)    |
| `destructive` | (no standard proto field)         | `destructiveHint=true`| DELETE/POST | Mutation      | `--confirm` / `--dry-run` |
| `openWorld`   | (no standard proto field)         | `openWorldHint=true` | (informational) | (no mapping) | (no special output) |

The name-prefix heuristics in `infer_http_method` and `is_query_method` should remain as
fallbacks when no explicit annotation is present, to preserve backward compatibility. The
two heuristics should also be reconciled to share the same prefix set.

See TODO.md for the tracking item.
