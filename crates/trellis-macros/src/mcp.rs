//! MCP (Model Context Protocol) tool generation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, ItemImpl, Token};

use crate::parse::{extract_methods, get_impl_name, MethodInfo};

/// Arguments for the #[mcp] attribute
#[derive(Default)]
pub struct McpArgs {
    /// Tool namespace/prefix
    pub namespace: Option<String>,
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
                        format!("unknown argument: {other}"),
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

/// Expand the #[mcp] attribute macro
pub fn expand_mcp(args: McpArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
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

    // Generate dispatch match arms
    let dispatch_arms: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm(&struct_name, &namespace_prefix, m))
        .collect::<syn::Result<Vec<_>>>()?;

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

            /// Call an MCP tool by name with JSON arguments
            pub fn mcp_call(
                &self,
                name: &str,
                args: ::trellis::serde_json::Value
            ) -> ::std::result::Result<::trellis::serde_json::Value, String> {
                match name {
                    #(#dispatch_arms)*
                    _ => Err(format!("Unknown tool: {}", name)),
                }
            }

            /// Call an MCP tool (async version)
            pub async fn mcp_call_async(
                &self,
                name: &str,
                args: ::trellis::serde_json::Value
            ) -> ::std::result::Result<::trellis::serde_json::Value, String> {
                // For now, just use the sync version
                // TODO: Support async methods properly
                self.mcp_call(name, args)
            }
        }
    })
}

/// Generate an MCP tool definition (JSON schema)
fn generate_tool_definition(namespace_prefix: &str, method: &MethodInfo) -> TokenStream {
    let name = format!("{}{}", namespace_prefix, method.name);
    let description = method
        .docs
        .clone()
        .unwrap_or_else(|| method.name.to_string());

    // Generate parameter schema
    let properties: Vec<_> = method
        .params
        .iter()
        .map(|p| {
            let param_name = p.name.to_string();
            let param_type = infer_json_type(&p.ty);
            let description = format!("Parameter: {}", param_name);

            quote! {
                (#param_name, #param_type, #description)
            }
        })
        .collect();

    let required_params: Vec<_> = method
        .params
        .iter()
        .filter(|p| !p.is_optional)
        .map(|p| p.name.to_string())
        .collect();

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

/// Generate a dispatch match arm for calling a method
fn generate_dispatch_arm(
    _struct_name: &syn::Ident,
    namespace_prefix: &str,
    method: &MethodInfo,
) -> syn::Result<TokenStream> {
    let tool_name = format!("{}{}", namespace_prefix, method.name);
    let method_name = &method.name;

    // Generate argument extraction from JSON
    let arg_extractions: Vec<_> = method
        .params
        .iter()
        .map(|p| {
            let name = &p.name;
            let name_str = p.name.to_string();
            let ty = &p.ty;

            if p.is_optional {
                // For Option<T>, we extract the inner T and wrap back in Option
                quote! {
                    let #name: #ty = args.get(#name_str)
                        .and_then(|v| if v.is_null() { None } else { ::trellis::serde_json::from_value(v.clone()).ok() });
                }
            } else {
                quote! {
                    let __val = args.get(#name_str)
                        .ok_or_else(|| format!("Missing required parameter: {}", #name_str))?
                        .clone();
                    let #name: #ty = ::trellis::serde_json::from_value::<#ty>(__val)
                        .map_err(|e| format!("Invalid parameter {}: {}", #name_str, e))?;
                }
            }
        })
        .collect();

    let arg_names: Vec<_> = method.params.iter().map(|p| &p.name).collect();

    // Generate the call and response
    let call = if method.is_async {
        // Can't call async from sync context easily
        quote! {
            return Err("Async methods not yet supported in sync MCP calls".to_string());
        }
    } else {
        quote! {
            let result = self.#method_name(#(#arg_names),*);
        }
    };

    // Generate response handling
    let response = if method.return_info.is_unit {
        quote! {
            Ok(::trellis::serde_json::json!({"success": true}))
        }
    } else if method.return_info.is_result {
        quote! {
            match result {
                Ok(value) => Ok(::trellis::serde_json::to_value(value)
                    .map_err(|e| format!("Serialization error: {}", e))?),
                Err(err) => Err(format!("{:?}", err)),
            }
        }
    } else if method.return_info.is_option {
        quote! {
            match result {
                Some(value) => Ok(::trellis::serde_json::to_value(value)
                    .map_err(|e| format!("Serialization error: {}", e))?),
                None => Ok(::trellis::serde_json::Value::Null),
            }
        }
    } else {
        quote! {
            Ok(::trellis::serde_json::to_value(result)
                .map_err(|e| format!("Serialization error: {}", e))?)
        }
    };

    Ok(quote! {
        #tool_name => {
            #(#arg_extractions)*
            #call
            #response
        }
    })
}

/// Infer JSON schema type from Rust type
fn infer_json_type(ty: &syn::Type) -> &'static str {
    let ty_str = quote!(#ty).to_string();

    if ty_str.contains("String") || ty_str.contains("str") {
        "string"
    } else if ty_str.contains("i8")
        || ty_str.contains("i16")
        || ty_str.contains("i32")
        || ty_str.contains("i64")
        || ty_str.contains("u8")
        || ty_str.contains("u16")
        || ty_str.contains("u32")
        || ty_str.contains("u64")
        || ty_str.contains("isize")
        || ty_str.contains("usize")
    {
        "integer"
    } else if ty_str.contains("f32") || ty_str.contains("f64") {
        "number"
    } else if ty_str.contains("bool") {
        "boolean"
    } else if ty_str.contains("Vec") || ty_str.contains("[]") {
        "array"
    } else {
        "object"
    }
}
