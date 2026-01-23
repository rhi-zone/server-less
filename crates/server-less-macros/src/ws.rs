//! WebSocket handler generation.
//!
//! Generates JSON-RPC style WebSocket message handlers from impl blocks.
//!
//! # Protocol
//!
//! Uses JSON-RPC 2.0 message format over WebSocket:
//! - Request: `{"method": "echo", "params": {"message": "hello"}, "id": 1}`
//! - Response: `{"result": "Echo: hello", "id": 1}`
//! - Error: `{"error": {"message": "Unknown method"}, "id": 1}`
//!
//! # Message Handling
//!
//! - Methods are called by name via JSON messages
//! - Parameters extracted from `params` object
//! - Both sync and async methods supported
//! - Supports optional `id` field for request/response correlation
//!
//! # Generated Methods
//!
//! - `ws_methods() -> Vec<&'static str>` - List of available methods
//! - `ws_handle_message(&self, message: &str) -> Result<String, String>` - Sync handler
//! - `ws_handle_message_async(&self, message: &str).await` - Async handler
//! - `ws_router(self) -> axum::Router` - Complete WebSocket server
//!
//! # Example
//!
//! ```ignore
//! use server_less::ws;
//!
//! #[derive(Clone)]
//! struct ChatService;
//!
//! #[ws(path = "/ws")]
//! impl ChatService {
//!     /// Echo a message
//!     fn echo(&self, message: String) -> String {
//!         format!("Echo: {}", message)
//!     }
//!
//!     /// Add two numbers
//!     async fn add(&self, a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//! }
//!
//! // Use it:
//! let service = ChatService;
//! let app = service.ws_router();
//!
//! // Client sends:
//! // {"method": "echo", "params": {"message": "hello"}}
//! // Server responds:
//! // {"result": "Echo: hello"}
//! ```

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use server_less_parse::{MethodInfo, extract_methods, get_impl_name};
use server_less_rpc::{self, AsyncHandling};
use syn::{ItemImpl, Token, parse::Parse};

// Import Context helpers
use crate::context::{has_qualified_context, partition_context_params};

/// Arguments for the #[ws] attribute
#[derive(Default)]
pub(crate) struct WsArgs {
    /// WebSocket endpoint path (e.g., "/ws")
    pub path: Option<String>,
}

impl Parse for WsArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = WsArgs::default();

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

