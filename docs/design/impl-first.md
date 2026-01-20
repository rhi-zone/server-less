# Impl-First Design

Trellis takes an **impl-first** approach: write your Rust methods, derive macros project them into protocols.

```rust
impl MyService {
    /// Create a new user
    fn create_user(&self, name: String, email: String) -> Result<User, UserError> { ... }

    /// Get user by ID
    fn get_user(&self, id: UserId) -> Option<User> { ... }

    /// List all users
    fn list_users(&self, limit: u32, offset: u32) -> Vec<User> { ... }

    /// Watch for new users
    fn watch_users(&self) -> impl Stream<Item=User> { ... }
}
```

Each derive macro projects this into a protocol:

| Derive | Projection |
|--------|------------|
| `Http` | REST endpoints |
| `Grpc` | Protobuf service |
| `GraphQL` | Schema + resolvers |
| `Cli` | Command-line interface |
| `Mcp` | MCP tools for LLMs |
| `Client` | Type-safe client SDK |
| `OpenApi` | Spec from types + docs |

## Naming Conventions

### HTTP Verb Inference

| Method prefix | HTTP verb | Path pattern |
|---------------|-----------|--------------|
| `create_*`, `add_*` | POST | `/resources` |
| `get_*`, `fetch_*` | GET | `/resources/{id}` |
| `list_*`, `find_*`, `search_*` | GET | `/resources` |
| `update_*`, `set_*` | PUT | `/resources/{id}` |
| `delete_*`, `remove_*` | DELETE | `/resources/{id}` |
| anything else | POST | `/rpc/{method_name}` |

### Parameter Conventions

```rust
fn get_user(id: UserId) -> User
//          ^^^ path param: ends with "Id" or named "id"

fn list_users(limit: u32, offset: u32) -> Vec<User>
//            ^^^ query params: GET + primitive types

fn create_user(name: String, email: String) -> User
//             ^^^ body: POST/PUT + non-id params
```

### CLI Conventions

```rust
fn create_user(name: String, email: String) -> User
// → myapp create-user --name "..." --email "..."

fn get_user(id: UserId) -> Option<User>
// → myapp get-user <ID>
//                  ^^^ positional: single id-like arg
```

### MCP Conventions

```rust
/// Search for users by name
fn search_users(&self, query: String, limit: Option<u32>) -> Vec<User>
```

Becomes:
```json
{
  "name": "search_users",
  "description": "Search for users by name",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": { "type": "string" },
      "limit": { "type": "integer" }
    },
    "required": ["query"]
  }
}
```

- Method name → tool name
- Doc comment → tool description
- Parameters → input schema
- `Option<T>` → optional parameter
- Return type → tool result

## Return Types

The return type is the **API contract**:

| Type | Meaning | HTTP | CLI |
|------|---------|------|-----|
| `T` | Success with data | 200 + body | stdout |
| `Option<T>` | Maybe not found | 200 or 404 | stdout or exit 1 |
| `Result<T, E>` | Success or typed error | 200 or error from E | stdout or stderr |
| `()` | Success, no data | 204 No Content | silent |
| `Vec<T>` | Collection (collected) | 200 + JSON array | JSON array |
| `impl Stream<T>` | Streaming | SSE/WebSocket | newline-delimited |

### Streaming

`impl Stream<Item=T>` means streaming. For non-streaming protocols:

```rust
#[http(collect)]  // explicitly opt-in to collecting
fn watch_events(&self) -> impl Stream<Item=Event>
```

Without `#[http(collect)]`, using `Http` derive on a streaming method is a compile error. This avoids hidden memory bombs.

## Error Handling

Errors are typed via `Result<T, E>`:

```rust
enum UserError {
    NotFound,       // → 404 (convention: contains "NotFound")
    InvalidEmail,   // → 400 (convention: "Invalid*")
    Forbidden,      // → 403 (exact match)
    AlreadyExists,  // → 409 (convention: "AlreadyExists", "*Conflict")
}
```

Or explicit mapping:
```rust
#[derive(Error)]
enum UserError {
    #[error(http = 404, grpc = "NOT_FOUND")]
    NotFound,
}
```

Each protocol derive maps errors appropriately:
- HTTP → status code + JSON body
- gRPC → status code + details
- CLI → stderr + exit code
- GraphQL → errors array
- MCP → error response

## Overrides

Conventions work 80% of the time. Override when needed:

```rust
#[http(path = "/api/v1/users", method = "POST")]
fn register(&self, ...) -> User  // wouldn't infer POST from "register"

#[cli(name = "add")]
fn create_user(&self, ...) -> User  // override CLI subcommand name

#[mcp(name = "find_users", description = "Search the user database")]
fn search_users(&self, ...) -> Vec<User>  // override MCP tool metadata
```

## Protocol-Specific Context

Some protocols need context (headers, metadata, etc.):

```rust
fn create_user(
    &self,
    ctx: Context,  // injected: HTTP headers, gRPC metadata, CLI env, etc.
    name: String,
    email: String,
) -> Result<User, UserError>
```

`Context` is protocol-agnostic. Each derive provides the relevant data:
- HTTP: headers, cookies, query params
- gRPC: metadata
- CLI: env vars, config files
- MCP: conversation context (if available)
