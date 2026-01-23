# Building a REST API with Server-less

This tutorial walks through building a complete REST API for a blog application using Server-less.

## What We're Building

A blog API with:
- Posts (CRUD operations)
- Comments
- Authors
- OpenAPI documentation
- Error handling

## Setup

Add Server-less to your `Cargo.toml`:

```toml
[dependencies]
server-less = { git = "https://github.com/rhizome-lab/server-less" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

## Step 1: Define Your Data Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
    pub author_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published: Option<bool>,
}
```

## Step 2: Create Your Service

```rust
use server_less::http;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct BlogService {
    posts: Arc<Mutex<Vec<Post>>>,
}

impl BlogService {
    pub fn new() -> Self {
        Self {
            posts: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
```

## Step 3: Add HTTP Handlers

Server-less uses **convention over configuration**. Method names determine HTTP methods and paths:

```rust
#[http(prefix = "/api/v1")]
impl BlogService {
    /// Create a new post
    pub async fn create_post(&self, req: CreatePostRequest) -> Post {
        let post = Post {
            id: uuid::Uuid::new_v4().to_string(),
            title: req.title,
            content: req.content,
            author_id: req.author_id,
            published: false,
        };

        self.posts.lock().unwrap().push(post.clone());
        post
    }

    /// Get a post by ID
    pub async fn get_post(&self, id: String) -> Option<Post> {
        self.posts
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    /// List all posts
    pub async fn list_posts(&self) -> Vec<Post> {
        self.posts.lock().unwrap().clone()
    }

    /// Update a post
    pub async fn update_post(&self, id: String, req: UpdatePostRequest) -> Option<Post> {
        let mut posts = self.posts.lock().unwrap();
        if let Some(post) = posts.iter_mut().find(|p| p.id == id) {
            if let Some(title) = req.title {
                post.title = title;
            }
            if let Some(content) = req.content {
                post.content = content;
            }
            if let Some(published) = req.published {
                post.published = published;
            }
            Some(post.clone())
        } else {
            None
        }
    }

    /// Delete a post
    pub async fn delete_post(&self, id: String) -> Option<Post> {
        let mut posts = self.posts.lock().unwrap();
        if let Some(idx) = posts.iter().position(|p| p.id == id) {
            Some(posts.remove(idx))
        } else {
            None
        }
    }
}
```

## Generated Routes

Server-less automatically generates these routes:

| Method | Path | Handler |
|--------|------|---------|
| `POST` | `/api/v1/posts` | `create_post` |
| `GET` | `/api/v1/posts/{id}` | `get_post` |
| `GET` | `/api/v1/posts` | `list_posts` |
| `PUT` | `/api/v1/posts/{id}` | `update_post` |
| `DELETE` | `/api/v1/posts/{id}` | `delete_post` |

## Step 4: Run Your Server

```rust
#[tokio::main]
async fn main() {
    let service = BlogService::new();
    let app = service.http_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Server running at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
```

## Step 5: Test Your API

```bash
# Create a post
curl -X POST http://localhost:3000/api/v1/posts \
  -H "Content-Type: application/json" \
  -d '{
    "title": "My First Post",
    "content": "Hello, world!",
    "author_id": "author-1"
  }'

# List posts
curl http://localhost:3000/api/v1/posts

# Get a specific post
curl http://localhost:3000/api/v1/posts/{id}

# Update a post
curl -X PUT http://localhost:3000/api/v1/posts/{id} \
  -H "Content-Type: application/json" \
  -d '{
    "published": true
  }'

# Delete a post
curl -X DELETE http://localhost:3000/api/v1/posts/{id}
```

## Step 6: Customize Routes (Optional)

Override default routing with `#[route]` attribute:

```rust
#[http(prefix = "/api/v1")]
impl BlogService {
    /// Publish a post
    #[route(method = "POST", path = "/posts/{id}/publish")]
    pub async fn publish_post(&self, id: String) -> Option<Post> {
        // Custom endpoint: POST /api/v1/posts/{id}/publish
        // ...
    }

    /// Internal helper - don't expose as HTTP endpoint
    #[route(skip)]
    fn internal_validation(&self, post: &Post) -> bool {
        !post.title.is_empty()
    }
}
```

## Step 7: Add Error Handling

Use Server-less error types for proper HTTP status codes:

```rust
use server_less::Server-lessError;

#[derive(Debug, Server-lessError)]
pub enum BlogError {
    #[error(code = NotFound, message = "Post not found")]
    PostNotFound,

    #[error(code = InvalidInput)]
    InvalidTitle(String),

    #[error(code = Forbidden, message = "Cannot delete published posts")]
    CannotDeletePublished,
}

// Update methods to return Result
pub async fn get_post(&self, id: String) -> Result<Post, BlogError> {
    self.posts
        .lock()
        .unwrap()
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or(BlogError::PostNotFound)
}
```

## Step 8: Access OpenAPI Documentation

Server-less automatically generates OpenAPI specs:

```rust
// Get the OpenAPI JSON
let spec = BlogService::openapi_spec();
println!("{}", serde_json::to_string_pretty(&spec).unwrap());
```

Or serve it as an endpoint:

```rust
use axum::routing::get;
use axum::Json;

let app = service.http_router()
    .route("/openapi.json", get(|| async {
        Json(BlogService::openapi_spec())
    }));
```

## Next Steps

- Add authentication with middleware
- Add database persistence (PostgreSQL, SQLite)
- Add pagination to `list_posts`
- Add search and filtering
- Deploy to production

## Complete Example

See the full example in `examples/blog-api/` in the Server-less repository.

## What You Learned

- ✅ Convention-based routing (method names → HTTP methods)
- ✅ Automatic path generation
- ✅ Type-safe request/response handling
- ✅ Custom route overrides with `#[route]`
- ✅ Error handling with proper status codes
- ✅ Automatic OpenAPI generation

Ready for more? Check out the [Multi-Protocol Services](./multi-protocol.md) tutorial!
