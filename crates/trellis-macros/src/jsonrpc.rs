//! JSON-RPC over HTTP handler generation.
//!
//! Generates JSON-RPC 2.0 handlers over HTTP POST.
//! Methods become callable via `{"jsonrpc": "2.0", "method": "name", "params": {...}, "id": 1}`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parse, ItemImpl, Token};

use crate::parse::{extract_methods, get_impl_name, MethodInfo};
use crate::rpc::{self, AsyncHandling};

/// Arguments for the #[jsonrpc] attribute
#[derive(Default)]
pub struct JsonRpcArgs {
    /// HTTP endpoint path (e.g., "/rpc")
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

/// Expand the #[jsonrpc] attribute macro
pub fn expand_jsonrpc(args: JsonRpcArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    let path = args.path.unwrap_or_else(|| "/rpc".to_string());

    // Generate dispatch match arms
    let dispatch_arms_async: Vec<_> = methods
        .iter()
        .map(|m| generate_dispatch_arm(m))
        .collect::<syn::Result<Vec<_>>>()?;

    // Method names for documentation
    let method_names: Vec<_> = methods.iter().map(|m| m.name.to_string()).collect();

    // Generate unique handler function name
    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_jsonrpc_handler_{}", struct_name_snake);

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get available JSON-RPC method names
            pub fn jsonrpc_methods() -> Vec<&'static str> {
                vec![#(#method_names),*]
            }

            /// Handle a JSON-RPC 2.0 request
            pub async fn jsonrpc_handle(
                &self,
                request: ::trellis::serde_json::Value,
            ) -> ::trellis::serde_json::Value {
                // Check if batch request
                if let Some(arr) = request.as_array() {
                    let mut responses = Vec::new();
                    for req in arr {
                        if let Some(resp) = self.jsonrpc_handle_single(req.clone()).await {
                            responses.push(resp);
                        }
                    }
                    if responses.is_empty() {
                        // All notifications, no response
                        ::trellis::serde_json::Value::Null
                    } else {
                        ::trellis::serde_json::Value::Array(responses)
                    }
                } else {
                    self.jsonrpc_handle_single(request).await
                        .unwrap_or(::trellis::serde_json::Value::Null)
                }
            }

            /// Handle a single JSON-RPC 2.0 request
            async fn jsonrpc_handle_single(
                &self,
                request: ::trellis::serde_json::Value,
            ) -> Option<::trellis::serde_json::Value> {
                let id = request.get("id").cloned();
                let is_notification = id.is_none();

                // Validate jsonrpc version
                let version = request.get("jsonrpc").and_then(|v| v.as_str());
                if version != Some("2.0") {
                    if is_notification {
                        return None;
                    }
                    return Some(Self::jsonrpc_error(-32600, "Invalid Request: missing jsonrpc 2.0", id));
                }

                // Get method
                let method = match request.get("method").and_then(|v| v.as_str()) {
                    Some(m) => m,
                    None => {
                        if is_notification {
                            return None;
                        }
                        return Some(Self::jsonrpc_error(-32600, "Invalid Request: missing method", id));
                    }
                };

                // Get params (default to empty object)
                let params = request.get("params")
                    .cloned()
                    .unwrap_or(::trellis::serde_json::json!({}));

                // Dispatch
                let result = self.jsonrpc_dispatch(method, params).await;

                // Notifications don't get responses
                if is_notification {
                    return None;
                }

                // Format response
                Some(match result {
                    Ok(value) => {
                        ::trellis::serde_json::json!({
                            "jsonrpc": "2.0",
                            "result": value,
                            "id": id
                        })
                    }
                    Err(err) => Self::jsonrpc_error(-32603, &err, id),
                })
            }

            /// Create a JSON-RPC 2.0 error response
            fn jsonrpc_error(
                code: i32,
                message: &str,
                id: Option<::trellis::serde_json::Value>,
            ) -> ::trellis::serde_json::Value {
                ::trellis::serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": code,
                        "message": message
                    },
                    "id": id
                })
            }

            /// Dispatch a method call
            async fn jsonrpc_dispatch(
                &self,
                method: &str,
                args: ::trellis::serde_json::Value,
            ) -> ::std::result::Result<::trellis::serde_json::Value, String> {
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

        // Handler function for this specific struct
        async fn #handler_name(
            ::axum::extract::State(state): ::axum::extract::State<::std::sync::Arc<#struct_name>>,
            ::axum::Json(request): ::axum::Json<::trellis::serde_json::Value>,
        ) -> impl ::axum::response::IntoResponse {
            use ::axum::response::IntoResponse;
            let response = state.jsonrpc_handle(request).await;
            if response.is_null() {
                // Notification - return 204 No Content
                ::axum::http::StatusCode::NO_CONTENT.into_response()
            } else {
                ::axum::Json(response).into_response()
            }
        }
    })
}

/// Generate a dispatch match arm for a method
fn generate_dispatch_arm(method: &MethodInfo) -> syn::Result<TokenStream> {
    Ok(rpc::generate_dispatch_arm(method, None, AsyncHandling::Await))
}
