# Building Multi-Protocol Services with Server-less

This tutorial shows how to expose the same service over multiple protocols simultaneously: HTTP, WebSocket, JSON-RPC, GraphQL, CLI, and MCP.

## The Power of Server-less

Write your business logic **once**, expose it **everywhere**:

```rust
impl Calculator {
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}
```

Becomes available as:
- HTTP: `POST /add`
- WebSocket: `{"method": "add", "params": {"a": 5, "b": 3}}`
- JSON-RPC: `{"jsonrpc": "2.0", "method": "add", ...}`
- GraphQL: `mutation { add(a: 5, b: 3) }`
- CLI: `mycalc add 5 3`
- MCP: Tool for Claude/LLMs

## Project Setup

```toml
[dependencies]
server-less = { git = "https://github.com/rhi-zone/server-less", features = ["full"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

## Step 1: Define Your Service

```rust
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

#[derive(Clone)]
pub struct TaskService {
    tasks: Arc<Mutex<Vec<Task>>>,
}

impl TaskService {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
```

## Step 2: Implement Business Logic

```rust
impl TaskService {
    /// Create a new task
    pub fn create_task(&self, title: String) -> Task {
        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            completed: false,
        };
        self.tasks.lock().unwrap().push(task.clone());
        task
    }

    /// List all tasks
    pub fn list_tasks(&self) -> Vec<Task> {
        self.tasks.lock().unwrap().clone()
    }

    /// Get a specific task
    pub fn get_task(&self, id: String) -> Option<Task> {
        self.tasks
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    /// Complete a task
    pub fn complete_task(&self, id: String) -> Option<Task> {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
            task.completed = true;
            Some(task.clone())
        } else {
            None
        }
    }

    /// Delete a task
    pub fn delete_task(&self, id: String) -> Option<Task> {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(idx) = tasks.iter().position(|t| t.id == id) {
            Some(tasks.remove(idx))
        } else {
            None
        }
    }
}
```

## Step 3: Add Protocol Support

Now the magic happens - add derive macros to expose your service:

### HTTP REST API

```rust
use server_less::http;

#[http(prefix = "/api")]
impl TaskService {
    // All methods automatically exposed as REST endpoints
}
```

### WebSocket

```rust
use server_less::ws;

#[ws(path = "/ws")]
impl TaskService {
    // All methods available via WebSocket JSON-RPC
}
```

### JSON-RPC

```rust
use server_less::jsonrpc;

#[jsonrpc(path = "/rpc")]
impl TaskService {
    // JSON-RPC 2.0 server
}
```

### GraphQL

```rust
use server_less::graphql;

#[graphql]
impl TaskService {
    // GraphQL queries and mutations
    // get_*, list_* → queries
    // create_*, update_*, delete_* → mutations
}
```

### CLI Application

```rust
use server_less::cli;

#[cli(name = "tasks", version = "1.0.0")]
impl TaskService {
    // Command-line interface
    // Methods become subcommands
}
```

### MCP Tools (for LLMs)

```rust
use server_less::mcp;

#[mcp(namespace = "tasks")]
impl TaskService {
    // Model Context Protocol tools
    // Callable by Claude and other LLMs
}
```

## Step 4: Combine Everything with `#[serve]`

The `#[serve]` macro combines all protocols into one server:

```rust
use server_less::{serve, http, ws, jsonrpc, graphql};

#[derive(Clone)]
pub struct TaskService {
    tasks: Arc<Mutex<Vec<Task>>>,
}

#[http(prefix = "/api")]
#[ws(path = "/ws")]
#[jsonrpc(path = "/rpc")]
#[graphql]
#[serve]
impl TaskService {
    pub fn create_task(&self, title: String) -> Task { /* ... */ }
    pub fn list_tasks(&self) -> Vec<Task> { /* ... */ }
    pub fn get_task(&self, id: String) -> Option<Task> { /* ... */ }
    pub fn complete_task(&self, id: String) -> Option<Task> { /* ... */ }
    pub fn delete_task(&self, id: String) -> Option<Task> { /* ... */ }
}
```

## Step 5: Run Your Server

```rust
#[tokio::main]
async fn main() {
    let service = TaskService::new();
    let app = service.serve();  // All protocols in one!

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Multi-protocol server running at http://localhost:3000");
    println!("- HTTP REST:  http://localhost:3000/api/tasks");
    println!("- WebSocket:  ws://localhost:3000/ws");
    println!("- JSON-RPC:   http://localhost:3000/rpc");
    println!("- GraphQL:    http://localhost:3000/graphql");

    axum::serve(listener, app).await.unwrap();
}
```

## Step 6: Use Each Protocol

### HTTP REST

```bash
# Create task
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{"title": "Buy groceries"}'

# List tasks
curl http://localhost:3000/api/tasks

# Get task
curl http://localhost:3000/api/tasks/{id}
```

### WebSocket

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = () => {
    ws.send(JSON.stringify({
        method: 'create_task',
        params: { title: 'Buy groceries' }
    }));
};

ws.onmessage = (event) => {
    console.log('Response:', JSON.parse(event.data));
};
```

### JSON-RPC

```bash
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_task",
    "params": {"title": "Buy groceries"},
    "id": 1
  }'