pub(crate) fn expand_ws(args: WsArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    // PASS 1: Scan for qualified server_less::Context usage
    let has_qualified = has_qualified_context(&methods);

    let path = args.path.unwrap_or_else(|| "/ws".to_string());

    // Generate dispatch match arms (sync and async versions)
    let dispatch_arms_sync: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm_sync(m, has_qualified))
        .collect::<syn::Result<Vec<_>>>()?;

    let dispatch_arms_async: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm_async(m, has_qualified))
        .collect::<syn::Result<Vec<_>>>()?;

    // Method names for documentation
    let method_names: Vec<_> = methods.iter().map(|m| m.name.to_string()).collect();

    // Check if any method uses Context
    let uses_context = methods.iter().any(|m| {
        partition_context_params(&m.params, has_qualified)
            .map(|(ctx, _)| ctx.is_some())
            .unwrap_or(false)
    });

    // Generate dispatch signatures and calls based on Context usage
    let (dispatch_sig_sync, dispatch_sig_async, dispatch_call_sync, dispatch_call_async) =
        if uses_context {
            (
                quote! {
                    fn ws_dispatch(
                        &self,
                        __ctx: ::server_less::Context,
                        method: &str,
                        args: ::server_less::serde_json::Value,
                    ) -> ::std::result::Result<::server_less::serde_json::Value, String>
                },
                quote! {
                    async fn ws_dispatch_async(
                        &self,
                        __ctx: ::server_less::Context,
                        method: &str,
                        args: ::server_less::serde_json::Value,
                    ) -> ::std::result::Result<::server_less::serde_json::Value, String>
                },
                quote! { self.ws_dispatch(__ctx, method, params) },
                quote! { self.ws_dispatch_async(__ctx, method, params).await },
            )
        } else {
            (
                quote! {
                    fn ws_dispatch(
                        &self,
                        method: &str,
                        args: ::server_less::serde_json::Value,
                    ) -> ::std::result::Result<::server_less::serde_json::Value, String>
                },
                quote! {
                    async fn ws_dispatch_async(
                        &self,
                        method: &str,
                        args: ::server_less::serde_json::Value,
                    ) -> ::std::result::Result<::server_less::serde_json::Value, String>
                },
                quote! { self.ws_dispatch(method, params) },
                quote! { self.ws_dispatch_async(method, params).await },
            )
        };

    // Generate message handler call based on Context usage
    let message_handler_call = if uses_context {
        quote! { state.ws_handle_message_async(__ctx.clone(), &text).await }
    } else {
        quote! { state.ws_handle_message_async(&text).await }
    };

    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_ws_handler_{}", struct_name_snake);
    let connection_fn_name = format_ident!("__trellis_ws_connection_{}", struct_name_snake);

    // Generate method signatures based on Context usage
    let (handle_sig_sync, handle_sig_async) = if uses_context {
        (
            quote! {
                pub fn ws_handle_message(
                    &self,
                    __ctx: ::server_less::Context,
                    message: &str,
                ) -> ::std::result::Result<String, String>
            },
            quote! {
                pub async fn ws_handle_message_async(
                    &self,
                    __ctx: ::server_less::Context,
                    message: &str,
                ) -> ::std::result::Result<String, String>
            },
        )
    } else {
        (
            quote! {
                pub fn ws_handle_message(
                    &self,
                    message: &str,
                ) -> ::std::result::Result<String, String>
            },
            quote! {
                pub async fn ws_handle_message_async(
                    &self,
                    message: &str,
                ) -> ::std::result::Result<String, String>
            },
        )
    };

    // Generate Context creation code if not provided
    let ctx_creation = if uses_context {
        quote! {}
    } else {
        quote! { let __ctx = ::server_less::Context::new(); }
    };

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get available WebSocket method names
            pub fn ws_methods() -> Vec<&'static str> {
                vec![#(#method_names),*]
            }

            /// Handle an incoming WebSocket JSON-RPC message (sync version)
            ///
            /// Note: Async methods will return an error. Use `ws_handle_message_async` for async methods.
            #handle_sig_sync {
                #ctx_creation
                // Parse the incoming message as JSON-RPC
                let parsed: ::server_less::serde_json::Value = ::server_less::serde_json::from_str(message)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                let method = parsed.get("method")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'method' field".to_string())?;

                let params = parsed.get("params")
                    .cloned()
                    .unwrap_or(::server_less::serde_json::json!({}));

                let id = parsed.get("id").cloned();

                // Dispatch to the appropriate method
                let result = #dispatch_call_sync;

                // Format response
                Self::__format_ws_response(result, id)
            }

            /// Handle an incoming WebSocket JSON-RPC message (async version)
            ///
            /// Supports both sync and async methods. Async methods are awaited properly.
            #handle_sig_async {
                #ctx_creation
                // Parse the incoming message as JSON-RPC
                let parsed: ::server_less::serde_json::Value = ::server_less::serde_json::from_str(message)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                let method = parsed.get("method")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'method' field".to_string())?;

                let params = parsed.get("params")
                    .cloned()
                    .unwrap_or(::server_less::serde_json::json!({}));

                let id = parsed.get("id").cloned();

                // Dispatch to the appropriate method (async)
                let result = #dispatch_call_async;

                // Format response
                Self::__format_ws_response(result, id)
            }

            /// Format a WebSocket JSON-RPC response
            fn __format_ws_response(
                result: ::std::result::Result<::server_less::serde_json::Value, String>,
                id: Option<::server_less::serde_json::Value>,
            ) -> ::std::result::Result<String, String> {
                let response = match result {
                    Ok(value) => {
                        let mut resp = ::server_less::serde_json::json!({
                            "result": value
                        });
                        if let Some(id) = id {
                            resp.as_object_mut().unwrap().insert("id".to_string(), id);
                        }
                        resp
                    }
                    Err(err) => {
                        let mut resp = ::server_less::serde_json::json!({
                            "error": {
                                "message": err
                            }
                        });
                        if let Some(id) = id {
                            resp.as_object_mut().unwrap().insert("id".to_string(), id);
                        }
                        resp
                    }
                };

                ::server_less::serde_json::to_string(&response)
                    .map_err(|e| format!("Serialization error: {}", e))
            }

            /// Dispatch a method call (sync version)
            #dispatch_sig_sync {
                match method {
                    #(#dispatch_arms_sync)*
                    _ => Err(format!("Unknown method: {}", method)),
                }
            }

            /// Dispatch a method call (async version)
            #dispatch_sig_async {
                match method {
                    #(#dispatch_arms_async)*
                    _ => Err(format!("Unknown method: {}", method)),
                }
            }

            /// Create an axum Router with WebSocket endpoint
            pub fn ws_router(self) -> ::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                let state = ::std::sync::Arc::new(self);
                ::axum::Router::new()
                    .route(#path, ::axum::routing::get(#handler_name))
                    .with_state(state)
            }
        }

        // WebSocket upgrade handler
        async fn #handler_name(
            ws: ::axum::extract::WebSocketUpgrade,
            state_extractor: ::axum::extract::State<::std::sync::Arc<#struct_name>>,
            __context_headers: ::axum::http::HeaderMap,
        ) -> impl ::axum::response::IntoResponse {
            let state = state_extractor.0;

            // Extract Context from HTTP upgrade headers
            let mut __ctx = ::server_less::Context::new();
            if #uses_context {
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
            }

            ws.on_upgrade(move |socket| async move {
                #connection_fn_name(socket, state, __ctx).await
            })
        }

        // Handle individual WebSocket connection
        async fn #connection_fn_name(
            socket: ::axum::extract::ws::WebSocket,
            state: ::std::sync::Arc<#struct_name>,
            __ctx: ::server_less::Context,
        ) {
            use ::futures::stream::StreamExt;
            use ::futures::sink::SinkExt;

            let (mut sender, mut receiver) = socket.split();

            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(::axum::extract::ws::Message::Text(text)) => {
                        // Use async handler to support async methods
                        let response = #message_handler_call;
                        let reply = match response {
                            Ok(json) => json,
                            Err(err) => ::server_less::serde_json::json!({
                                "error": {"message": err}
                            }).to_string(),
                        };
                        if sender.send(::axum::extract::ws::Message::Text(reply.into())).await.is_err() {
                            break;
                        }
                    }
                    Ok(::axum::extract::ws::Message::Close(_)) => break,
                    Ok(_) => {} // Ignore binary, ping, pong
                    Err(_) => break,
                }
            }
        }
    })
}

