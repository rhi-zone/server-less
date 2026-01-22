# Trellis Tutorials

Learn Trellis by building real applications.

## Getting Started

New to Trellis? Start here:

### [Building a REST API](./rest-api.md)
Learn the fundamentals by building a blog API:
- Convention-based routing
- Type-safe handlers
- Error handling
- OpenAPI generation
- Custom route overrides

**Time:** 30 minutes
**Level:** Beginner

---

### [Multi-Protocol Services](./multi-protocol.md)
Write business logic once, expose it everywhere:
- HTTP REST API
- WebSocket (JSON-RPC)
- GraphQL
- CLI applications
- MCP for LLMs
- Protocol-agnostic design

**Time:** 45 minutes
**Level:** Intermediate

---

## Quick Start

Want to jump right in? Here's a minimal example:

```rust
use rhizome_trellis::http;

#[derive(Clone)]
struct HelloService;

#[http]
impl HelloService {
    /// Say hello
    async fn get_hello(&self, name: String) -> String {
        format!("Hello, {}!", name)
    }
}

#[tokio::main]
async fn main() {
    let service = HelloService;
    let app = service.http_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
```

```bash
curl http://localhost:3000/hello?name=World
# Output: "Hello, World!"
```

## Core Concepts

### Convention Over Configuration

Trellis uses method name prefixes to infer HTTP methods:

```rust
// GET /users
fn list_users(&self) -> Vec<User> { }

// POST /users
fn create_user(&self, user: User) -> User { }

// GET /users/{id}
fn get_user(&self, id: String) -> Option<User> { }

// PUT /users/{id}
fn update_user(&self, id: String, user: User) -> User { }

// DELETE /users/{id}
fn delete_user(&self, id: String) -> Option<User> { }
```

### Progressive Disclosure

Start simple, add complexity when needed:

```rust
// Level 1: Just works
#[http]
impl Service { }

// Level 2: Customize
#[http(prefix = "/api/v1")]
impl Service { }

// Level 3: Per-method overrides
#[route(method = "POST", path = "/custom")]
fn my_method(&self) { }

// Level 4: Escape hatch - write your own axum handlers
```

### Protocol-Agnostic Logic

Your business logic shouldn't know about protocols:

```rust
impl Service {
    // ✅ Good - works with HTTP, GraphQL, CLI, etc.
    pub fn calculate(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    // ❌ Bad - tied to HTTP
    pub fn calculate(&self, req: HttpRequest) -> HttpResponse {
        // ...
    }
}
```

## Learning Path

1. **Start with HTTP** - [REST API Tutorial](./rest-api.md)
2. **Add protocols** - [Multi-Protocol Tutorial](./multi-protocol.md)
3. **Read design docs** - [Design Philosophy](../design/)
4. **Check examples** - Repository `/examples` directory

## Common Patterns

### Error Handling

```rust
use rhizome_trellis::TrellisError;

#[derive(Debug, TrellisError)]
enum MyError {
    #[error(code = NotFound, message = "User not found")]
    UserNotFound,

    #[error(code = InvalidInput)]
    ValidationFailed(String),
}
```

### State Management

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct Service {
    db: Arc<RwLock<Database>>,
}
```

### Async Methods

```rust
#[http]
impl Service {
    // Async methods are automatically awaited
    async fn fetch_data(&self) -> Result<Data, Error> {
        self.db.read().await.get_data().await
    }
}
```

## Need Help?

- **Issues**: [GitHub Issues](https://github.com/rhizome-lab/trellis/issues)
- **Discussions**: [GitHub Discussions](https://github.com/rhizome-lab/trellis/discussions)
- **Examples**: See `/examples` in the repository

## Contributing

Found an issue in a tutorial? Have a tutorial idea?

Open an issue or PR at [github.com/rhizome-lab/trellis](https://github.com/rhizome-lab/trellis)

---

Happy building! 🚀
