//! MCP (Model Context Protocol) tool generation macro.
//!
//! Generates MCP tool definitions from Rust impl blocks for use with Claude and other LLMs.
//!
//! # What is MCP?
//!
//! Model Context Protocol (MCP) is a standard for exposing tools to language models.
//! Each method becomes a callable tool with JSON schema for parameters.
//!
//! # Tool Naming
//!
//! - Methods are exposed with their original names
//! - Optional namespace prefix: `#[mcp(namespace = "myapp")]` → `myapp_create_user`
//!
//! # Parameter Schema
//!
//! Parameters are automatically converted to JSON schema:
//! - `String` → string
//! - `i32`, `u64`, etc. → integer
//! - `f32`, `f64` → number
//! - `bool` → boolean
//! - `Option<T>` → optional parameter
//!
//! # Streaming Support
//!
//! Methods that return `impl Stream<Item = T>` are automatically supported.
//! Streams are collected into arrays before returning to the LLM.
//!
//! **Note:** Requires async context. Use `mcp_call_async` for streaming methods.
//!
//! ```ignore
//! use futures::stream::{self, Stream};
//!
//! #[mcp]
//! impl DataService {
//!     // Returns collected array: [1, 2, 3, 4, 5]
//!     fn stream_numbers(&self, count: u32) -> impl Stream<Item = u32> + use<> {
//!         stream::iter(0..count)
//!     }
//! }
//! ```
//!
//! # Generated Methods
//!
//! - `mcp_tools() -> Vec<serde_json::Value>` - Tool definitions for MCP
//! - `mcp_tool_names() -> Vec<&'static str>` - List of tool names
//! - `mcp_call(&self, name: &str, args: Value) -> Result<Value, String>` - Execute tool
//! - `mcp_call_async(&self, name: &str, args: Value).await` - Async execution
//!
//! # Example
//!
//! ```ignore
//! use server_less::mcp;
//!
//! struct FileTools;
//!
//! #[mcp(namespace = "file")]
//! impl FileTools {
//!     /// Read a file from the filesystem
//!     fn read_file(&self, path: String) -> Result<String, std::io::Error> {
//!         std::fs::read_to_string(path)
//!     }
//!
//!     /// Write content to a file
//!     fn write_file(&self, path: String, content: String) -> Result<(), std::io::Error> {
//!         std::fs::write(path, content)
//!     }
//! }
//!
//! // Use it:
//! let tools = FileTools;
//! let definitions = FileTools::mcp_tools();  // For MCP server
//! let result = tools.mcp_call("file_read_file", json!({"path": "/tmp/test.txt"}));
//! ```

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, extract_methods, get_impl_name};
use server_less_rpc::{self, AsyncHandling};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[mcp] attribute
#[derive(Default)]
pub(crate) struct McpArgs {
    /// Tool namespace/prefix
    namespace: Option<String>,
}

impl Parse for McpArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = McpArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "namespace" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.namespace = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: namespace"),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(args)
    }
}

/// Generate MCP tools from an impl block.
///
/// # Example
///
/// ```ignore
/// use trellis::mcp;
///
/// struct MyService;
///
/// #[mcp]
/// impl MyService {
///     /// Say hello
///     fn hello(&self, name: String) -> String {
///         format!("Hello, {}!", name)
///     }
/// }
///
/// // Generated methods:
/// // - MyService::mcp_tools() -> Vec<serde_json::Value>
/// // - MyService::mcp_call(&self, name, args) -> Result<Value, String>
/// // - MyService::mcp_call_async(&self, name, args).await -> Result<Value, String>
/// ```
pub(crate) fn expand_mcp(args: McpArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    let namespace = args.namespace.unwrap_or_default();
    let namespace_prefix = if namespace.is_empty() {
        String::new()
    } else {
        format!("{}_", namespace)
    };

    // Generate tool definitions
    let tool_definitions: Vec<_> = methods
        .iter()
        .map(|m| generate_tool_definition(&namespace_prefix, m))
        .collect();

    // Generate dispatch match arms (sync and async versions)
    let dispatch_arms_sync: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm_sync(&namespace_prefix, m))
        .collect();

    let dispatch_arms_async: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm_async(&namespace_prefix, m))
        .collect();

    // Tool names for the list
    let tool_names: Vec<_> = methods
        .iter()
        .map(|m| format!("{}{}", namespace_prefix, m.name))
        .collect();

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get the list of available MCP tool definitions
            pub fn mcp_tools() -> Vec<::server_less::serde_json::Value> {
                vec![
                    #(#tool_definitions),*
                ]
            }

            /// Get tool names
            pub fn mcp_tool_names() -> Vec<&'static str> {
                vec![#(#tool_names),*]
            }

            /// Call an MCP tool by name with JSON arguments (sync version)
            ///
            /// Note: Async methods will return an error. Use `mcp_call_async` for async methods.
            pub fn mcp_call(
                &self,
                name: &str,
                args: ::server_less::serde_json::Value
            ) -> ::std::result::Result<::server_less::serde_json::Value, String> {
                match name {
                    #(#dispatch_arms_sync)*
                    _ => Err(format!("Unknown tool: {}", name)),
                }
            }

            /// Call an MCP tool (async version)
            ///
            /// Supports both sync and async methods. Async methods are awaited properly.
            pub async fn mcp_call_async(
                &self,
                name: &str,
                args: ::server_less::serde_json::Value
            ) -> ::std::result::Result<::server_less::serde_json::Value, String> {
                match name {
                    #(#dispatch_arms_async)*
                    _ => Err(format!("Unknown tool: {}", name)),
                }
            }
        }
    })
}

/// Generate an MCP tool definition (JSON schema)
fn generate_tool_definition(namespace_prefix: &str, method: &MethodInfo) -> TokenStream2 {
    let name = format!("{}{}", namespace_prefix, method.name);
    let description = method
        .docs
        .clone()
        .unwrap_or_else(|| method.name.to_string());

    // Generate parameter schema using shared utility
    let (properties, required_params) = server_less_rpc::generate_param_schema(&method.params);

    quote! {
        {
            let mut properties = ::server_less::serde_json::Map::new();
            #(
                {
                    let (name, type_str, desc): (&str, &str, &str) = #properties;
                    properties.insert(name.to_string(), ::server_less::serde_json::json!({
                        "type": type_str,
                        "description": desc
                    }));
                }
            )*

            ::server_less::serde_json::json!({
                "name": #name,
                "description": #description,
                "inputSchema": {
                    "type": "object",
                    "properties": properties,
                    "required": [#(#required_params),*]
                }
            })
        }
    }
}

/// Generate a dispatch match arm for calling a method (sync version)
fn generate_dispatch_arm_sync(namespace_prefix: &str, method: &MethodInfo) -> TokenStream2 {
    let tool_name = format!("{}{}", namespace_prefix, method.name);
    server_less_rpc::generate_dispatch_arm(method, Some(&tool_name), AsyncHandling::Error)
}

/// Generate a dispatch match arm for calling a method (async version)
fn generate_dispatch_arm_async(namespace_prefix: &str, method: &MethodInfo) -> TokenStream2 {
    let tool_name = format!("{}{}", namespace_prefix, method.name);
    server_less_rpc::generate_dispatch_arm(method, Some(&tool_name), AsyncHandling::Await)
}
