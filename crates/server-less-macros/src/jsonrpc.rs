//! JSON-RPC over HTTP handler generation macro.
//!
//! Generates JSON-RPC 2.0 handlers over HTTP POST with full spec compliance.
//!
//! # JSON-RPC 2.0
//!
//! Implements the JSON-RPC 2.0 specification:
//! - Request: `{"jsonrpc": "2.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1}`
//! - Response: `{"jsonrpc": "2.0", "result": 8, "id": 1}`
//! - Error: `{"jsonrpc": "2.0", "error": {"code": -32601, "message": "Method not found"}, "id": 1}`
//! - Notification (no response): `{"jsonrpc": "2.0", "method": "log", "params": {"msg": "hello"}}`
//!
//! # Features
//!
//! - Single requests and batch requests
//! - Notifications (requests without `id`)
//! - Both sync and async methods
//! - Positional and named parameters
//!
//! # Generated Methods
//!
//! - `jsonrpc_methods() -> Vec<&'static str>` - List of available methods
//! - `jsonrpc_handle(&self, request: &str) -> String` - Handle request (sync)
//! - `jsonrpc_handle_async(&self, request: &str).await` - Handle request (async)
//! - `jsonrpc_router(self) -> axum::Router` - HTTP server at /rpc
//!
//! # Example
//!
//! ```ignore
//! use server_less::jsonrpc;
//!
//! #[derive(Clone)]
//! struct Calculator;
//!
//! #[jsonrpc(path = "/rpc")]
//! impl Calculator {
//!     /// Add two numbers
//!     fn add(&self, a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//!
//!     /// Subtract two numbers
//!     fn subtract(&self, a: i32, b: i32) -> i32 {
//!         a - b
//!     }
//! }
//!
//! // Use it:
//! let calc = Calculator;
//! let app = calc.jsonrpc_router();
//!
//! // Client POST to /rpc:
//! // {"jsonrpc": "2.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1}
//! // Response:
//! // {"jsonrpc": "2.0", "result": 8, "id": 1}
//! ```

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use server_less_parse::{MethodInfo, extract_methods, get_impl_name};
use server_less_rpc::{self, AsyncHandling};
use syn::{ItemImpl, Token, parse::Parse};

// Import Context helpers
use crate::context::{has_qualified_context, partition_context_params};

/// Arguments for the #[jsonrpc] attribute
#[derive(Default)]
pub(crate) struct JsonRpcArgs {
    pub path: Option<String>,
}

impl Parse for JsonRpcArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = JsonRpcArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "path" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.path = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: path"),
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

