//! WebSocket handler generation.
//!
//! Generates JSON-RPC style message handlers over WebSocket.
//! Methods become callable via `{"method": "name", "params": {...}}` messages.


use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse::Parse, ItemImpl, Token};
use trellis_parse::{extract_methods, get_impl_name, MethodInfo};
use trellis_rpc::{self, AsyncHandling};

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

    let path = args.path.unwrap_or_else(|| "/ws".to_string());

    // Generate dispatch match arms (sync and async versions)
    let dispatch_arms_sync: Vec<_> = methods
        .iter()
        .map(generate_dispatch_arm_sync)
        .collect::<syn::Result<Vec<_>>>()?;

    let dispatch_arms_async: Vec<_> = methods
        .iter()
        .map(generate_dispatch_arm_async)
        .collect::<syn::Result<Vec<_>>>()?;

    // Method names for documentation
    let method_names: Vec<_> = methods.iter().map(|m| m.name.to_string()).collect();

    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_ws_handler_{}", struct_name_snake);
    let connection_fn_name = format_ident!("__trellis_ws_connection_{}", struct_name_snake);

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
            pub fn ws_handle_message(
                &self,
                message: &str,
            ) -> ::std::result::Result<String, String> {
                // Parse the incoming message as JSON-RPC
                let parsed: ::trellis::serde_json::Value = ::trellis::serde_json::from_str(message)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                let method = parsed.get("method")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'method' field".to_string())?;

                let params = parsed.get("params")
                    .cloned()
                    .unwrap_or(::trellis::serde_json::json!({}));

                let id = parsed.get("id").cloned();

                // Dispatch to the appropriate method
                let result = self.ws_dispatch(method, params);

                // Format response
                Self::__format_ws_response(result, id)
            }

            /// Handle an incoming WebSocket JSON-RPC message (async version)
            ///
            /// Supports both sync and async methods. Async methods are awaited properly.
            pub async fn ws_handle_message_async(
                &self,
                message: &str,
            ) -> ::std::result::Result<String, String> {
                // Parse the incoming message as JSON-RPC
                let parsed: ::trellis::serde_json::Value = ::trellis::serde_json::from_str(message)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                let method = parsed.get("method")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'method' field".to_string())?;

                let params = parsed.get("params")
                    .cloned()
                    .unwrap_or(::trellis::serde_json::json!({}));

                let id = parsed.get("id").cloned();

                // Dispatch to the appropriate method (async)
                let result = self.ws_dispatch_async(method, params).await;

                // Format response
                Self::__format_ws_response(result, id)
            }

            /// Format a WebSocket JSON-RPC response
            fn __format_ws_response(
                result: ::std::result::Result<::trellis::serde_json::Value, String>,
                id: Option<::trellis::serde_json::Value>,
            ) -> ::std::result::Result<String, String> {
                let response = match result {
                    Ok(value) => {
                        let mut resp = ::trellis::serde_json::json!({
                            "result": value
                        });
                        if let Some(id) = id {
                            resp.as_object_mut().unwrap().insert("id".to_string(), id);
                        }
                        resp
                    }
                    Err(err) => {
                        let mut resp = ::trellis::serde_json::json!({
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

                ::trellis::serde_json::to_string(&response)
                    .map_err(|e| format!("Serialization error: {}", e))
            }

            /// Dispatch a method call (sync version)
            fn ws_dispatch(
                &self,
                method: &str,
                args: ::trellis::serde_json::Value,
            ) -> ::std::result::Result<::trellis::serde_json::Value, String> {
                match method {
                    #(#dispatch_arms_sync)*
                    _ => Err(format!("Unknown method: {}", method)),
                }
            }

            /// Dispatch a method call (async version)
            async fn ws_dispatch_async(
                &self,
                method: &str,
                args: ::trellis::serde_json::Value,
            ) -> ::std::result::Result<::trellis::serde_json::Value, String> {
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
        ) -> impl ::axum::response::IntoResponse {
            let state = state_extractor.0;
            ws.on_upgrade(move |socket| async move {
                #connection_fn_name(socket, state).await
            })
        }

        // Handle individual WebSocket connection
        async fn #connection_fn_name(
            socket: ::axum::extract::ws::WebSocket,
            state: ::std::sync::Arc<#struct_name>,
        ) {
            use ::futures::stream::StreamExt;
            use ::futures::sink::SinkExt;

            let (mut sender, mut receiver) = socket.split();

            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(::axum::extract::ws::Message::Text(text)) => {
                        // Use async handler to support async methods
                        let response = state.ws_handle_message_async(&text).await;
                        let reply = match response {
                            Ok(json) => json,
                            Err(err) => ::trellis::serde_json::json!({
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
fn generate_dispatch_arm_sync(method: &MethodInfo) -> syn::Result<TokenStream2> {
    // Use shared RPC dispatch generation
    Ok(trellis_rpc::generate_dispatch_arm(method, None, AsyncHandling::Error))
}

/// Generate a dispatch match arm for a method (async version)
fn generate_dispatch_arm_async(method: &MethodInfo) -> syn::Result<TokenStream2> {
    // Use shared RPC dispatch generation with await support
    Ok(trellis_rpc::generate_dispatch_arm(method, None, AsyncHandling::Await))
}