```

### GraphQL

```bash
curl -X POST http://localhost:3000/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { createTask(title: \"Buy groceries\") { id title completed } }"
  }'

# Or use the GraphQL Playground at http://localhost:3000/graphql
```

### CLI

Build a CLI binary:

```rust
// src/bin/tasks-cli.rs
use server_less::cli;

#[cli(name = "tasks", version = "1.0.0")]
impl TaskService { /* ... */ }

#[tokio::main]
async fn main() {
    let service = TaskService::new();
    let matches = TaskService::cli_app().get_matches();
    service.cli_run(&matches);
}
```

```bash
cargo build --bin tasks-cli

# Use it
./target/debug/tasks-cli create-task "Buy groceries"
./target/debug/tasks-cli list-tasks
./target/debug/tasks-cli complete-task {id}
```

### MCP (Model Context Protocol)

Expose your service to Claude:

```rust
use server_less::mcp;

#[mcp(namespace = "tasks")]
impl TaskService { /* ... */ }

// Generate tool definitions
let tools = TaskService::mcp_tools();

// Claude can now call:
// tasks_create_task(title: "Buy groceries")
// tasks_list_tasks()
```

## Protocol Comparison

| Protocol | Best For | Request Format |
|----------|----------|----------------|
| **HTTP** | REST APIs, browsers | HTTP methods + JSON |
| **WebSocket** | Real-time, bidirectional | JSON-RPC over WS |
| **JSON-RPC** | RPC-style APIs | JSON-RPC 2.0 |
| **GraphQL** | Flexible queries, mobile | GraphQL SDL |
| **CLI** | Command-line tools | Shell arguments |
| **MCP** | LLM integration | JSON tool definitions |

## Schema Generation

Generate schemas for all protocols:

```rust
// Protocol Buffers
use server_less::grpc;
#[grpc(package = "tasks.v1")]
impl TaskService { /* ... */ }
let proto = TaskService::proto_schema();

// OpenAPI
let openapi = TaskService::openapi_spec();

// AsyncAPI (WebSocket)
let asyncapi = TaskService::asyncapi_spec();

// OpenRPC (JSON-RPC)
let openrpc = TaskService::openrpc_spec();
```

## Best Practices

### 1. Keep Methods Protocol-Agnostic

```rust
// ✅ Good - works everywhere
pub fn create_task(&self, title: String) -> Task { }

// ❌ Bad - HTTP-specific
pub fn create_task(&self, req: HttpRequest) -> HttpResponse { }
```

### 2. Use Result Types for Errors

```rust
use server_less::Server-lessError;

#[derive(Debug, Server-lessError)]
enum TaskError {
    #[error(code = NotFound)]
    TaskNotFound,
}

pub fn get_task(&self, id: String) -> Result<Task, TaskError> {
    // Proper error handling across all protocols
}
```

### 3. Document Everything

```rust
impl TaskService {
    /// Create a new task with the given title
    ///
    /// The task will be created in an incomplete state
    pub fn create_task(&self, title: String) -> Task {
        // Doc comments become:
        // - OpenAPI descriptions
        // - GraphQL field docs
        // - CLI help text
        // - MCP tool descriptions
    }
}
```

### 4. Use Semantic Method Names

```rust
// Queries (read-only)
pub fn get_task(&self, id: String) -> Option<Task> { }
pub fn list_tasks(&self) -> Vec<Task> { }
pub fn search_tasks(&self, query: String) -> Vec<Task> { }

// Mutations (modify state)
pub fn create_task(&self, title: String) -> Task { }
pub fn update_task(&self, id: String, title: String) -> Task { }
pub fn delete_task(&self, id: String) -> Option<Task> { }
```

## Architecture Patterns

### Shared State

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct TaskService {
    db: Arc<RwLock<Database>>,
}
```

### Middleware

```rust
use tower::ServiceBuilder;

let app = service.serve()
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive())
    );
```

### Feature Flags

```rust
#[cfg(feature = "http")]
#[http]
impl TaskService { }

#[cfg(feature = "graphql")]
#[graphql]
impl TaskService { }
```

## Performance Tips

1. **Clone is cheap** - Services are typically `Arc<T>` under the hood
2. **Async everything** - Use `async fn` for I/O operations
3. **Connection pooling** - Share database pools across protocols
4. **Caching** - Add caching layer that all protocols benefit from

## Deployment

All protocols run on a single port:

```rust
// Production setup
let app = service.serve()
    .layer(ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(CorsLayer::permissive())
    );

axum::serve(listener, app).await?;
```

## What You Learned

- ✅ Write business logic once, expose it everywhere
- ✅ Combine HTTP, WebSocket, JSON-RPC, GraphQL, CLI, MCP
- ✅ Generate schemas automatically
- ✅ Protocol-agnostic error handling
- ✅ Single server, multiple protocols

## Next Steps

- Add authentication across all protocols
- Implement subscriptions (GraphQL/WebSocket)
- Add rate limiting
- Deploy with Docker
- Monitor with OpenTelemetry

## Complete Example

See `examples/multi-protocol/` in the Server-less repository for the full code.

---

**The beauty of Server-less**: Your business logic stays clean and protocol-agnostic. Add new protocols by adding a single derive macro!
