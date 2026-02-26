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

## Auth/Context Injection

How do methods access request context (headers, user info, metadata)?

```rust
// Option A: Magic parameter
fn create_user(&self, ctx: Context, name: String) -> User

// Option B: Method on self
fn create_user(&self, name: String) -> User {
    let user_id = self.context().user_id();  // where does context come from?
}

// Option C: Separate trait
impl Authenticated<MyServer> {
    fn create_user(&self, name: String) -> User {
        self.user_id()  // available because Authenticated
    }
}
```

**Needs:** Real auth requirements to determine what context is actually needed.

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

## Protocol-Specific Overrides

How much protocol-specific annotation is acceptable before it defeats the purpose?

```rust
// Too much?
#[http(method = "POST", path = "/api/v1/users", content_type = "application/json")]
#[grpc(name = "CreateUser", package = "users.v1")]
#[graphql(mutation, name = "createUser")]
#[cli(subcommand = "user create", positional = ["name"])]
fn create_user(&self, name: String) -> User
```

**Needs:** Real cases where conventions fail to determine what overrides are actually needed.

## Error Type Unification

Should there be a server-less error trait, or just conventions?

```rust
// Option A: Trait with protocol mappings
#[derive(ServerlessError)]
enum MyError {
    #[error(http = 404, grpc = "NOT_FOUND", cli_exit = 1)]
    NotFound,
}

// Option B: Just conventions
enum MyError {
    NotFound,  // all protocols infer from name
}

// Option C: Per-protocol derives
#[derive(HttpError, GrpcError, CliError)]
enum MyError { ... }
```

**Needs:** Real error handling pain across protocols.

## CLI / Clap Depth

The `#[cli]` macro works for flat subcommands. Several directions to deepen it:

### Nested Command Groups

**Decision: Composition via types.**

Struct fields define the command hierarchy. Each type is independently annotated with `#[cli]` (and other protocol macros). The parent explicitly delegates via `#[cli(subcommand)]` accessor methods:

```rust
struct Users;

#[cli]
#[http]
impl Users {
    fn create(&self, name: String) -> User { ... }
    fn list(&self) -> Vec<User> { ... }
    fn delete(&self, id: UserId) { ... }
}

struct MyApp { users: Users }

#[cli(name = "myapp")]
#[http(prefix = "/api")]
impl MyApp {
    /// Delegate to Users subcommand group
    #[cli(subcommand)]
    fn users(&self) -> &Users { &self.users }

    fn health(&self) -> String { "ok".into() }
}
```

Each protocol projects the nesting independently:
- **CLI:** `myapp users create --name foo`, `myapp users list`, `myapp health`
- **HTTP:** `POST /api/users`, `GET /api/users`, `DELETE /api/users/{id}`, `GET /api/health`
- **MCP:** `users_create`, `users_list`, `users_delete`, `health` (namespace prefix from field name)

Why this approach:
- **Just Rust** — struct fields are the hierarchy, no new syntax
- **Automatic detection via trait bound (serde model)** — methods returning `&T` (no other params) are structurally distinct from command methods (which have params or return owned values). The macro generates `<T as CliSubcommand>::cli_command()` and lets the compiler resolve the trait bound. If `T` doesn't have `#[cli]`, you get a clear error. Opt out with `#[cli(skip)]` for getters that aren't subcommand groups. Prior art: serde generates `field.serialize()` for all fields and lets `T: Serialize` resolve or fail — no explicit `#[serde(serialize)]` needed
- **Composable** — each type is self-contained, works with any protocol independently
- **Recursive by composition** — each type's `#[cli]` generates `cli_command()` independently; the parent nests the child's `cli_command()` as a subcommand. No recursive macro expansion, just method calls. Depth is unlimited but naturally self-limiting (deep CLIs are a UX problem, not a macro problem).
- **State flows naturally** — parent dispatches via `&self.users`, child has its own `&self`

Alternatives considered:
- **Naming conventions** (`user_create` → `user create`): fragile, no structural boundary
- **Associated types** (`type User: UserCommands`): not valid on inherent impls, needs trait overhead
- **`#[cli(group = "users")]` on separate impl blocks**: coordination between macro invocations is hard
- **`mod`-like syntax in impl blocks**: not valid Rust

### Richer Type Mapping

Currently: `String` → required, `Option<T>` → optional, `bool` → flag. Real CLIs need:
- `Vec<T>` for multi-value args
- Enums as value variants
- `PathBuf` for file args
- Passthrough `#[arg(...)]` attributes to clap

### Output Formatting (Prior Art: normalize)

Normalize's CLI has a well-developed output system worth studying. Every command supports:

- `--json` — full JSON output
- `--jsonl` — JSON Lines (arrays emit each element as a separate line)
- `--jq <EXPR>` — filter JSON through jq expressions (using `jaq` crate, not shelling out)
- `--jq <EXPR> --jsonl` — jq + JSONL compose: jq results are exploded as JSONL
- `--pretty` / `--compact` — human-friendly vs LLM-optimized text
- `--output-schema` / `--input-schema` — emit JSON Schema for the command's return/input types
- `--params-json <JSON>` — pass arguments as JSON instead of CLI flags

Key design decisions:
1. **All output flags are global** (defined once on root `Cli` struct with `#[arg(global = true)]`), not repeated per command.
2. **`OutputFormatter` trait** — types implement `format_text()` + `format_pretty()`, get `print(&OutputFormat)` for free. JSON comes from `Serialize`.
3. **Schema from schemars** — all arg/output types derive `schemars::JsonSchema`, enabling programmatic discovery.
4. **jq is in-process** via `jaq` crate — no subprocess, no `jq` binary dependency.

For server-less, this should **not** be forced. The baseline `#[cli]` should just give you a CLI — no trait bounds required on return types. But if your types happen to implement the right traits, features unlock automatically:

```rust
// Level 0: Just works. No bounds needed. Output is Debug or ToString.
#[cli(name = "myapp")]
impl MyApp {
    fn greet(&self, name: String) -> String { format!("hi {name}") }
}

// Level 1: Return type is Serialize → --json and --jsonl appear automatically
fn list_users(&self) -> Vec<User> { ... }  // where User: Serialize

// Level 2: Return type also has JsonSchema → --output-schema unlocks too
fn list_users(&self) -> Vec<User> { ... }  // where User: Serialize + JsonSchema

// Level 3: jq support could be a feature flag on the cli feature itself
// server-less = { features = ["cli"] }       → no jq
// server-less = { features = ["cli-jq"] }    → adds --jq (pulls in jaq)
```

The macro detects trait bounds at expansion time (or uses feature flags for deps like jaq) so the user never sees an error about missing impls they didn't opt into. You discover `--json` when you notice it in `--help` because your types already qualified.

Everything auto-detected can be explicitly disabled:
```rust
#[cli(name = "myapp", json = false)]   // no --json even though types are Serialize
#[cli(name = "myapp", jsonl = false)]  // no --jsonl
#[cli(name = "myapp", jq = false)]     // no --jq even if cli-jq feature is on
#[cli(name = "myapp", schema = false)] // no --output-schema / --input-schema
```

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

(Move questions here once resolved with rationale)
