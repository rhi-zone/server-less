# Inference vs. Configuration

Server-less infers as much as possible from names and types. Explicit configuration is the exception, not the rule — but it's always available.

## The Pattern

1. Look at the function name and parameter types.
2. Apply a small set of well-known conventions to make a decision.
3. If the decision would surprise a reasonable user, require explicit config instead.
4. Always provide an escape hatch.

This is the same model as Rails conventions or serde's rename rules: do the obvious thing by default, let users override when the obvious thing is wrong.

## HTTP: Verb and Path Inference

### Method → HTTP verb

The verb is inferred from the function name prefix:

| Prefix | HTTP verb |
|--------|-----------|
| `get_*`, `fetch_*`, `read_*`, `list_*`, `find_*`, `search_*` | GET |
| `create_*`, `add_*`, `new_*` | POST |
| `update_*`, `set_*` | PUT |
| `patch_*`, `modify_*` | PATCH |
| `delete_*`, `remove_*` | DELETE |
| anything else | POST (RPC fallback) |

```rust
fn create_user(...)  // → POST /users
fn get_user(...)     // → GET  /users/{id}
fn list_users(...)   // → GET  /users
fn delete_user(...)  // → DELETE /users/{id}
fn register(...)     // → POST /registers (fallback — probably wrong, see below)
```

The fallback to POST for unrecognized prefixes is intentional: POST is the safe choice for unknown operations, and the path will signal that something is off (`/registers`), prompting the user to add an explicit override.

### Parameter name → path vs. query vs. body

Parameters are placed based on their name and the HTTP verb:

- Parameter named `id` or ending in `_id` → path parameter (`{id}`)
- GET + non-id parameters → query string
- POST/PUT/PATCH + non-id parameters → JSON body
- `Context` → injected from request headers, not a user-provided parameter

```rust
fn get_user(&self, user_id: u64) -> Option<User>
// → GET /users/{id}  (user_id ends with _id → path)

fn list_users(&self, limit: u32, offset: u32) -> Vec<User>
// → GET /users?limit=...&offset=...  (GET + non-id → query)

fn create_user(&self, name: String, email: String) -> User
// → POST /users  with JSON body {name, email}  (POST + non-id → body)
```

