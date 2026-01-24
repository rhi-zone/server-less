//! Proc macros for server-less.
//!
//! This crate provides attribute macros that transform impl blocks into protocol handlers,
//! and derive macros for common patterns.

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemImpl, parse_macro_input};

#[cfg(feature = "asyncapi")]
mod asyncapi;
#[cfg(feature = "capnp")]
mod capnp;
#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "connect")]
mod connect;
mod context;
mod error;
#[cfg(feature = "graphql")]
mod graphql;
#[cfg(feature = "grpc")]
mod grpc;
#[cfg(feature = "http")]
mod http;
#[cfg(feature = "jsonrpc")]
mod jsonrpc;
#[cfg(feature = "jsonschema")]
mod jsonschema;
#[cfg(feature = "markdown")]
mod markdown;
#[cfg(feature = "mcp")]
mod mcp;
#[cfg(feature = "http")]
mod openapi;
#[cfg(feature = "openrpc")]
mod openrpc;
#[cfg(feature = "smithy")]
mod smithy;
#[cfg(feature = "thrift")]
mod thrift;
#[cfg(feature = "ws")]
mod ws;

/// Generate HTTP handlers from an impl block.
///
/// # Basic Usage
///
/// ```ignore
/// use server_less::http;
///
/// #[http]
/// impl UserService {
///     async fn create_user(&self, name: String) -> User { /* ... */ }
/// }
/// ```
///
/// # With URL Prefix
///
/// ```ignore
/// #[http(prefix = "/api/v1")]
/// impl UserService {
///     // POST /api/v1/users
///     async fn create_user(&self, name: String) -> User { /* ... */ }
/// }
/// ```
///
/// # Per-Method Route Overrides
///
/// ```ignore
/// #[http]
/// impl UserService {
///     // Override HTTP method: GET /data becomes POST /data
///     #[route(method = "POST")]
///     async fn get_data(&self, payload: String) -> String { /* ... */ }
///
///     // Override path: POST /users becomes POST /custom-endpoint
///     #[route(path = "/custom-endpoint")]
///     async fn create_user(&self, name: String) -> User { /* ... */ }
///
///     // Override both
///     #[route(method = "PUT", path = "/special/{id}")]
///     async fn do_something(&self, id: String) -> String { /* ... */ }
///
///     // Skip route generation (internal methods)
///     #[route(skip)]
///     fn internal_helper(&self) -> String { /* ... */ }
///
///     // Hide from OpenAPI but still generate route
///     #[route(hidden)]
///     fn secret_endpoint(&self) -> String { /* ... */ }
/// }
/// ```
///
/// # Parameter Handling
///
/// ```ignore
/// #[http]
/// impl BlogService {
///     // Path parameters (id, post_id, etc. go in URL)
///     async fn get_post(&self, post_id: u32) -> Post { /* ... */ }
///     // GET /posts/{post_id}
///
///     // Query parameters (GET methods use query string)
///     async fn search_posts(&self, query: String, tag: Option<String>) -> Vec<Post> {
///         /* ... */
///     }
///     // GET /posts?query=rust&tag=tutorial
///
///     // Body parameters (POST/PUT/PATCH use JSON body)
///     async fn create_post(&self, title: String, content: String) -> Post {
///         /* ... */
///     }
///     // POST /posts with body: {"title": "...", "content": "..."}
/// }
/// ```
///
/// # Error Handling
///
/// ```ignore
/// #[http]
/// impl UserService {
///     // Return Result for error handling
///     async fn get_user(&self, id: u32) -> Result<User, MyError> {
///         if id == 0 {
///             return Err(MyError::InvalidId);
///         }
///         Ok(User { id, name: "Alice".into() })
///     }
///
///     // Return Option - None becomes 404
///     async fn find_user(&self, email: String) -> Option<User> {
///         // Returns 200 with user or 404 if None
///         None
///     }
/// }
/// ```
///
/// # Server-Sent Events (SSE) Streaming
///
/// Return `impl Stream<Item = T>` to enable Server-Sent Events streaming.
///
/// **Important for Rust 2024:** You must add `+ use<>` to impl Trait return types
/// to explicitly capture all generic parameters in scope. This is required by the
/// Rust 2024 edition's stricter lifetime capture rules.
///
/// ```ignore
/// use futures::stream::{self, Stream};
///
/// #[http]
/// impl DataService {
///     // Simple stream - emits values immediately
///     // Note the `+ use<>` syntax for Rust 2024
///     fn stream_numbers(&self, count: u32) -> impl Stream<Item = u32> + use<> {
///         stream::iter(0..count)
///     }
///
///     // Async stream with delays
///     async fn stream_events(&self, n: u32) -> impl Stream<Item = Event> + use<> {
///         stream::unfold(0, move |count| async move {
///             if count >= n {
///                 return None;
///             }
///             tokio::time::sleep(Duration::from_secs(1)).await;
///             Some((Event { id: count }, count + 1))
///         })
///     }
/// }
/// ```
///
/// Clients receive data as SSE:
/// ```text
/// data: {"id": 0}
///
/// data: {"id": 1}
///
/// data: {"id": 2}
/// ```
///
/// **Why `+ use<>`?**
/// - Rust 2024 requires explicit capture of generic parameters in return position impl Trait
/// - `+ use<>` captures all type parameters and lifetimes from the function context
/// - Without it, you'll get compilation errors about uncaptured parameters
/// - See: examples/streaming_service.rs for a complete working example
///
/// # Real-World Example
///
/// ```ignore
/// #[http(prefix = "/api/v1")]
/// impl UserService {
///     // GET /api/v1/users?page=0&limit=10
///     async fn list_users(
///         &self,
///         #[param(default = 0)] page: u32,
///         #[param(default = 20)] limit: u32,
///     ) -> Vec<User> {
///         /* ... */
///     }
///
///     // GET /api/v1/users/{user_id}
///     async fn get_user(&self, user_id: u32) -> Result<User, ApiError> {
///         /* ... */
///     }
///
///     // POST /api/v1/users with body: {"name": "...", "email": "..."}
///     #[response(status = 201)]
///     #[response(header = "Location", value = "/api/v1/users/{id}")]
///     async fn create_user(&self, name: String, email: String) -> Result<User, ApiError> {
///         /* ... */
///     }
///
///     // PUT /api/v1/users/{user_id}
///     async fn update_user(
///         &self,
///         user_id: u32,
///         name: Option<String>,
///         email: Option<String>,
///     ) -> Result<User, ApiError> {
///         /* ... */
///     }
///
///     // DELETE /api/v1/users/{user_id}
///     #[response(status = 204)]
///     async fn delete_user(&self, user_id: u32) -> Result<(), ApiError> {
///         /* ... */
///     }
/// }
/// ```
///
/// # Generated Methods
/// - `http_router() -> axum::Router` - Complete router with all endpoints
/// - `http_routes() -> Vec<&'static str>` - List of route paths
/// - `openapi_spec() -> serde_json::Value` - OpenAPI 3.0 specification (unless `openapi = false`)
///
/// # OpenAPI Control
///
/// By default, `#[http]` generates both HTTP routes and OpenAPI specs. You can disable
/// OpenAPI generation:
///
/// ```ignore
/// #[http(openapi = false)]  // No openapi_spec() method generated
/// impl MyService { /* ... */ }
/// ```
///
/// For standalone OpenAPI generation without HTTP routing, see `#[openapi]`.
#[cfg(feature = "http")]
#[proc_macro_attribute]
pub fn http(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as http::HttpArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match http::expand_http(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate OpenAPI specification without HTTP routing.
///
/// Generates OpenAPI 3.0 specs using the same naming conventions as `#[http]`,
/// but without creating route handlers. Useful for:
/// - Schema-first development
/// - Documentation-only use cases
/// - Separate OpenAPI generation from HTTP routing
///
/// # Basic Usage
///
/// ```ignore
/// use server_less::openapi;
///
/// #[openapi]
/// impl UserService {
///     /// Create a new user
///     fn create_user(&self, name: String, email: String) -> User { /* ... */ }
///
///     /// Get user by ID
///     fn get_user(&self, id: String) -> Option<User> { /* ... */ }
/// }
///
/// // Generate spec:
/// let spec = UserService::openapi_spec();
/// ```
///
/// # With URL Prefix
///
/// ```ignore
/// #[openapi(prefix = "/api/v1")]
/// impl UserService { /* ... */ }
/// ```
///
/// # Generated Methods
///
/// - `openapi_spec() -> serde_json::Value` - OpenAPI 3.0 specification
///
/// # Combining with #[http]
///
/// If you want separate control over OpenAPI generation:
///
/// ```ignore
/// // Option 1: Disable OpenAPI in http, use standalone macro
/// #[http(openapi = false)]
/// #[openapi(prefix = "/api")]
/// impl MyService { /* ... */ }
///
/// // Option 2: Just use http with default (openapi = true)
/// #[http]
/// impl MyService { /* ... */ }
/// ```
#[cfg(feature = "http")]
#[proc_macro_attribute]
pub fn openapi(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as openapi::OpenApiArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match openapi::expand_openapi(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate a CLI application from an impl block.
///
/// # Basic Usage
///
/// ```ignore
/// use server_less::cli;
///
/// #[cli]
/// impl MyApp {
///     fn create_user(&self, name: String) { /* ... */ }
/// }
/// ```
///
/// # With All Options
///
/// ```ignore
/// #[cli(
///     name = "myapp",
///     version = "1.0.0",
///     about = "My awesome application"
/// )]
/// impl MyApp {
///     /// Create a new user (becomes: myapp create-user <NAME>)
///     fn create_user(&self, name: String) { /* ... */ }
///
///     /// Optional flags use Option<T>
///     fn list_users(&self, limit: Option<usize>) { /* ... */ }
/// }
/// ```
///
/// # Generated Methods
/// - `cli_app() -> clap::Command` - Complete CLI application
/// - `cli_run(&self, matches: &ArgMatches)` - Execute matched command
#[cfg(feature = "cli")]
#[proc_macro_attribute]
pub fn cli(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as cli::CliArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match cli::expand_cli(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate MCP (Model Context Protocol) tools from an impl block.
///
/// # Basic Usage
///
/// ```ignore
/// use server_less::mcp;
///
/// #[mcp]
/// impl FileTools {
///     fn read_file(&self, path: String) -> String { /* ... */ }
/// }
/// ```
///
/// # With Namespace
///
/// ```ignore
/// #[mcp(namespace = "file")]
/// impl FileTools {
///     // Exposed as "file_read_file" tool
///     fn read_file(&self, path: String) -> String { /* ... */ }
/// }
/// ```
///
/// # Streaming Support
///
/// Methods returning `impl Stream<Item = T>` are automatically collected into arrays:
///
/// ```ignore
/// use futures::stream::{self, Stream};
///
/// #[mcp]
/// impl DataService {
///     // Returns JSON array: [0, 1, 2, 3, 4]
///     fn stream_numbers(&self, count: u32) -> impl Stream<Item = u32> + use<> {
///         stream::iter(0..count)
///     }
/// }
///
/// // Call with:
/// service.mcp_call_async("stream_numbers", json!({"count": 5})).await
/// // Returns: [0, 1, 2, 3, 4]
/// ```
///
/// **Note:** Streaming methods require `mcp_call_async`, not `mcp_call`.
///
/// # Generated Methods
/// - `mcp_tools() -> Vec<serde_json::Value>` - Tool definitions
/// - `mcp_call(&self, name, args) -> Result<Value, String>` - Execute tool (sync only)
/// - `mcp_call_async(&self, name, args).await` - Execute tool (supports async & streams)
#[cfg(feature = "mcp")]
#[proc_macro_attribute]
pub fn mcp(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as mcp::McpArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match mcp::expand_mcp(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate WebSocket JSON-RPC handlers from an impl block.
///
/// Methods are exposed as JSON-RPC methods over WebSocket connections.
/// Supports both sync and async methods.
///
/// # Basic Usage
///
/// ```ignore
/// use server_less::ws;
///
/// #[ws(path = "/ws")]
/// impl ChatService {
///     fn send_message(&self, room: String, content: String) -> Message {
///         // ...
///     }
/// }
/// ```
///
/// # With Async Methods
///
/// ```ignore
/// #[ws(path = "/ws")]
/// impl ChatService {
///     // Async methods work seamlessly
///     async fn send_message(&self, room: String, content: String) -> Message {
///         // Can await database, network calls, etc.
///     }
///
///     // Mix sync and async
///     fn get_rooms(&self) -> Vec<String> {
///         // Synchronous method
///     }
/// }
/// ```
///
/// # Error Handling
///
/// ```ignore
/// #[ws(path = "/ws")]
/// impl ChatService {
///     fn send_message(&self, room: String, content: String) -> Result<Message, ChatError> {
///         if room.is_empty() {
///             return Err(ChatError::InvalidRoom);
///         }
///         Ok(Message::new(room, content))
///     }
/// }
/// ```
///
/// # Client Usage
///
/// Clients send JSON-RPC 2.0 messages over WebSocket:
///
/// ```json
/// // Request
/// {
///   "jsonrpc": "2.0",
///   "method": "send_message",
///   "params": {"room": "general", "content": "Hello!"},
///   "id": 1
/// }
///
/// // Response
/// {
///   "jsonrpc": "2.0",
///   "result": {"id": 123, "room": "general", "content": "Hello!"},
///   "id": 1
/// }
/// ```
///
/// # Generated Methods
/// - `ws_router() -> axum::Router` - Router with WebSocket endpoint
/// - `ws_handle_message(msg) -> String` - Sync message handler
/// - `ws_handle_message_async(msg) -> String` - Async message handler
/// - `ws_methods() -> Vec<&'static str>` - List of available methods
#[cfg(feature = "ws")]
#[proc_macro_attribute]
pub fn ws(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ws::WsArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match ws::expand_ws(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate JSON-RPC 2.0 handlers over HTTP.
///
/// # Example
///
/// ```ignore
/// use server_less::jsonrpc;
///
/// struct Calculator;
///
/// #[jsonrpc]
/// impl Calculator {
///     /// Add two numbers
///     fn add(&self, a: i32, b: i32) -> i32 {
///         a + b
///     }
///
///     /// Multiply two numbers
///     fn multiply(&self, a: i32, b: i32) -> i32 {
///         a * b
///     }
/// }
///
/// // POST /rpc with {"jsonrpc": "2.0", "method": "add", "params": {"a": 1, "b": 2}, "id": 1}
/// // Returns: {"jsonrpc": "2.0", "result": 3, "id": 1}
/// ```
///
/// This generates:
/// - `Calculator::jsonrpc_router()` returning an axum Router
/// - `Calculator::jsonrpc_handle(request)` to handle JSON-RPC requests
/// - `Calculator::jsonrpc_methods()` listing available methods
///
/// Supports JSON-RPC 2.0 features:
/// - Named and positional parameters
/// - Batch requests (array of requests)
/// - Notifications (requests without id)
#[cfg(feature = "jsonrpc")]
#[proc_macro_attribute]
pub fn jsonrpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as jsonrpc::JsonRpcArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match jsonrpc::expand_jsonrpc(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate OpenRPC specification for JSON-RPC services.
///
/// OpenRPC is to JSON-RPC what OpenAPI is to REST APIs.
///
/// # Example
///
/// ```ignore
/// use server_less::openrpc;
///
/// struct Calculator;
///
/// #[openrpc(title = "Calculator API", version = "1.0.0")]
/// impl Calculator {
///     /// Add two numbers
///     fn add(&self, a: i32, b: i32) -> i32 { a + b }
/// }
///
/// // Get OpenRPC spec as JSON
/// let spec = Calculator::openrpc_spec();
/// let json = Calculator::openrpc_json();
///
/// // Write to file
/// Calculator::write_openrpc("openrpc.json")?;
/// ```
#[cfg(feature = "openrpc")]
#[proc_macro_attribute]
pub fn openrpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as openrpc::OpenRpcArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match openrpc::expand_openrpc(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate Markdown API documentation from an impl block.
///
/// Creates human-readable documentation that can be used with
/// any static site generator (VitePress, Docusaurus, MkDocs, etc.).
///
/// # Example
///
/// ```ignore
/// use server_less::markdown;
///
/// struct UserService;
///
/// #[markdown(title = "User API")]
/// impl UserService {
///     /// Create a new user
///     fn create_user(&self, name: String, email: String) -> User { ... }
///
///     /// Get user by ID
///     fn get_user(&self, id: String) -> Option<User> { ... }
/// }
///
/// // Get markdown string
/// let docs = UserService::markdown_docs();
///
/// // Write to file
/// UserService::write_markdown("docs/api.md")?;
/// ```
#[cfg(feature = "markdown")]
#[proc_macro_attribute]
pub fn markdown(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as markdown::MarkdownArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match markdown::expand_markdown(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate AsyncAPI specification for event-driven services.
///
/// AsyncAPI is to WebSockets/messaging what OpenAPI is to REST.
///
/// # Example
///
/// ```ignore
/// use server_less::asyncapi;
///
/// struct ChatService;
///
/// #[asyncapi(title = "Chat API", server = "ws://localhost:8080")]
/// impl ChatService {
///     /// Send a message to a room
///     fn send_message(&self, room: String, content: String) -> bool { true }
///
///     /// Get message history
///     fn get_history(&self, room: String, limit: Option<u32>) -> Vec<String> { vec![] }
/// }
///
/// // Get AsyncAPI spec
/// let spec = ChatService::asyncapi_spec();
/// let json = ChatService::asyncapi_json();
///
/// // Write to file
/// ChatService::write_asyncapi("asyncapi.json")?;
/// ```
#[cfg(feature = "asyncapi")]
#[proc_macro_attribute]
pub fn asyncapi(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as asyncapi::AsyncApiArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match asyncapi::expand_asyncapi(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate Connect protocol schema from an impl block.
///
/// Connect is a modern RPC protocol from Buf that works over HTTP/1.1, HTTP/2, and HTTP/3.
/// The generated schema is compatible with connect-go, connect-es, connect-swift, etc.
///
/// # Example
///
/// ```ignore
/// use server_less::connect;
///
/// struct UserService;
///
/// #[connect(package = "users.v1")]
/// impl UserService {
///     fn get_user(&self, id: String) -> User { ... }
/// }
///
/// // Get schema and endpoint paths
/// let schema = UserService::connect_schema();
/// let paths = UserService::connect_paths(); // ["/users.v1.UserService/GetUser", ...]
/// ```
#[cfg(feature = "connect")]
#[proc_macro_attribute]
pub fn connect(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as connect::ConnectArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match connect::expand_connect(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate Protocol Buffers schema from an impl block.
///
/// # Example
///
/// ```ignore
/// use server_less::grpc;
///
/// struct UserService;
///
/// #[grpc(package = "users")]
/// impl UserService {
///     /// Get user by ID
///     fn get_user(&self, id: String) -> User { ... }
///
///     /// Create a new user
///     fn create_user(&self, name: String, email: String) -> User { ... }
/// }
///
/// // Get the proto schema
/// let proto = UserService::proto_schema();
///
/// // Write to file for use with tonic-build
/// UserService::write_proto("proto/users.proto")?;
/// ```
///
/// The generated schema can be used with tonic-build in your build.rs
/// to generate the full gRPC client/server implementation.
#[cfg(feature = "grpc")]
#[proc_macro_attribute]
pub fn grpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as grpc::GrpcArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match grpc::expand_grpc(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate Cap'n Proto schema from an impl block.
///
/// # Example
///
/// ```ignore
/// use server_less::capnp;
///
/// struct UserService;
///
/// #[capnp(id = "0x85150b117366d14b")]
/// impl UserService {
///     /// Get user by ID
///     fn get_user(&self, id: String) -> String { ... }
///
///     /// Create a new user
///     fn create_user(&self, name: String, email: String) -> String { ... }
/// }
///
/// // Get the Cap'n Proto schema
/// let schema = UserService::capnp_schema();
///
/// // Write to file for use with capnpc
/// UserService::write_capnp("schema/users.capnp")?;
/// ```
///
/// The generated schema can be used with capnpc to generate
/// the full Cap'n Proto serialization code.
#[cfg(feature = "capnp")]
#[proc_macro_attribute]
pub fn capnp(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as capnp::CapnpArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match capnp::expand_capnp(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate Apache Thrift schema from an impl block.
///
/// # Example
///
/// ```ignore
/// use server_less::thrift;
///
/// struct UserService;
///
/// #[thrift(namespace = "users")]
/// impl UserService {
///     /// Get user by ID
///     fn get_user(&self, id: String) -> String { ... }
///
///     /// Create a new user
///     fn create_user(&self, name: String, email: String) -> String { ... }
/// }
///
/// // Get the Thrift schema
/// let schema = UserService::thrift_schema();
///
/// // Write to file for use with thrift compiler
/// UserService::write_thrift("idl/users.thrift")?;
/// ```
///
/// The generated schema can be used with the Thrift compiler to generate
/// client/server code in various languages.
#[cfg(feature = "thrift")]
#[proc_macro_attribute]
pub fn thrift(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as thrift::ThriftArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match thrift::expand_thrift(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate Smithy IDL schema from an impl block.
///
/// Smithy is AWS's open-source interface definition language for defining APIs.
/// The generated schema follows Smithy 2.0 specification.
///
/// # Example
///
/// ```ignore
/// use server_less::smithy;
///
/// struct UserService;
///
/// #[smithy(namespace = "com.example.users")]
/// impl UserService {
///     /// Get user by ID
///     fn get_user(&self, id: String) -> User { ... }
///
///     /// Create a new user
///     fn create_user(&self, name: String, email: String) -> User { ... }
/// }
///
/// // Get Smithy schema
/// let schema = UserService::smithy_schema();
/// // Write to file
/// UserService::write_smithy("service.smithy")?;
/// ```
///
/// The generated schema can be used with the Smithy toolchain for code generation.
#[cfg(feature = "smithy")]
#[proc_macro_attribute]
pub fn smithy(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as smithy::SmithyArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match smithy::expand_smithy(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate JSON Schema from an impl block.
///
/// Generates JSON Schema definitions for request/response types.
/// Useful for API validation, documentation, and tooling.
///
/// # Example
///
/// ```ignore
/// use server_less::jsonschema;
///
/// struct UserService;
///
/// #[jsonschema(title = "User API")]
/// impl UserService {
///     /// Get user by ID
///     fn get_user(&self, id: String) -> User { ... }
///
///     /// Create a new user
///     fn create_user(&self, name: String, email: String) -> User { ... }
/// }
///
/// // Get JSON Schema
/// let schema = UserService::json_schema();
/// // Write to file
/// UserService::write_json_schema("schema.json")?;
/// ```
#[cfg(feature = "jsonschema")]
#[proc_macro_attribute]
pub fn jsonschema(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as jsonschema::JsonSchemaArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match jsonschema::expand_jsonschema(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate GraphQL schema from an impl block using async-graphql.
///
/// Methods are automatically classified as Queries or Mutations based on naming:
/// - Queries: `get_*`, `list_*`, `find_*`, `search_*`, `fetch_*`, `query_*`
/// - Mutations: everything else (create, update, delete, etc.)
///
/// # Basic Usage
///
/// ```ignore
/// use server_less::graphql;
///
/// #[graphql]
/// impl UserService {
///     // Query: returns single user
///     async fn get_user(&self, id: String) -> Option<User> {
///         // ...
///     }
///
///     // Query: returns list of users
///     async fn list_users(&self) -> Vec<User> {
///         // ...
///     }
///
///     // Mutation: creates new user
///     async fn create_user(&self, name: String, email: String) -> User {
///         // ...
///     }
/// }
/// ```
///
/// # Type Mappings
///
/// - `String`, `i32`, `bool`, etc. → GraphQL scalars
/// - `Option<T>` → nullable GraphQL field
/// - `Vec<T>` → GraphQL list `[T]`
/// - Custom structs → GraphQL objects (must derive SimpleObject)
///
/// ```ignore
/// use async_graphql::SimpleObject;
///
/// #[derive(SimpleObject)]
/// struct User {
///     id: String,
///     name: String,
///     email: Option<String>,  // Nullable field
/// }
///
/// #[graphql]
/// impl UserService {
///     async fn get_user(&self, id: String) -> Option<User> {
///         // Returns User object with proper GraphQL schema
///     }
///
///     async fn list_users(&self) -> Vec<User> {
///         // Returns [User] in GraphQL
///     }
/// }
/// ```
///
/// # GraphQL Queries
///
/// ```graphql
/// # Query single user
/// query {
///   getUser(id: "123") {
///     id
///     name
///     email
///   }
/// }
///
/// # List all users
/// query {
///   listUsers {
///     id
///     name
///   }
/// }
///
/// # Mutation
/// mutation {
///   createUser(name: "Alice", email: "alice@example.com") {
///     id
///     name
///   }
/// }
/// ```
///
/// # Custom Scalars
///
/// Common custom scalar types are automatically supported:
///
/// ```ignore
/// use chrono::{DateTime, Utc};
/// use uuid::Uuid;
///
/// #[graphql]
/// impl EventService {
///     // UUID parameter
///     async fn get_event(&self, event_id: Uuid) -> Option<Event> { /* ... */ }
///
///     // DateTime parameter
///     async fn list_events(&self, since: DateTime<Utc>) -> Vec<Event> { /* ... */ }
///
///     // JSON parameter
///     async fn search_events(&self, filter: serde_json::Value) -> Vec<Event> { /* ... */ }
/// }
/// ```
///
/// Supported custom scalars:
/// - `chrono::DateTime<Utc>` → DateTime
/// - `uuid::Uuid` → UUID
/// - `url::Url` → Url
/// - `serde_json::Value` → JSON
///
/// # Generated Methods
/// - `graphql_schema() -> Schema` - async-graphql Schema
/// - `graphql_router() -> axum::Router` - Router with /graphql endpoint
/// - `graphql_sdl() -> String` - Schema Definition Language string
#[cfg(feature = "graphql")]
#[proc_macro_attribute]
pub fn graphql(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as graphql::GraphqlArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match graphql::expand_graphql(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Coordinate multiple protocol handlers into a single server.
///
/// # Example
///
/// ```ignore
/// use server_less::{http, ws, jsonrpc, serve};
///
/// struct MyService;
///
/// #[http]
/// #[ws]
/// #[jsonrpc]
/// #[serve(http, ws, jsonrpc)]
/// impl MyService {
///     fn list_items(&self) -> Vec<String> { vec![] }
/// }
///
/// // Now you can:
/// // - service.serve("0.0.0.0:3000").await  // start server
/// // - service.router()                     // get combined router
/// ```
///
/// # Arguments
///
/// - `http` - Include the HTTP router (REST API)
/// - `ws` - Include the WebSocket router (WS JSON-RPC)
/// - `jsonrpc` - Include the JSON-RPC HTTP router
/// - `graphql` - Include the GraphQL router
/// - `health = "/path"` - Custom health check path (default: `/health`)
#[cfg(feature = "http")]
#[proc_macro_attribute]
pub fn serve(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as http::ServeArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match http::expand_serve(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Helper attribute for method-level HTTP route customization.
///
/// This attribute is used within `#[http]` impl blocks to customize
/// individual method routing. It is a no-op on its own.
///
/// # Example
///
/// ```ignore
/// #[http(prefix = "/api")]
/// impl MyService {
///     #[route(method = "POST", path = "/custom")]
///     fn my_method(&self) { }
///
///     #[route(skip)]
///     fn internal_method(&self) { }
///
///     #[route(hidden)]  // Hidden from OpenAPI but still routed
///     fn secret(&self) { }
/// }
/// ```
#[cfg(feature = "http")]
#[proc_macro_attribute]
pub fn route(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass through unchanged - the #[http] macro parses these attributes
    item
}

/// Helper attribute for method-level HTTP response customization.
///
/// This attribute is used within `#[http]` impl blocks to customize
/// individual method responses. It is a no-op on its own.
///
/// # Supported Options
///
/// - `status = <code>` - Custom HTTP status code (e.g., 201, 204)
/// - `content_type = "<type>"` - Custom content type
/// - `header = "<name>", value = "<value>"` - Add custom response header
///
/// Multiple `#[response(...)]` attributes can be combined on a single method.
///
/// # Examples
///
/// ```ignore
/// #[http(prefix = "/api")]
/// impl MyService {
///     // Custom status code for creation
///     #[response(status = 201)]
///     fn create_item(&self, name: String) -> Item { /* ... */ }
///
///     // No content response
///     #[response(status = 204)]
///     fn delete_item(&self, id: String) { /* ... */ }
///
///     // Binary response with custom content type
///     #[response(content_type = "application/octet-stream")]
///     fn download(&self, id: String) -> Vec<u8> { /* ... */ }
///
///     // Add custom headers
///     #[response(header = "X-Custom", value = "foo")]
///     fn with_header(&self) -> String { /* ... */ }
///
///     // Combine multiple response attributes
///     #[response(status = 201)]
///     #[response(header = "Location", value = "/api/items/123")]
///     #[response(header = "X-Request-Id", value = "abc")]
///     fn create_with_headers(&self, name: String) -> Item { /* ... */ }
/// }
/// ```
#[cfg(feature = "http")]
#[proc_macro_attribute]
pub fn response(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass through unchanged - the #[http] macro parses these attributes
    item
}

/// Helper attribute for parameter-level HTTP customization.
///
/// This attribute is used on function parameters within `#[http]` impl blocks
/// to customize parameter extraction and naming. It is a no-op on its own.
///
/// **Note:** Requires nightly Rust with `#![feature(register_tool)]` and
/// `#![register_tool(param)]` at the crate root.
///
/// # Supported Options
///
/// - `name = "<wire_name>"` - Use a different name on the wire (e.g., `q` instead of `query`)
/// - `default = <value>` - Provide a default value for optional parameters
/// - `query` - Force parameter to come from query string
/// - `path` - Force parameter to come from URL path
/// - `body` - Force parameter to come from request body
/// - `header` - Extract parameter from HTTP header
///
/// # Location Inference
///
/// When no location is specified, parameters are inferred based on conventions:
/// - Parameters named `id` or ending in `_id` → path parameters
/// - POST/PUT/PATCH methods → body parameters
/// - GET/DELETE methods → query parameters
///
/// # Examples
///
/// ```ignore
/// #![feature(register_tool)]
/// #![register_tool(param)]
///
/// #[http(prefix = "/api")]
/// impl SearchService {
///     // Rename parameter: code uses `query`, API accepts `q`
///     fn search(&self, #[param(name = "q")] query: String) -> Vec<Result> {
///         /* ... */
///     }
///
///     // Default value for pagination
///     fn list_items(
///         &self,
///         #[param(default = 0)] offset: u32,
///         #[param(default = 10)] limit: u32,
///     ) -> Vec<Item> {
///         /* ... */
///     }
///
///     // Extract API key from header
///     fn protected_endpoint(
///         &self,
///         #[param(header, name = "X-API-Key")] api_key: String,
///         data: String,
///     ) -> String {
///         /* ... */
///     }
///
///     // Override location inference: force to query even though method is POST
///     fn search_posts(
///         &self,
///         #[param(query)] filter: String,
///         #[param(body)] content: String,
///     ) -> Vec<Post> {
///         /* ... */
///     }
///
///     // Combine multiple options
///     fn advanced(
///         &self,
///         #[param(query, name = "page", default = 1)] page_num: u32,
///     ) -> Vec<Item> {
///         /* ... */
///     }
/// }
/// ```
///
/// # OpenAPI Integration
///
/// - Parameters with `name` are documented with their wire names
/// - Parameters with `default` are marked as not required
/// - Location overrides are reflected in OpenAPI specs
#[cfg(feature = "http")]
#[proc_macro_attribute]
pub fn param(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass through unchanged - the #[http] macro parses these attributes
    item
}

/// Derive macro for error types that implement `IntoErrorCode`.
///
/// # Example
///
/// ```ignore
/// use server_less::ServerlessError;
///
/// #[derive(ServerlessError)]
/// enum MyError {
///     #[error(code = NotFound, message = "User not found")]
///     UserNotFound,
///     #[error(code = 400)]  // HTTP status also works
///     InvalidInput(String),
///     // Code inferred from variant name
///     Unauthorized,
/// }
/// ```
///
/// This generates:
/// - `impl IntoErrorCode for MyError`
/// - `impl Display for MyError`
/// - `impl Error for MyError`
///
/// # Attributes
///
/// - `#[error(code = X)]` - Set error code (ErrorCode variant or HTTP status)
/// - `#[error(message = "...")]` - Set custom message
///
/// Without attributes, the error code is inferred from the variant name.
#[proc_macro_derive(ServerlessError, attributes(error))]
pub fn serverless_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match error::expand_serverless_error(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
