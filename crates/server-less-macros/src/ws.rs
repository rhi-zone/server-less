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
//! # Basic Example
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
//!
//! # Server Push with WsSender
//!
//! Methods can receive a `WsSender` parameter to send messages independently
//! of the request/response cycle, enabling true bidirectional communication:
//!
//! ```ignore
//! use server_less::{ws, WsSender};
//! use std::collections::HashMap;
//! use std::sync::{Arc, Mutex};
//! use serde_json::json;
//!
//! #[derive(Clone)]
//! struct ChatRoom {
//!     users: Arc<Mutex<HashMap<String, Vec<WsSender>>>>,
//! }
//!
//! #[ws(path = "/chat")]
//! impl ChatRoom {
//!     /// Join a chat room
//!     async fn join(&self, sender: WsSender, room: String, username: String) -> String {
//!         // Store the sender for server push
//!         let mut users = self.users.lock().unwrap();
//!         users.entry(room.clone()).or_default().push(sender.clone());
//!
//!         // Broadcast join notification to all users in room
//!         for s in users.get(&room).unwrap() {
//!             s.send_json(&json!({
//!                 "type": "user_joined",
//!                 "username": username
//!             })).await.ok();
//!         }
//!
//!         format!("Joined room: {}", room)
//!     }
//!
//!     /// Send a message to all users in a room
//!     async fn send_message(&self, room: String, username: String, message: String) -> String {
//!         let users = self.users.lock().unwrap();
//!         if let Some(senders) = users.get(&room) {
//!             for sender in senders {
//!                 sender.send_json(&json!({
//!                     "type": "message",
//!                     "username": username,
//!                     "message": message
//!                 })).await.ok();
//!             }
//!         }
//!         "Message sent".to_string()
//!     }
//!
//!     /// Background task example: periodic updates
//!     async fn subscribe_updates(&self, sender: WsSender) -> String {
//!         // Clone sender for background task
//!         let sender_clone = sender.clone();
//!         tokio::spawn(async move {
//!             let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
//!             loop {
//!                 interval.tick().await;
//!                 if sender_clone.send_json(&json!({
//!                     "type": "heartbeat",
//!                     "timestamp": chrono::Utc::now().to_rfc3339()
//!                 })).await.is_err() {
//!                     break; // Connection closed
//!                 }
//!             }
//!         });
//!         "Subscribed to updates".to_string()
//!     }
//! }
//! ```
//!
//! # Combining Context and WsSender
//!
//! Methods can request both Context and WsSender for full access to request metadata
//! and bidirectional communication:
//!
//! ```ignore
//! use server_less::{ws, Context, WsSender};
//!
//! #[ws(path = "/api")]
//! impl ApiService {
//!     async fn authenticated_subscribe(
//!         &self,
//!         ctx: Context,
//!         sender: WsSender,
//!         topic: String,
//!     ) -> Result<String, String> {
//!         // Verify authentication from headers
//!         let user_id = ctx.header("x-user-id")
//!             .ok_or("Unauthorized")?;
//!
//!         // Subscribe with authentication context
//!         self.subscribe_user(user_id, topic.clone(), sender).await;
//!
//!         Ok(format!("Subscribed to {}", topic))
//!     }
//! }
//! ```

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use server_less_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};
use server_less_rpc::{self, AsyncHandling};
use syn::{ItemImpl, Token, parse::Parse};

// Import Context helpers
use crate::context::has_qualified_context;

