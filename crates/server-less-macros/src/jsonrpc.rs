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

use crate::server_attrs::{has_server_hidden, has_server_skip};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use server_less_parse::{MethodInfo, extract_methods, get_impl_name, partition_methods};
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
    let (impl_generics, _ty_generics, where_clause) = impl_block.generics.split_for_impl();
    let self_ty = &impl_block.self_ty;
    let methods = extract_methods(&impl_block)?;

    // PASS 1: Scan for qualified server_less::Context usage
    let has_qualified = has_qualified_context(&methods);

    let path = args.path.unwrap_or_else(|| "/rpc".to_string());

    let partitioned = partition_methods(&methods, has_server_skip);

    // Separate hidden from visible leaf methods.
    // Hidden methods are still dispatchable but absent from method listings.
    let visible_leaf: Vec<_> = partitioned
        .leaf
        .iter()
        .copied()
        .filter(|m| !has_server_hidden(m))
        .collect();

    let dispatch_arms_async: Vec<_> = partitioned
        .leaf
        .iter()
        .map(|m| generate_dispatch_arm(m, has_qualified))
        .collect::<syn::Result<Vec<_>>>()?;

    let dispatch_arms_sync: Vec<_> = partitioned
        .leaf
        .iter()
        .map(|m| generate_sync_dispatch_arm(m, has_qualified))
        .collect::<syn::Result<Vec<_>>>()?;

    // method_names for jsonrpc_methods() and OpenRPC listing: visible only
    let method_names: Vec<_> = visible_leaf
        .iter()
        .map(|m| m.name.to_string())
        .collect();

    // Build method documentation (visible methods only)
    let jsonrpc_method_doc_entries: Vec<String> = visible_leaf
        .iter()
        .map(|m| {
            let name = m.name.to_string();
            match &m.docs {
                Some(doc) => format!("- `{name}` — {doc}"),
                None => format!("- `{name}`"),
            }
        })
        .collect();
    let has_jsonrpc_mounts =
        !partitioned.static_mounts.is_empty() || !partitioned.slug_mounts.is_empty();
    let jsonrpc_methods_doc = if jsonrpc_method_doc_entries.is_empty() && !has_jsonrpc_mounts {
        "Get available JSON-RPC method names.".to_string()
    } else {
        let mount_note = if has_jsonrpc_mounts {
            "\n\nAlso includes methods from mounted sub-services."
        } else {
            ""
        };
        format!(
            "Get available JSON-RPC method names.\n\n# Methods\n\n{}{}",
            jsonrpc_method_doc_entries.join("\n"),
            mount_note
        )
    };
    let jsonrpc_router_doc = format!(
        "Create an axum Router with JSON-RPC endpoint at `{}`.\n\n\
         Exposes {} method{}.",
        path,
        method_names.len(),
        if method_names.len() == 1 { "" } else { "s" }
    );

    // Generate mount dispatch arms and method names
    let mount_dispatch_arms: Vec<_> = partitioned
        .static_mounts
        .iter()
        .map(|m| generate_static_mount_dispatch(m))
        .chain(
            partitioned
                .slug_mounts
                .iter()
                .map(|m| generate_slug_mount_dispatch(m)),
        )
        .collect::<syn::Result<Vec<_>>>()?;

    let mount_dispatch_arms_sync: Vec<_> = partitioned
        .static_mounts
        .iter()
        .map(|m| generate_static_mount_dispatch_sync(m))
        .chain(
            partitioned
                .slug_mounts
                .iter()
                .map(|m| generate_slug_mount_dispatch_sync(m)),
        )
        .collect();

    let mount_method_names: Vec<_> = partitioned
        .static_mounts
        .iter()
        .chain(partitioned.slug_mounts.iter())
        .map(|m| generate_mount_method_names(m))
        .collect::<syn::Result<Vec<_>>>()?;

    // Check if any leaf method uses Context
    let uses_context = partitioned.leaf.iter().any(|m| {
        partition_context_params(&m.params, has_qualified)
            .map(|(ctx, _)| ctx.is_some())
            .unwrap_or(false)
    });

    // Mount dispatch inner method — always takes (method, args) without Context.
    // Maps the (i32, String) error from jsonrpc_dispatch down to a plain String
    // for the JsonRpcMount::jsonrpc_mount_dispatch_async interface.
    let mount_dispatch_inner = if uses_context {
        quote! {
            async fn jsonrpc_mount_dispatch_inner(
                &self,
                method: &str,
                args: ::server_less::serde_json::Value,
            ) -> ::std::result::Result<::server_less::serde_json::Value, String> {
                let __ctx = ::server_less::Context::new();
                self.jsonrpc_dispatch(__ctx, method, args).await.map_err(|(_, msg)| msg)
            }
        }
    } else {
        quote! {
            async fn jsonrpc_mount_dispatch_inner(
                &self,
                method: &str,
                args: ::server_less::serde_json::Value,
            ) -> ::std::result::Result<::server_less::serde_json::Value, String> {
                self.jsonrpc_dispatch(method, args).await.map_err(|(_, msg)| msg)
            }
        }
    };

    // Sync mount dispatch inner — returns Err for async-only methods
    let mount_dispatch_sync_inner = quote! {
        /// Internal sync dispatch for mount trait (no Context, returns Err for async-only methods).
        fn jsonrpc_mount_dispatch_sync_inner(
            &self,
            method: &str,
            args: ::server_less::serde_json::Value,
        ) -> ::std::result::Result<::server_less::serde_json::Value, String> {
            match method {
                #(#dispatch_arms_sync)*
                #(#mount_dispatch_arms_sync)*
                _ => Err(format!("Method not found: {}", method)),
            }
        }
    };

    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__server_less_jsonrpc_handler_{}", struct_name_snake);

    // Generate dispatch signature and public API based on Context usage.
    // The private jsonrpc_dispatch returns Result<Value, (i32, String)> where
    // the i32 is the JSON-RPC error code, enabling per-error code propagation.
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
                ) -> ::std::result::Result<::server_less::serde_json::Value, (i32, String)>
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
                ) -> ::std::result::Result<::server_less::serde_json::Value, (i32, String)>
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

        impl #impl_generics ::server_less::JsonRpcMount for #self_ty #where_clause {
            fn jsonrpc_mount_methods() -> Vec<String> {
                Self::jsonrpc_methods()
            }

            fn jsonrpc_mount_dispatch(
                &self,
                method: &str,
                params: ::server_less::serde_json::Value,
            ) -> ::std::result::Result<::server_less::serde_json::Value, String> {
                self.jsonrpc_mount_dispatch_sync_inner(method, params)
            }

            async fn jsonrpc_mount_dispatch_async(
                &self,
                method: &str,
                params: ::server_less::serde_json::Value,
            ) -> ::std::result::Result<::server_less::serde_json::Value, String> {
                self.jsonrpc_mount_dispatch_inner(method, params).await
            }
        }

        impl #impl_generics #self_ty #where_clause {
            #[doc = #jsonrpc_methods_doc]
            pub fn jsonrpc_methods() -> Vec<String> {
                let mut names: Vec<String> = vec![#(#method_names.to_string()),*];
                #(#mount_method_names)*
                names
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
                    Err((code, err)) => Self::jsonrpc_error(code, &err, id),
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
                    #(#mount_dispatch_arms)*
                    _ => Err((-32601i32, format!("Method not found: {}", method))),
                }
            }

            #mount_dispatch_sync_inner

            #mount_dispatch_inner

            #[doc = #jsonrpc_router_doc]
            pub fn jsonrpc_router(self) -> ::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                let state = ::std::sync::Arc::new(self);
                ::axum::Router::new()
                    .route(#path, ::axum::routing::post(#handler_name))
                    .with_state(state)
            }

            /// Get OpenAPI paths for this JSON-RPC service (for composition with OpenApiBuilder)
            ///
            /// Returns a single POST endpoint for the JSON-RPC interface.
            pub fn jsonrpc_openapi_paths() -> ::std::vec::Vec<::server_less::OpenApiPath> {
                let methods: Vec<&str> = vec![#(#method_names),*];
                let methods_desc = methods.join(", ");

                vec![
                    ::server_less::OpenApiPath {
                        path: #path.to_string(),
                        method: "post".to_string(),
                        operation: ::server_less::OpenApiOperation {
                            summary: Some(format!("JSON-RPC 2.0 endpoint (methods: {})", methods_desc)),
                            description: None,
                            operation_id: Some("jsonrpc".to_string()),
                            tags: vec!["jsonrpc".to_string()],
                            deprecated: false,
                            parameters: vec![],
                            request_body: Some(::server_less::serde_json::json!({
                                "required": true,
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "required": ["jsonrpc", "method"],
                                            "properties": {
                                                "jsonrpc": {
                                                    "type": "string",
                                                    "enum": ["2.0"]
                                                },
                                                "method": {
                                                    "type": "string",
                                                    "enum": methods
                                                },
                                                "params": {
                                                    "type": "object"
                                                },
                                                "id": {
                                                    "oneOf": [
                                                        {"type": "string"},
                                                        {"type": "integer"},
                                                        {"type": "null"}
                                                    ]
                                                }
                                            }
                                        }
                                    }
                                }
                            })),
                            responses: {
                                let mut r = ::server_less::serde_json::Map::new();
                                r.insert("200".to_string(), ::server_less::serde_json::json!({
                                    "description": "JSON-RPC response",
                                    "content": {
                                        "application/json": {
                                            "schema": {
                                                "type": "object",
                                                "properties": {
                                                    "jsonrpc": {"type": "string"},
                                                    "result": {},
                                                    "error": {
                                                        "type": "object",
                                                        "properties": {
                                                            "code": {"type": "integer"},
                                                            "message": {"type": "string"}
                                                        }
                                                    },
                                                    "id": {}
                                                }
                                            }
                                        }
                                    }
                                }));
                                r.insert("204".to_string(), ::server_less::serde_json::json!({
                                    "description": "Notification (no response)"
                                }));
                                r
                            },
                            extra: ::server_less::serde_json::Map::new(),
                        },
                    }
                ]
            }
        }

        async fn #handler_name(
            ::axum::extract::State(state): ::axum::extract::State<::std::sync::Arc<#self_ty>>,
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

