//! Shared utilities for RPC-style macros (MCP, WebSocket).
//!
//! Both MCP and WebSocket use JSON-RPC-like dispatch:
//! - Receive `{"method": "name", "params": {...}}`
//! - Extract params from JSON
//! - Call the method
//! - Serialize result back to JSON

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::{MethodInfo, ParamInfo};

/// Generate code to extract a parameter from a `serde_json::Value` args object.
pub fn generate_param_extraction(param: &ParamInfo) -> TokenStream {
    let name = &param.name;
    let name_str = param.name.to_string();
    let ty = &param.ty;

    if param.is_optional {
        // For Option<T>, extract inner value, return None if missing/null
        quote! {
            let #name: #ty = args.get(#name_str)
                .and_then(|v| if v.is_null() { None } else {
                    ::trellis::serde_json::from_value(v.clone()).ok()
                });
        }
    } else {
        // Required parameter - error if missing
        quote! {
            let __val = args.get(#name_str)
                .ok_or_else(|| format!("Missing required parameter: {}", #name_str))?
                .clone();
            let #name: #ty = ::trellis::serde_json::from_value::<#ty>(__val)
                .map_err(|e| format!("Invalid parameter {}: {}", #name_str, e))?;
        }
    }
}

/// Generate all param extractions for a method.
pub fn generate_all_param_extractions(method: &MethodInfo) -> Vec<TokenStream> {
    method.params.iter().map(generate_param_extraction).collect()
}

/// Generate the method call expression.
///
/// Returns tokens for calling `self.method_name(arg1, arg2, ...)`.
/// For async methods, returns an error (caller should handle async context).
pub fn generate_method_call(method: &MethodInfo, handle_async: AsyncHandling) -> TokenStream {
    let method_name = &method.name;
    let arg_names: Vec<_> = method.params.iter().map(|p| &p.name).collect();

    match (method.is_async, handle_async) {
        (true, AsyncHandling::Error) => {
            quote! {
                return Err("Async methods not supported in sync context".to_string());
            }
        }
        (true, AsyncHandling::Await) => {
            quote! {
                let result = self.#method_name(#(#arg_names),*).await;
            }
        }
        (true, AsyncHandling::BlockOn) => {
            quote! {
                let result = ::tokio::runtime::Runtime::new()
                    .expect("Failed to create Tokio runtime")
                    .block_on(self.#method_name(#(#arg_names),*));
            }
        }
        (false, _) => {
            quote! {
                let result = self.#method_name(#(#arg_names),*);
            }
        }
    }
}

/// How to handle async methods.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Await and BlockOn reserved for future async support
pub enum AsyncHandling {
    /// Return an error if method is async
    Error,
    /// Await the method (caller must be async)
    Await,
    /// Use tokio::runtime::Runtime::block_on
    BlockOn,
}

/// Generate response handling that converts the method result to JSON.
///
/// Handles:
/// - `()` → `{"success": true}`
/// - `Result<T, E>` → `Ok(T)` or `Err(message)`
/// - `Option<T>` → `T` or `null`
/// - `T` → serialized T
pub fn generate_json_response(method: &MethodInfo) -> TokenStream {
    let ret = &method.return_info;

    if ret.is_unit {
        quote! {
            Ok(::trellis::serde_json::json!({"success": true}))
        }
    } else if ret.is_result {
        quote! {
            match result {
                Ok(value) => Ok(::trellis::serde_json::to_value(value)
                    .map_err(|e| format!("Serialization error: {}", e))?),
                Err(err) => Err(format!("{:?}", err)),
            }
        }
    } else if ret.is_option {
        quote! {
            match result {
                Some(value) => Ok(::trellis::serde_json::to_value(value)
                    .map_err(|e| format!("Serialization error: {}", e))?),
                None => Ok(::trellis::serde_json::Value::Null),
            }
        }
    } else {
        // Plain T
        quote! {
            Ok(::trellis::serde_json::to_value(result)
                .map_err(|e| format!("Serialization error: {}", e))?)
        }
    }
}

/// Generate a complete dispatch match arm for an RPC method.
///
/// Combines param extraction, method call, and response handling.
pub fn generate_dispatch_arm(
    method: &MethodInfo,
    method_name_override: Option<&str>,
    async_handling: AsyncHandling,
) -> TokenStream {
    let method_name_str = method_name_override
        .map(String::from)
        .unwrap_or_else(|| method.name.to_string());

    // For async methods with Error handling, return early without generating unreachable code
    if method.is_async && matches!(async_handling, AsyncHandling::Error) {
        return quote! {
            #method_name_str => {
                return Err("Async methods not supported in sync context".to_string());
            }
        };
    }

    let param_extractions = generate_all_param_extractions(method);
    let call = generate_method_call(method, async_handling);
    let response = generate_json_response(method);

    quote! {
        #method_name_str => {
            #(#param_extractions)*
            #call
            #response
        }
    }
}

/// Infer JSON schema type from Rust type.
pub fn infer_json_type(ty: &syn::Type) -> &'static str {
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

/// Generate JSON schema properties for method parameters.
pub fn generate_param_schema(params: &[ParamInfo]) -> (Vec<TokenStream>, Vec<String>) {
    let properties: Vec<_> = params
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

    let required: Vec<_> = params
        .iter()
        .filter(|p| !p.is_optional)
        .map(|p| p.name.to_string())
        .collect();

    (properties, required)
}
