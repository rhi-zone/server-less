# Open Questions

Design questions that need real use cases to drive decisions.

## Bidirectional Streaming

What does a bidirectional streaming method look like?

```rust
// Option A: Two streams
fn chat(&self, input: impl Stream<Item=Message>) -> impl Stream<Item=Message>

// Option B: Channel-based
fn chat(&self) -> (Sender<Message>, Receiver<Message>)

// Option C: Callback-based
fn chat(&self, on_message: impl Fn(Message) -> Option<Message>)
```

**Needs:** Real use case (chat? live collaboration?) to determine ergonomics.

## API Versioning

How to handle breaking changes?

```rust
// Option A: Attribute-based
#[version("v1")]
fn get_user_v1(&self, id: UserId) -> UserV1

#[version("v2")]
fn get_user(&self, id: UserId) -> User

// Option B: Module-based
mod v1 { impl MyService { ... } }
mod v2 { impl MyService { ... } }

// Option C: Just use different structs
struct MyServiceV1;
struct MyServiceV2;
```

**Needs:** Real versioning pain to determine what's worth abstracting.

## MCP Beyond Tools

MCP has more than tools:
- **Resources** - files, data the LLM can read
- **Prompts** - pre-built prompt templates
- **Sampling** - letting the server request LLM completions

How do these map to Rust?

```rust
// Tools are methods (covered)
fn search_users(&self, query: String) -> Vec<User>

// Resources?
#[mcp(resource)]
fn user_data(&self, id: UserId) -> Resource<UserData>

// Prompts?
#[mcp(prompt)]
fn summarize_prompt(&self) -> Prompt {
    prompt!("Summarize the following: {input}")
}
```

**Needs:** Real MCP server use case to understand what's valuable.


## CLI / Clap Depth

The `#[cli]` macro works for flat subcommands. Several directions to deepen it:

### Shell Completions and Man Pages

`clap_complete` and `clap_mangen` are low-effort, high-value. Could auto-generate a `completions` subcommand.

### CLI as Client Mode

When `#[http]` and `#[cli]` are on the same service, the CLI could act as a client to the HTTP server:

```rust
#[cli(mode = "client", endpoint = "http://localhost:3000")]
impl MyService { ... }
// Generates: `mycli create-user --name foo` → POST /users
```

### CLI Design Principles (from normalize)

Worth adopting as conventions for generated CLIs:
- **Group by domain, not verb** — `users list` not `list-users`
- **Positional args for primary targets** — `mycli get-user <id>` not `mycli get-user --id <id>`
- **`list` as subcommand, not flag** — consistent across all resource types
- **Filters compose** — multiple filters AND together, no special cases
- **`--dry-run` on every mutating command**

**Needs:** Real CLI use case beyond toy examples to determine priority.

## Middleware Ordering

When composing multiple extensions, does order matter? How to control it?

```rust
#[derive(ServerCore, Auth, RateLimit, Logging, Serve)]
// Is this: Logging(RateLimit(Auth(Core)))?
// Or: Auth(RateLimit(Logging(Core)))?
// Does it matter?
```

**Needs:** Real middleware composition to understand ordering requirements.

## Testing Generated Code

How should users test code that uses server-less derives?

- Mock the generated server?
- Test the impl directly?
- Integration test the whole thing?

**Needs:** Real testing pain to determine what helpers are useful.

---

## Resolved Questions

### Error Type Unification

**Resolved: `#[derive(ServerlessError)]` with inference.** See [error-mapping.md](./error-mapping.md).

Variant names are inferred to `ErrorCode` values (e.g., `NotFound` → 404, `InvalidInput` → 400). Explicit overrides via `#[error(code = NotFound)]` or `#[error(code = 409)]`. Each protocol maps the `ErrorCode` to its native representation (HTTP status, gRPC code, CLI exit code, JSON-RPC error code).

### Auth/Context Injection

**Resolved: Magic `Context` parameter.** Parameters typed `Context` are injected from the request context rather than extracted from the API caller's input. The `Context` type is re-exported from `server-less-core`.

### Nested Command Groups

**Resolved: Automatic detection via `&T` return type.** See [mount-points.md](./mount-points.md).

Methods returning `&T` where `T: CliSubcommand` are automatically detected as subcommand group delegation. No explicit `#[cli(subcommand)]` attribute needed — opt out with `#[cli(skip)]` for getters that aren't subcommand groups. The same pattern applies across protocols: `HttpMount`, `McpNamespace`, `WsMount`, `JsonRpcMount`.

### Output Formatting

**Resolved: Display by default, opt-in JSON.** See [cli-output-formatting.md](./cli-output-formatting.md).

Default output uses `Display` (human-readable). Global flags `--json`, `--jsonl`, `--jq <expr>` opt into machine-readable output. jq filtering uses the `jaq` library in-process (no external binary). `--output-schema` and `--input-schema` emit JSON Schema via `schemars`. `display_with` escape hatch for custom formatting.

### Protocol-Specific Overrides

**Resolved: Layered attribute system.**

- **HTTP:** `#[route(method = "POST", path = "/custom")]` for method/path overrides, `#[route(skip)]` to exclude
- **Parameters:** `#[param(query)]`, `#[param(path)]`, `#[param(body)]`, `#[param(header)]` for HTTP placement; `#[param(short = 'x')]`, `#[param(positional)]`, `#[param(help = "...")]` for CLI; `#[param(name = "...")]` and `#[param(default = ...)]` cross-protocol
- **CLI:** `#[cli(skip)]`, `#[cli(display_with = "...")]` per-method
- See [param-attributes.md](./param-attributes.md) and [inference-vs-configuration.md](./inference-vs-configuration.md)

### Richer Type Mapping

**Resolved: Implemented for common types.**

`bool` → `SetTrue` flag, `Vec<T>` → `Append` (comma-delimited), `Option<T>` → optional flag. `#[param(positional)]` for explicit positional args, `_id` heuristic for automatic positional. `#[param(short = 'x')]` for short flags.