/// Generate response handling for the private `jsonrpc_dispatch` method.
///
/// Unlike `server_less_rpc::generate_json_response`, this produces
/// `Result<Value, (i32, String)>` so that the JSON-RPC error code is preserved.
/// For `Result<T, E: IntoErrorCode>` returns, the code is taken from
/// `IntoErrorCode::jsonrpc_code()`. For other returns, `-32603` (internal error)
/// is used as the fallback.
fn generate_jsonrpc_json_response(method: &MethodInfo) -> TokenStream2 {
    let ret = &method.return_info;

    if ret.is_unit {
        quote! {
            Ok(::server_less::serde_json::json!({"success": true}))
        }
    } else if ret.is_stream {
        quote! {
            {
                use ::server_less::futures::StreamExt;
                let collected: Vec<_> = result.collect().await;
                Ok(::server_less::serde_json::to_value(collected)
                    .map_err(|e| (-32603i32, format!("Serialization error: {}", e)))
                    .map_err(|e| e)?)
            }
        }
    } else if ret.is_iterator {
        // Collect iterator into Vec before serializing (Iterator doesn't implement Serialize)
        quote! {
            {
                let __collected: Vec<_> = result.collect();
                ::server_less::serde_json::to_value(&__collected)
                    .map(Ok)
                    .map_err(|e| Err((-32603i32, format!("Serialization error: {}", e))))
                    .unwrap_or_else(|e| e)
            }
        }
    } else if ret.is_result {
        quote! {
            match result {
                Ok(value) => ::server_less::serde_json::to_value(value)
                    .map(Ok)
                    .map_err(|e| Err((-32603i32, format!("Serialization error: {}", e))))
                    .unwrap_or_else(|e| e),
                Err(err) => {
                    let __code = ::server_less::IntoErrorCode::jsonrpc_code(&err);
                    let __msg = ::server_less::IntoErrorCode::message(&err);
                    Err((__code, __msg))
                }
            }
        }
    } else if ret.is_option {
        quote! {
            match result {
                Some(value) => ::server_less::serde_json::to_value(value)
                    .map(Ok)
                    .map_err(|e| Err((-32603i32, format!("Serialization error: {}", e))))
                    .unwrap_or_else(|e| e),
                None => Ok(::server_less::serde_json::Value::Null),
            }
        }
    } else {
        quote! {
            ::server_less::serde_json::to_value(result)
                .map(Ok)
                .map_err(|e| Err((-32603i32, format!("Serialization error: {}", e))))
                .unwrap_or_else(|e| e)
        }
    }
}