The `_id` suffix convention is deliberately conservative: `user_id` and `post_id` are unambiguously identifiers, but `identity` is not (it doesn't end with `_id`, it just contains the substring `id`). This precision prevents false positives.

### Override: `#[route(...)]`

When inference produces the wrong result, `#[route]` overrides it:

```rust
#[route(method = "POST", path = "/api/v1/auth/register")]
fn register(&self, ...) -> User  // "register" doesn't match any prefix

#[route(path = "/users/{user_id}/profile")]
fn get_user_profile(&self, user_id: u64) -> Profile  // custom path

#[route(skip)]
fn internal_helper(&self, ...)  // excluded from HTTP routing entirely

#[route(hidden)]
fn admin_reset(&self, ...)  // routed but excluded from OpenAPI spec
```

### Override: `#[param(...)]`

Individual parameters can be relocated:

```rust
fn create_user(
    &self,
    #[param(path)] tenant: String,   // force into path even though not *_id
    #[param(header, name = "X-API-Key")] api_key: String,
    name: String,  // body (inferred from POST)
) -> User
```

Valid locations: `path`, `query`, `body`, `header`.

`#[param(name = "q")]` renames the wire name without moving the parameter. `#[param(default = 10)]` supplies a default for query parameters.

## CLI: Argument Style Inference

### Type → argument kind

The CLI argument style is inferred from the parameter type:

| Type | Clap argument |
|------|--------------|
| `bool` | Flag (`--verbose`, stores true when present) |
| `Vec<T>` | Append (`--tag foo --tag bar`, or `--tag foo,bar`) |
| name ends in `_id` or is `id` | Positional (`<ID>`) |
| `Option<T>` | Optional named (`--limit <LIMIT>`, not required) |
| anything else | Named (`--name <NAME>`, required) |

```rust
fn create_user(&self, name: String, email: String)
// → myapp create-user --name "Alice" --email "alice@example.com"

fn get_user(&self, user_id: u64)
// → myapp get-user <USER_ID>  (positional, because user_id ends with _id)

fn deploy(&self, dry_run: bool, services: Vec<String>)
// → myapp deploy --dry-run --services web,api
//              ^^^^^^^^^^ SetTrue    ^^^^^^^^^^^^^^^^ Append
```

Boolean flags are `SetTrue` because the alternative — requiring `--verbose true` — is unexpectedly verbose and not what users expect from a flag. This is an unambiguous convention.

`Vec` becomes append because users expect repeatable flags or comma-separated values, not JSON arrays on the command line.

`_id` parameters become positional because a command with a single identifier argument reads better as `myapp get-user 42` than `myapp get-user --user-id 42`.

### Override: `#[cli(skip)]` and `#[cli(hidden)]`

Exclude a method from the CLI entirely, or keep it accessible but hide it from help and shell completions:

```rust
#[cli(skip)]
fn internal_method(&self, ...)  // not exposed as a subcommand at all

#[cli(hidden)]
fn debug_dump(&self, ...)  // subcommand exists but doesn't appear in --help
```

`#[cli(default, hidden)]` is a common combination: a method that's the fallback action for a subcommand group but isn't listed in help output.

### Override: method-level CLI name

Override the subcommand name:

```rust
#[cli(name = "add")]
fn create_user(&self, ...)  // subcommand is "add", not "create-user"
```

## HTTP: Debug and Trace Logging

Two opt-in attributes emit diagnostic output from generated handlers. Neither affects behavior — they only add logging.

### `#[http(debug = true)]` — request/response logging

Emits `eprintln!` lines before and after each method call: the method name, incoming parameters, and return value. Apply to the impl block to enable for all methods, or to a specific method to enable for just that one.

```rust
#[http(debug = true)]
impl MyService {
    fn get_user(&self, user_id: u64) -> Option<User> { ... }
    // → prints: "[http] get_user called with user_id=42"
    // → prints: "[http] get_user returned Some(User { ... })"
}
```

Per-method override:
```rust
#[http]
impl MyService {
    #[http(debug = true)]
    fn flaky_endpoint(&self, ...) -> ... { ... }  // only this one is logged
}
```

### `#[http(trace = true)]` — parameter extraction tracing

Emits an `eprintln!` line after each parameter is extracted from the request, showing the parameter name and its `{:?}` value. Useful for debugging extraction issues (wrong location, type mismatch, missing fields).

```rust
#[http(trace = true)]
impl MyService {
    fn create_user(&self, name: String, role: Role) -> User { ... }
    // → prints: "[http] extracted name = \"Alice\""
    // → prints: "[http] extracted role = Admin"
}
```

Both flags can be combined. `debug` logs around the method call; `trace` logs inside the extraction phase.

## When Not to Infer

Conventions should be **unsurprising**. If applying a convention would produce output that doesn't match what a developer familiar with the domain would expect, don't apply it — require explicit configuration.

Examples where inference would surprise users:

- `fn process_batch(...)` — is this GET or POST? "process" doesn't suggest either. Falls back to POST, which is reasonable, but the generated path `/processs` looks wrong. The error is surfaced (strange plural form) so the user will notice and add `#[route(path = "/batch/process")]`.
- A `bool` parameter named `is_admin` that controls a privilege — making it a flag means `--is-admin` sets it to true, which is correct behavior but could be a security footgun if the user expected it to be a required positional. In this case, inference is still right (a boolean flag is always SetTrue), but the user should be aware.
- A parameter named `kind` that happens to be a `Vec<String>` — it becomes append. This is correct. But if the user named it `kindness`, the CLI flag would be `--kindness` which is fine. Naming is the user's responsibility; inference only acts on types.

The guiding test: **would a developer reading the function signature immediately understand the generated behavior?** If yes, infer. If no, require explicit config.

## Cross-Protocol Skip and Hidden

When a method should be excluded from every protocol at once, use `#[server(skip)]` instead of repeating per-protocol attributes:

```rust
// Instead of:
#[cli(skip)]
#[route(skip)]
// ... and so on for every protocol
fn internal(&self, ...) { ... }

// Write:
#[server(skip)]
fn internal(&self, ...) { ... }
```

Similarly, `#[server(hidden)]` exposes a method but suppresses it from protocol-level discoverability — CLI help/completions, OpenAPI spec, etc.:

```rust
#[server(hidden)]
fn debug_dump(&self, ...) { ... }
// reachable as a subcommand, but not listed in --help
// excluded from the OpenAPI spec
```

Per-protocol attributes (`#[cli(skip)]`, `#[route(skip)]`, etc.) still work for finer control — `#[server(skip)]` is a convenience, not a replacement.

## Private Methods

Methods starting with `_` are excluded from all protocol projections unconditionally. No attribute needed:

```rust
fn _helper(&self, ...)  // never exposed — not HTTP, not CLI, not MCP
```

This is a Rust naming convention (leading underscore = private/internal) applied consistently. It's more ergonomic than a `#[server(skip)]` on every internal helper, and it composes with the explicit attribute: `_` for truly private implementation details, `#[server(skip)]` when the method name shouldn't start with an underscore but still shouldn't be exposed.

## Inspecting Generated Code

Two tools let you see exactly what code the macros produce:

- `cargo expand` — expands all macros in a file and prints the result. The output is readable Rust.
- `SERVER_LESS_DEBUG=1 cargo build` — prints generated code to stderr for every macro invocation as the build runs. Useful when `cargo expand` doesn't isolate the right macro.

```bash
SERVER_LESS_DEBUG=1 cargo build 2>&1 | grep -A 50 "=== cli ==="
```

## Error Messages: "Did You Mean?"

When an unknown attribute argument is used, server-less computes the Levenshtein distance between the typo and all valid arguments and suggests the closest match:

```
error: unknown argument `nane` — did you mean `name`?
```

This applies to all macros (`#[cli]`, `#[http]`, `#[server]`, `#[program]`, etc.). The suggestion is suppressed if no valid argument is close enough to avoid misleading suggestions.

## Prior Art

**Rails conventions.** `UsersController#index` → `GET /users`, `#show` → `GET /users/:id`, `#create` → `POST /users`. The resource name is inferred from the controller name; the verb from the action name. Overrides are possible via `routes.rb`. Server-less follows the same premise: verb from method prefix, resource from the suffix, id from the parameter name.

**Serde's rename rules.** `#[serde(rename_all = "camelCase")]` applies a transformation; `#[serde(rename = "foo")]` overrides it for a single field. Server-less follows the same layering: a global rule (prefix → verb), overridable per-item (`#[route(method = "...")]`).

**The escape hatch is always granular.** You don't have to abandon `#[http]` to fix one route. You override just that method. This is the progressive disclosure principle: inference handles 80% of cases, attributes handle the rest, dropping to manual Tower code is the nuclear option.
