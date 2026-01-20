//! Proc macros for trellis.
//!
//! This crate provides attribute macros that transform impl blocks into protocol handlers,
//! and derive macros for common patterns.

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemImpl};

mod cli;
mod error;
mod graphql;
mod grpc;
mod http;
mod mcp;
mod parse;
mod rpc;
mod serve;
mod ws;

/// Generate HTTP handlers from an impl block.
///
/// # Example
///
/// ```ignore
/// use trellis::http;
///
/// struct UserService;
///
/// #[http]
/// impl UserService {
///     /// Create a new user
///     async fn create_user(&self, name: String, email: String) -> User {
///         // ...
///     }
///
///     /// Get user by ID
///     async fn get_user(&self, id: UserId) -> Option<User> {
///         // ...
///     }
/// }
/// ```
///
/// This generates:
/// - `UserService::http_router()` returning an axum Router
/// - Individual handler functions for each method
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
/// # Example
///
/// ```ignore
/// use trellis::cli;
///
/// struct MyApp;
///
/// #[cli(name = "myapp")]
/// impl MyApp {
///     /// Create a new user
///     fn create_user(&self, name: String, email: String) {
///         // ...
///     }
///
///     /// Get user by ID
///     fn get_user(&self, id: String) {
///         // ...
///     }
/// }
/// ```
///
/// This generates:
/// - `MyApp::cli()` returning a clap Command
/// - `MyApp::run()` to execute the CLI
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
/// # Example
///
/// ```ignore
/// use trellis::mcp;
///
/// struct MyTools;
///
/// #[mcp]
/// impl MyTools {
///     /// Search for users by name
///     fn search_users(&self, query: String, limit: Option<u32>) -> Vec<User> {
///         // ...
///     }
/// }
/// ```
///
/// This generates:
/// - `MyTools::mcp_tools()` returning tool definitions
/// - `MyTools::mcp_call(name, args)` to dispatch tool calls
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
/// use trellis::ws;
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
#[proc_macro_attribute]
pub fn ws(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ws::WsArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match ws::expand_ws(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate Protocol Buffers schema from an impl block.
///
/// # Example
///
/// ```ignore
/// use trellis::grpc;
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
#[proc_macro_attribute]
pub fn grpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as grpc::GrpcArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match grpc::expand_grpc(args, impl_block) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate GraphQL schema from an impl block using async-graphql.
///
/// # Example
///
/// ```ignore
/// use trellis::graphql;
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
/// use trellis::{http, ws, serve};
///
/// struct MyService;
///
/// #[http]
/// #[ws]
/// #[serve(http, ws)]
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
/// - `http` - Include the HTTP router
/// - `ws` - Include the WebSocket router
/// - `health = "/path"` - Custom health check path (default: `/health`)
#[proc_macro_attribute]
pub fn serve(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as serve::ServeArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);

    match serve::expand_serve(args, impl_block) {
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
#[proc_macro_attribute]
pub fn route(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass through unchanged - the #[http] macro parses these attributes
    item
}

/// Derive macro for error types that implement `IntoErrorCode`.
///
/// # Example
///
/// ```ignore
/// use trellis::TrellisError;
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