/// Generate a dispatch match arm for a method (sync version)
fn generate_dispatch_arm_sync(
    method: &MethodInfo,
    has_qualified: bool,
) -> syn::Result<TokenStream2> {
    generate_dispatch_arm_with_context(method, has_qualified, AsyncHandling::Error)
}

/// Generate a dispatch match arm for a method (async version)
fn generate_dispatch_arm_async(
    method: &MethodInfo,
    has_qualified: bool,
) -> syn::Result<TokenStream2> {
    generate_dispatch_arm_with_context(method, has_qualified, AsyncHandling::Await)
}

/// Generate dispatch arm with Context support
fn generate_dispatch_arm_with_context(
    method: &MethodInfo,
    has_qualified: bool,
    async_handling: AsyncHandling,
) -> syn::Result<TokenStream2> {
    let method_name_str = method.name.to_string();

    // Partition Context vs regular parameters
    let (context_param, regular_params) = partition_context_params(&method.params, has_qualified)?;

    // If no Context, use default RPC dispatch
    if context_param.is_none() {
        return Ok(server_less_rpc::generate_dispatch_arm(
            method,
            None,
            async_handling,
        ));
    }

    // Check if this will error out early (async method in sync context)
    let requires_async = method.is_async || method.return_info.is_stream;
    if requires_async && matches!(async_handling, AsyncHandling::Error) {
        // For async methods in sync context with Context params,
        // we need to extract params but then error immediately
        let param_extractions = server_less_rpc::generate_param_extractions_for(&regular_params);

        return Ok(quote! {
            #method_name_str => {
                #(#param_extractions)*
                return Err("Async methods and streaming methods not supported in sync context".to_string());
            }
        });
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

    let call = server_less_rpc::generate_method_call_with_args(method, arg_exprs, async_handling);
    let response = server_less_rpc::generate_json_response(method);

    Ok(quote! {
        #method_name_str => {
            #(#param_extractions)*
            #call
            #response
        }
    })
}
