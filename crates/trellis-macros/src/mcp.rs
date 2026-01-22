//! MCP (Model Context Protocol) tool generation macro.
//!
//! This crate provides the `#[mcp]` attribute macro for generating
//! MCP tool definitions from Rust impl blocks.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Token, parse::Parse};
use trellis_parse::{MethodInfo, extract_methods, get_impl_name};
use trellis_rpc::{self, AsyncHandling};

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
            pub fn mcp_tools() -> Vec<::trellis::serde_json::Value> {
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
                args: ::trellis::serde_json::Value
            ) -> ::std::result::Result<::trellis::serde_json::Value, String> {
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
                args: ::trellis::serde_json::Value
            ) -> ::std::result::Result<::trellis::serde_json::Value, String> {
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
    let (properties, required_params) = trellis_rpc::generate_param_schema(&method.params);

    quote! {
        {
            let mut properties = ::trellis::serde_json::Map::new();
            #(
                {
                    let (name, type_str, desc): (&str, &str, &str) = #properties;
                    properties.insert(name.to_string(), ::trellis::serde_json::json!({
                        "type": type_str,
                        "description": desc
                    }));
                }
            )*

            ::trellis::serde_json::json!({
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
    trellis_rpc::generate_dispatch_arm(method, Some(&tool_name), AsyncHandling::Error)
}

/// Generate a dispatch match arm for calling a method (async version)
fn generate_dispatch_arm_async(namespace_prefix: &str, method: &MethodInfo) -> TokenStream2 {
    let tool_name = format!("{}{}", namespace_prefix, method.name);
    trellis_rpc::generate_dispatch_arm(method, Some(&tool_name), AsyncHandling::Await)
}
