//! Proc macros for trellis.
//!
//! This crate provides attribute macros that transform impl blocks into protocol handlers.

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemImpl};

mod http;
mod cli;
mod mcp;
mod parse;

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
