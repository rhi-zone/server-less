//! Proc macros for trellis.
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
/// # Generated Methods
/// - `http_router() -> axum::Router` - Complete router with all endpoints
/// - `http_routes() -> Vec<&'static str>` - List of route paths
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
/// # Generated Methods
/// - `mcp_tools() -> Vec<serde_json::Value>` - Tool definitions
/// - `mcp_call(&self, name, args) -> Result<Value, String>` - Execute tool
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
/// # Example
///
/// ```ignore
/// use server_less::ws;
///
/// struct ChatService;
///
/// #[ws(path = "/ws")]
/// impl ChatService {
///     /// Send a message to a room
///     fn send_message(&self, room: String, content: String) -> Message {
///         // ...
///     }
///
///     /// Get recent messages
///     fn get_history(&self, room: String, limit: Option<u32>) -> Vec<Message> {
///         // ...
///     }
/// }
/// ```
///
/// This generates:
/// - `ChatService::ws_router()` returning an axum Router with WS endpoint
/// - `ChatService::ws_handle_message(msg)` to handle incoming messages
/// - `ChatService::ws_methods()` listing available methods
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
/// # Example
///
/// ```ignore
/// use server_less::graphql;
///
/// struct UserService;
///
/// #[graphql]
/// impl UserService {
///     /// Get user by ID
///     async fn get_user(&self, id: String) -> Option<User> { None }
///
///     /// Create a new user
///     async fn create_user(&self, name: String) -> User { ... }
/// }
///
/// // Generated:
/// // - UserServiceQuery with get_user resolver
/// // - UserServiceMutation with create_user resolver
/// // - service.graphql_schema() -> Schema
/// // - service.graphql_router() -> axum Router at /graphql
/// // - service.graphql_sdl() -> SDL string
/// ```
///
/// Methods starting with `get_`, `list_`, `find_`, etc. become Queries.
/// Other methods become Mutations.
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
/// # Example
///
/// ```ignore
/// #[http(prefix = "/api")]
/// impl MyService {
///     #[response(status = 201)]
///     fn create_item(&self, name: String) -> Item { /* ... */ }
///
///     #[response(status = 204)]
///     fn delete_item(&self, id: String) { /* ... */ }
///
///     #[response(content_type = "application/octet-stream")]
///     fn download(&self, id: String) -> Vec<u8> { /* ... */ }
///
///     #[response(header = "X-Custom", value = "foo")]
///     fn with_header(&self) -> String { /* ... */ }
/// }
/// ```
#[cfg(feature = "http")]
#[proc_macro_attribute]
pub fn response(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass through unchanged - the #[http] macro parses these attributes
    item
}

/// Derive macro for error types that implement `IntoErrorCode`.
///
/// # Example
///
/// ```ignore
/// use server_less::TrellisError;
///
/// #[derive(TrellisError)]
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
#[proc_macro_derive(TrellisError, attributes(error))]
pub fn trellis_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match error::expand_trellis_error(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