pub(crate) fn expand_jsonrpc(args: JsonRpcArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    // PASS 1: Scan for qualified server_less::Context usage
    let has_qualified = has_qualified_context(&methods);

    let path = args.path.unwrap_or_else(|| "/rpc".to_string());

    let dispatch_arms_async: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm(m, has_qualified))
        .collect::<syn::Result<Vec<_>>>()?;

    let method_names: Vec<_> = methods.iter().map(|m| m.name.to_string()).collect();

    // Check if any method uses Context
    let uses_context = methods.iter().any(|m| {
        partition_context_params(&m.params, has_qualified)
            .map(|(ctx, _)| ctx.is_some())
            .unwrap_or(false)
    });

    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_jsonrpc_handler_{}", struct_name_snake);

    // Generate dispatch signature and public API based on Context usage
    let (
        dispatch_sig,
        dispatch_call,
        handle_sig,
        handle_single_sig,
        handle_single_call_batch,
        handle_single_call,
        handler_call,
        ctx_creation,
    ) = if uses_context {
        (
            quote! {
                async fn jsonrpc_dispatch(
                    &self,
                    __ctx: ::server_less::Context,
                    method: &str,
                    args: ::server_less::serde_json::Value,
                ) -> ::std::result::Result<::server_less::serde_json::Value, String>
            },
            quote! { self.jsonrpc_dispatch(__ctx, method, params).await },
            quote! {
                pub async fn jsonrpc_handle(
                    &self,
                    __ctx: ::server_less::Context,
                    request: ::server_less::serde_json::Value,
                ) -> ::server_less::serde_json::Value
            },
            quote! {
                async fn jsonrpc_handle_single(
                    &self,
                    __ctx: ::server_less::Context,
                    request: ::server_less::serde_json::Value,
                ) -> Option<::server_less::serde_json::Value>
            },
            quote! { self.jsonrpc_handle_single(__ctx.clone(), req.clone()).await },
            quote! { self.jsonrpc_handle_single(__ctx, request).await },
            quote! { state.jsonrpc_handle(__ctx, request).await },
            quote! {},
        )
    } else {
        (
            quote! {
                async fn jsonrpc_dispatch(
                    &self,
                    method: &str,
                    args: ::server_less::serde_json::Value,
                ) -> ::std::result::Result<::server_less::serde_json::Value, String>
            },
            quote! { self.jsonrpc_dispatch(method, params).await },
            quote! {
                pub async fn jsonrpc_handle(
                    &self,
                    request: ::server_less::serde_json::Value,
                ) -> ::server_less::serde_json::Value
            },
            quote! {
                async fn jsonrpc_handle_single(
                    &self,
                    request: ::server_less::serde_json::Value,
                ) -> Option<::server_less::serde_json::Value>
            },
            quote! { self.jsonrpc_handle_single(req.clone()).await },
            quote! { self.jsonrpc_handle_single(request).await },
            quote! { state.jsonrpc_handle(request).await },
            quote! { let __ctx = ::server_less::Context::new(); },
        )
    };

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get available JSON-RPC method names
            pub fn jsonrpc_methods() -> Vec<&'static str> {
                vec![#(#method_names),*]
            }

            /// Handle a JSON-RPC 2.0 request
            #handle_sig {
                #ctx_creation
                if let Some(arr) = request.as_array() {
                    let mut responses = Vec::new();
                    for req in arr {
                        if let Some(resp) = #handle_single_call_batch {
                            responses.push(resp);
                        }
                    }
                    if responses.is_empty() {
                        ::server_less::serde_json::Value::Null
                    } else {
                        ::server_less::serde_json::Value::Array(responses)
                    }
                } else {
                    #handle_single_call
                        .unwrap_or(::server_less::serde_json::Value::Null)
                }
            }

            #handle_single_sig {
                let id = request.get("id").cloned();
                let is_notification = id.is_none();

                let version = request.get("jsonrpc").and_then(|v| v.as_str());
                if version != Some("2.0") {
                    if is_notification {
                        return None;
                    }
                    return Some(Self::jsonrpc_error(-32600, "Invalid Request: missing jsonrpc 2.0", id));
                }

                let method = match request.get("method").and_then(|v| v.as_str()) {
                    Some(m) => m,
                    None => {
                        if is_notification {
                            return None;
                        }
                        return Some(Self::jsonrpc_error(-32600, "Invalid Request: missing method", id));
                    }
                };

                let params = request.get("params")
                    .cloned()
                    .unwrap_or(::server_less::serde_json::json!({}));

                let result = #dispatch_call;

                if is_notification {
                    return None;
                }

                Some(match result {
                    Ok(value) => {
                        ::server_less::serde_json::json!({
                            "jsonrpc": "2.0",
                            "result": value,
                            "id": id
                        })
                    }
                    Err(err) => Self::jsonrpc_error(-32603, &err, id),
                })
            }

            fn jsonrpc_error(
                code: i32,
                message: &str,
                id: Option<::server_less::serde_json::Value>,
            ) -> ::server_less::serde_json::Value {
                ::server_less::serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": code,
                        "message": message
                    },
                    "id": id
                })
            }

            #dispatch_sig {
                match method {
                    #(#dispatch_arms_async)*
                    _ => Err(format!("Method not found: {}", method)),
                }
            }

            /// Create an axum Router with JSON-RPC endpoint
            pub fn jsonrpc_router(self) -> ::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                let state = ::std::sync::Arc::new(self);
                ::axum::Router::new()
                    .route(#path, ::axum::routing::post(#handler_name))
                    .with_state(state)
            }
        }

        async fn #handler_name(
            ::axum::extract::State(state): ::axum::extract::State<::std::sync::Arc<#struct_name>>,
            __context_headers: ::axum::http::HeaderMap,
            ::axum::Json(request): ::axum::Json<::server_less::serde_json::Value>,
        ) -> impl ::axum::response::IntoResponse {
            use ::axum::response::IntoResponse;

            // Extract Context from headers
            let mut __ctx = ::server_less::Context::new();
            for (name, value) in __context_headers.iter() {
                if let Ok(value_str) = value.to_str() {
                    __ctx.set(name.as_str(), value_str);
                }
            }
            if let Some(request_id) = __context_headers.get("x-request-id")
                .and_then(|v| v.to_str().ok())
            {
                __ctx.set_request_id(request_id);
            }

            let response = #handler_call;
            if response.is_null() {
                ::axum::http::StatusCode::NO_CONTENT.into_response()
            } else {
                ::axum::Json(response).into_response()
            }
        }
    })
}

fn generate_dispatch_arm(method: &MethodInfo, has_qualified: bool) -> syn::Result<TokenStream2> {
    let method_name_str = method.name.to_string();

    // Partition Context vs regular parameters
    let (context_param, regular_params) = partition_context_params(&method.params, has_qualified)?;

    // If no Context, use default RPC dispatch
    if context_param.is_none() {
        return Ok(server_less_rpc::generate_dispatch_arm(
            method,
            None,
            AsyncHandling::Await,
        ));
    }

    // Generate extractions only for regular params (Context is already in scope as __ctx)
    let param_extractions = server_less_rpc::generate_param_extractions_for(&regular_params);

    // Build argument list: Context first (if present), then regular params in order
    let mut arg_exprs = Vec::new();
    for param in &method.params {
        if crate::context::should_inject_context(&param.ty, has_qualified) {
            arg_exprs.push(quote! { __ctx.clone() });
        } else {
            let name = &param.name;
            arg_exprs.push(quote! { #name });
        }
    }

    let call =
        server_less_rpc::generate_method_call_with_args(method, arg_exprs, AsyncHandling::Await);
    let response = server_less_rpc::generate_json_response(method);

    Ok(quote! {
        #method_name_str => {
            #(#param_extractions)*
            #call
            #response
        }
    })
}
