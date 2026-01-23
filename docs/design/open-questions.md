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