/// Check if a type is server_less::WsSender (fully qualified)
fn is_qualified_ws_sender(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        let path = &type_path.path;
        let segments: Vec<_> = path.segments.iter().collect();

        if segments.len() >= 2 {
            // Look for server_less::WsSender pattern
            for i in 0..segments.len() - 1 {
                if segments[i].ident == "server_less" && segments[i + 1].ident == "WsSender" {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a type is bare `WsSender` (unqualified)
fn is_bare_ws_sender(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && type_path.path.segments.len() == 1
    {
        return type_path.path.segments[0].ident == "WsSender";
    }
    false
}

/// Check if this type should be treated as server_less::WsSender for injection
fn should_inject_ws_sender(ty: &syn::Type, has_qualified: bool) -> bool {
    if is_qualified_ws_sender(ty) {
        true
    } else if is_bare_ws_sender(ty) {
        // Only inject bare WsSender if no qualified version exists in the impl block
        !has_qualified
    } else {
        false
    }
}

/// Scan all methods to detect if any use qualified server_less::WsSender
fn has_qualified_ws_sender(methods: &[MethodInfo]) -> bool {
    methods.iter().any(|method| {
        method
            .params
            .iter()
            .any(|param| is_qualified_ws_sender(&param.ty))
    })
}

/// Partition parameters into Context, WsSender, and regular groups
///
/// Returns `(context_param, ws_sender_param, other_params)` where:
/// - `context_param` is `Some(param)` if a Context parameter was found
/// - `ws_sender_param` is `Some(param)` if a WsSender parameter was found
/// - `other_params` contains all regular parameters
///
/// Returns an error if multiple Context or WsSender parameters are found.
fn partition_ws_params(
    params: &[ParamInfo],
    has_qualified_ctx: bool,
    has_qualified_sender: bool,
) -> syn::Result<(Option<&ParamInfo>, Option<&ParamInfo>, Vec<&ParamInfo>)> {
    let mut context_param: Option<&ParamInfo> = None;
    let mut sender_param: Option<&ParamInfo> = None;
    let mut other_params = Vec::new();

    for param in params {
        if crate::context::should_inject_context(&param.ty, has_qualified_ctx) {
            if context_param.is_some() {
                return Err(syn::Error::new_spanned(
                    &param.ty,
                    "only one Context parameter allowed per method\n\
                     \n\
                     Hint: server_less::Context is automatically injected from request metadata.\n\
                     Remove the duplicate Context parameter.",
                ));
            }
            context_param = Some(param);
        } else if should_inject_ws_sender(&param.ty, has_qualified_sender) {
            if sender_param.is_some() {
                return Err(syn::Error::new_spanned(
                    &param.ty,
                    "only one WsSender parameter allowed per method\n\
                     \n\
                     Hint: server_less::WsSender is automatically injected for each WebSocket connection.\n\
                     Remove the duplicate WsSender parameter.",
                ));
            }
            sender_param = Some(param);
        } else {
            other_params.push(param);
        }
    }

    Ok((context_param, sender_param, other_params))
}

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

    // PASS 1: Scan for qualified server_less::Context and server_less::WsSender usage
    let has_qualified_ctx = has_qualified_context(&methods);
    let has_qualified_sender = has_qualified_ws_sender(&methods);

    let path = args.path.unwrap_or_else(|| "/ws".to_string());

    // Generate dispatch match arms (sync and async versions)
    let dispatch_arms_sync: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm_sync(m, has_qualified_ctx, has_qualified_sender))
        .collect::<syn::Result<Vec<_>>>()?;

    let dispatch_arms_async: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm_async(m, has_qualified_ctx, has_qualified_sender))
        .collect::<syn::Result<Vec<_>>>()?;

    // Method names for documentation
    let method_names: Vec<_> = methods.iter().map(|m| m.name.to_string()).collect();

    // Check if any method uses Context or WsSender
    let uses_injected_params = methods.iter().any(|m| {
        partition_ws_params(&m.params, has_qualified_ctx, has_qualified_sender)
            .map(|(ctx, sender, _)| ctx.is_some() || sender.is_some())
            .unwrap_or(false)
    });

    // Generate dispatch signatures and calls based on Context and WsSender usage
    let (dispatch_sig_sync, dispatch_sig_async, dispatch_call_sync, dispatch_call_async) =
        if uses_injected_params {
            (
                quote! {
                    fn ws_dispatch(
                        &self,
                        __ctx: ::server_less::Context,
                        __sender: ::server_less::WsSender,
                        method: &str,
                        args: ::server_less::serde_json::Value,
                    ) -> ::std::result::Result<::server_less::serde_json::Value, String>
                },
                quote! {
                    async fn ws_dispatch_async(
                        &self,
                        __ctx: ::server_less::Context,
                        __sender: ::server_less::WsSender,
                        method: &str,
                        args: ::server_less::serde_json::Value,
                    ) -> ::std::result::Result<::server_less::serde_json::Value, String>
                },
                quote! { self.ws_dispatch(__ctx, __sender, method, params) },
                quote! { self.ws_dispatch_async(__ctx, __sender, method, params).await },
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

    // Generate message handler call based on injected params usage
    let message_handler_call = if uses_injected_params {
        quote! { state.ws_handle_message_async(__ctx.clone(), __sender.clone(), &text).await }
    } else {
        quote! { state.ws_handle_message_async(&text).await }
    };

    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_ws_handler_{}", struct_name_snake);
    let connection_fn_name = format_ident!("__trellis_ws_connection_{}", struct_name_snake);

    // Generate method signatures based on injected params usage
    let (handle_sig_sync, handle_sig_async) = if uses_injected_params {
        (
            quote! {
                pub fn ws_handle_message(
                    &self,
                    __ctx: ::server_less::Context,
                    __sender: ::server_less::WsSender,
                    message: &str,
                ) -> ::std::result::Result<String, String>
            },
            quote! {
                pub async fn ws_handle_message_async(
                    &self,
                    __ctx: ::server_less::Context,
                    __sender: ::server_less::WsSender,
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

    // Generate Context creation code if not injected
    let ctx_creation = if uses_injected_params {
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

            /// Get OpenAPI paths for this WebSocket service (for composition with OpenApiBuilder)
            ///
            /// Returns a single GET endpoint for the WebSocket upgrade.
            pub fn ws_openapi_paths() -> ::std::vec::Vec<::server_less::OpenApiPath> {
                let methods: Vec<&str> = vec![#(#method_names),*];
                let methods_desc = methods.join(", ");

                vec![
                    ::server_less::OpenApiPath {
                        path: #path.to_string(),
                        method: "get".to_string(),
                        operation: ::server_less::OpenApiOperation {
                            summary: Some(format!("WebSocket endpoint (methods: {})", methods_desc)),
                            description: None,
                            operation_id: Some("websocket".to_string()),
                            tags: vec!["websocket".to_string()],
                            deprecated: false,
                            parameters: vec![],
                            request_body: None,
                            responses: {
                                let mut r = ::server_less::serde_json::Map::new();
                                r.insert("101".to_string(), ::server_less::serde_json::json!({
                                    "description": "Switching Protocols - WebSocket upgrade successful"
                                }));
                                r
                            },
                            extra: {
                                let mut e = ::server_less::serde_json::Map::new();
                                e.insert("x-websocket-protocol".to_string(), ::server_less::serde_json::json!({
                                    "format": "JSON-RPC style",
                                    "methods": methods,
                                    "request_example": {
                                        "method": "echo",
                                        "params": {"message": "hello"},
                                        "id": 1
                                    },
                                    "response_example": {
                                        "result": "Echo: hello",
                                        "id": 1
                                    }
                                }));
                                e
                            },
                        },
                    }
                ]
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
            if #uses_injected_params {
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

            let (sender, mut receiver) = socket.split();

            // Wrap sender in WsSender for sharing with methods
            let __sender = ::server_less::WsSender::new(sender);

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
                        // Send response using the sender through WsSender
                        if __sender.send(reply).await.is_err() {
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
    has_qualified_ctx: bool,
    has_qualified_sender: bool,
) -> syn::Result<TokenStream2> {
    generate_dispatch_arm_with_injected_params(
        method,
        has_qualified_ctx,
        has_qualified_sender,
        AsyncHandling::Error,
    )
}

/// Generate a dispatch match arm for a method (async version)
fn generate_dispatch_arm_async(
    method: &MethodInfo,
    has_qualified_ctx: bool,
    has_qualified_sender: bool,
) -> syn::Result<TokenStream2> {
    generate_dispatch_arm_with_injected_params(
        method,
        has_qualified_ctx,
        has_qualified_sender,
        AsyncHandling::Await,
    )
}

/// Generate dispatch arm with Context and WsSender support
fn generate_dispatch_arm_with_injected_params(
    method: &MethodInfo,
    has_qualified_ctx: bool,
    has_qualified_sender: bool,
    async_handling: AsyncHandling,
) -> syn::Result<TokenStream2> {
    let method_name_str = method.name.to_string();

    // Partition Context, WsSender, and regular parameters
    let (context_param, sender_param, regular_params) =
        partition_ws_params(&method.params, has_qualified_ctx, has_qualified_sender)?;

    // If no injected params, use default RPC dispatch
    if context_param.is_none() && sender_param.is_none() {
        return Ok(server_less_rpc::generate_dispatch_arm(
            method,
            None,
            async_handling,
        ));
    }

    // Check if this will error out early (async method in sync context)
    let requires_async = method.is_async || method.return_info.is_stream;
    if requires_async && matches!(async_handling, AsyncHandling::Error) {
        // For async methods in sync context with injected params,
        // we need to extract params but then error immediately
        let param_extractions = server_less_rpc::generate_param_extractions_for(&regular_params);

        return Ok(quote! {
            #method_name_str => {
                #(#param_extractions)*
                return Err("Async methods and streaming methods not supported in sync context".to_string());
            }
        });
    }

    // Generate extractions only for regular params (Context and WsSender already in scope)
    let param_extractions = server_less_rpc::generate_param_extractions_for(&regular_params);

    // Build argument list: injected params first, then regular params in their original order
    let mut arg_exprs = Vec::new();
    for param in &method.params {
        if crate::context::should_inject_context(&param.ty, has_qualified_ctx) {
            arg_exprs.push(quote! { __ctx.clone() });
        } else if should_inject_ws_sender(&param.ty, has_qualified_sender) {
            arg_exprs.push(quote! { __sender.clone() });
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