/// Generate jsonrpc-specific param extraction that produces `(i32, String)` errors.
///
/// Like `server_less_rpc::generate_param_extraction` but maps errors to `(i32, String)`
/// suitable for use in `jsonrpc_dispatch` which returns `Result<Value, (i32, String)>`.
fn generate_jsonrpc_param_extraction(param: &server_less_parse::ParamInfo) -> TokenStream2 {
    let name = &param.name;
    let name_str = param.name.to_string();
    let ty = &param.ty;

    if param.is_optional {
        quote! {
            let #name: #ty = args.get(#name_str)
                .and_then(|v| if v.is_null() { None } else {
                    ::server_less::serde_json::from_value(v.clone()).ok()
                });
        }
    } else {
        quote! {
            let __val = args.get(#name_str)
                .ok_or_else(|| (-32602i32, format!("Missing required parameter: {}", #name_str)))?
                .clone();
            let #name: #ty = ::server_less::serde_json::from_value::<#ty>(__val)
                .map_err(|e| (-32602i32, format!("Invalid parameter {}: {}", #name_str, e)))?;
        }
    }
}

/// Generate a sync dispatch arm for the mount trait's sync inner dispatch.
///
/// Returns `Err` for async-only methods, mirroring the WsMount sync pattern.
fn generate_sync_dispatch_arm(
    method: &MethodInfo,
    has_qualified: bool,
) -> syn::Result<TokenStream2> {
    let method_name_str = method.name.to_string();

    // Partition Context vs regular parameters
    let (context_param, regular_params) = partition_context_params(&method.params, has_qualified)?;

    // If no Context, use default RPC dispatch with AsyncHandling::Error
    if context_param.is_none() {
        return Ok(server_less_rpc::generate_dispatch_arm(
            method,
            None,
            AsyncHandling::Error,
        ));
    }

    // For Context methods: extract regular params but inject a fresh Context
    let param_extractions = server_less_rpc::generate_param_extractions_for(&regular_params);

    let mut arg_exprs = Vec::new();
    for param in &method.params {
        if crate::context::should_inject_context(&param.ty, has_qualified) {
            arg_exprs.push(quote! { ::server_less::Context::new() });
        } else {
            let name = &param.name;
            arg_exprs.push(quote! { #name });
        }
    }

    let call =
        server_less_rpc::generate_method_call_with_args(method, arg_exprs, AsyncHandling::Error);
    let response = server_less_rpc::generate_json_response(method);

    Ok(quote! {
        #method_name_str => {
            #(#param_extractions)*
            #call
            #response
        }
    })
}

/// Generate an async dispatch arm for the private `jsonrpc_dispatch` method.
///
/// Returns `Result<Value, (i32, String)>` arms so that JSON-RPC error codes
/// are propagated from `IntoErrorCode` implementations.
fn generate_dispatch_arm(method: &MethodInfo, has_qualified: bool) -> syn::Result<TokenStream2> {
    let method_name_str = method.name.to_string();

    // Partition Context vs regular parameters
    let (context_param, regular_params) = partition_context_params(&method.params, has_qualified)?;

    let response = generate_jsonrpc_json_response(method);

    if context_param.is_none() {
        // No Context injection: generate jsonrpc-specific param extractions and call directly
        let param_extractions: Vec<_> = method
            .params
            .iter()
            .map(generate_jsonrpc_param_extraction)
            .collect();
        let call = server_less_rpc::generate_method_call(method, AsyncHandling::Await);
        return Ok(quote! {
            #method_name_str => {
                #(#param_extractions)*
                #call
                #response
            }
        });
    }

    // Generate extractions only for regular params (Context is already in scope as __ctx)
    let param_extractions: Vec<_> = regular_params
        .iter()
        .map(|p| generate_jsonrpc_param_extraction(p))
        .collect();

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

    Ok(quote! {
        #method_name_str => {
            #(#param_extractions)*
            #call
            #response
        }
    })
}

/// Generate mount method names contribution for jsonrpc_methods().
fn generate_mount_method_names(method: &MethodInfo) -> syn::Result<TokenStream2> {
    let mount_name = method.name.to_string();
    let mount_prefix = format!("{}.", mount_name);
    let inner_ty = method.return_info.reference_inner.as_ref().ok_or_else(|| {
        syn::Error::new_spanned(
            &method.method.sig,
            "BUG: mount method must have a reference return type (&T)",
        )
    })?;

    Ok(quote! {
        {
            let child_methods = <#inner_ty as ::server_less::JsonRpcMount>::jsonrpc_mount_methods();
            for child_name in child_methods {
                let prefixed = format!("{}{}", #mount_prefix, child_name);
                names.push(prefixed);
            }
        }
    })
}

/// Generate dispatch for a static mount (`fn foo(&self) -> &T`) — async version.
///
/// Maps the `Result<Value, String>` from `jsonrpc_mount_dispatch_async` into
/// `Result<Value, (i32, String)>` to match the private `jsonrpc_dispatch` return type.
fn generate_static_mount_dispatch(method: &MethodInfo) -> syn::Result<TokenStream2> {
    let mount_name = method.name.to_string();
    let mount_prefix = format!("{}.", mount_name);
    let method_name = &method.name;
    let inner_ty = method.return_info.reference_inner.as_ref().ok_or_else(|| {
        syn::Error::new_spanned(
            &method.method.sig,
            "BUG: mount method must have a reference return type (&T)",
        )
    })?;

    Ok(quote! {
        __method if __method.starts_with(#mount_prefix) => {
            let __stripped = &__method[#mount_prefix.len()..];
            let __delegate = self.#method_name();
            <#inner_ty as ::server_less::JsonRpcMount>::jsonrpc_mount_dispatch_async(__delegate, __stripped, args).await
                .map_err(|msg| (-32603i32, msg))
        }
    })
}

/// Generate dispatch for a static mount (`fn foo(&self) -> &T`) — sync version.
fn generate_static_mount_dispatch_sync(method: &MethodInfo) -> TokenStream2 {
    let mount_name = method.name.to_string();
    let mount_prefix = format!("{}.", mount_name);
    let method_name = &method.name;
    let inner_ty = method.return_info.reference_inner.as_ref().unwrap();

    quote! {
        __method if __method.starts_with(#mount_prefix) => {
            let __stripped = &__method[#mount_prefix.len()..];
            let __delegate = self.#method_name();
            <#inner_ty as ::server_less::JsonRpcMount>::jsonrpc_mount_dispatch(__delegate, __stripped, args)
        }
    }
}

/// Generate dispatch for a slug mount (`fn foo(&self, id: Id) -> &T`) — async version.
///
/// Maps the `Result<Value, String>` from `jsonrpc_mount_dispatch_async` into
/// `Result<Value, (i32, String)>` to match the private `jsonrpc_dispatch` return type.
fn generate_slug_mount_dispatch(method: &MethodInfo) -> syn::Result<TokenStream2> {
    let mount_name = method.name.to_string();
    let mount_prefix = format!("{}.", mount_name);
    let method_name = &method.name;
    let inner_ty = method.return_info.reference_inner.as_ref().ok_or_else(|| {
        syn::Error::new_spanned(
            &method.method.sig,
            "BUG: mount method must have a reference return type (&T)",
        )
    })?;

    // Use jsonrpc-specific param extraction so errors produce (i32, String)
    let slug_extractions: Vec<_> = method
        .params
        .iter()
        .map(generate_jsonrpc_param_extraction)
        .collect();
    let slug_names: Vec<_> = method.params.iter().map(|p| &p.name).collect();

    Ok(quote! {
        __method if __method.starts_with(#mount_prefix) => {
            let __stripped = &__method[#mount_prefix.len()..];
            #(#slug_extractions)*
            let __delegate = self.#method_name(#(#slug_names),*);
            <#inner_ty as ::server_less::JsonRpcMount>::jsonrpc_mount_dispatch_async(__delegate, __stripped, args).await
                .map_err(|msg| (-32603i32, msg))
        }
    })
}

/// Generate dispatch for a slug mount (`fn foo(&self, id: Id) -> &T`) — sync version.
fn generate_slug_mount_dispatch_sync(method: &MethodInfo) -> TokenStream2 {
    let mount_name = method.name.to_string();
    let mount_prefix = format!("{}.", mount_name);
    let method_name = &method.name;
    let inner_ty = method.return_info.reference_inner.as_ref().unwrap();

    let slug_extractions: Vec<_> = method
        .params
        .iter()
        .map(server_less_rpc::generate_param_extraction)
        .collect();
    let slug_names: Vec<_> = method.params.iter().map(|p| &p.name).collect();

    quote! {
        __method if __method.starts_with(#mount_prefix) => {
            let __stripped = &__method[#mount_prefix.len()..];
            #(#slug_extractions)*
            let __delegate = self.#method_name(#(#slug_names),*);
            <#inner_ty as ::server_less::JsonRpcMount>::jsonrpc_mount_dispatch(__delegate, __stripped, args)
        }
    }
}
