//! HTTP handler generation macro.
//!
//! Generates axum HTTP handlers from impl blocks using convention-based routing.
//!
//! # Method Naming Conventions
//!
//! HTTP methods are inferred from function name prefixes:
//! - `get_*`, `fetch_*`, `read_*`, `list_*`, `find_*`, `search_*` → GET
//! - `create_*`, `add_*`, `new_*` → POST
//! - `update_*`, `set_*` → PUT
//! - `patch_*`, `modify_*` → PATCH
//! - `delete_*`, `remove_*` → DELETE
//!
//! # Path Generation
//!
//! Paths are derived from method names:
//! - `create_user` → `POST /users`
//! - `get_user` → `GET /users/{id}` (requires id parameter)
//! - `list_users` → `GET /users`
//! - `update_user` → `PUT /users/{id}`
//!
//! # Parameter Binding
//!
//! Parameters are automatically bound based on HTTP method:
//! - GET: Path parameters (`:id`) and query parameters (`?name=value`)
//! - POST/PUT/PATCH: JSON request body
//!
//! # Context Injection
//!
//! Methods can receive a `Context` parameter to access request metadata:
//!
//! ```ignore
//! use server_less::{http, Context};
//!
//! #[http]
//! impl UserService {
//!     async fn create_user(&self, ctx: Context, name: String) -> Result<User> {
//!         // Access request metadata
//!         let user_id = ctx.user_id()?;           // Authenticated user
//!         let request_id = ctx.request_id()?;     // Request trace ID
//!         let auth_header = ctx.authorization();   // Authorization header
//!
//!         // Create user...
//!     }
//! }
//! ```
//!
//! **Context is automatically injected and populated from HTTP headers:**
//! - All headers are available via `ctx.header("name")`
//! - `x-request-id` header → `ctx.request_id()`
//! - Custom headers can be accessed via `ctx.get("key")`
//!
//! **Context does NOT appear in the OpenAPI spec** - it's injected by the framework,
//! not provided by API consumers.
//!
//! ## Name Collision Handling
//!
//! If you have your own `Context` type, use one of these strategies:
//!
//! **Strategy 1: Qualify the server-less Context (recommended)**
//! ```ignore
//! struct Context { /* your type */ }
//!
//! #[http]
//! impl MyService {
//!     // Uses server-less Context (injected)
//!     fn api_endpoint(&self, ctx: server_less::Context) { }
//!
//!     // Uses your Context (not injected, treated as body param)
//!     fn internal(&self, ctx: Context) { }
//! }
//! ```
//!
//! **Strategy 2: Rename your Context type**
//! ```ignore
//! struct AppContext { /* your type */ }
//!
//! #[http]
//! impl MyService {
//!     fn handler(&self, ctx: Context) { }  // ✅ server-less Context injected
//! }
//! ```
//!
//! **Detection Logic:**
//! - If ANY method uses `server_less::Context`, bare `Context` is assumed to be YOUR type
//! - If NO method uses qualified form, bare `Context` is assumed to be server-less
//!
//! This gives you explicit control without needing configuration flags.
//!
//! # Streaming Support (SSE)
//!
//! Return `impl Stream<Item = T>` to enable Server-Sent Events:
//!
//! ```ignore
//! use futures::stream::Stream;
//!
//! #[http]
//! impl Service {
//!     // SSE streaming endpoint
//!     // IMPORTANT: Rust 2024 requires `+ use<>` syntax
//!     fn stream_data(&self, count: u32) -> impl Stream<Item = Event> + use<> {
//!         // Returns SSE stream
//!     }
//! }
//! ```
//!
//! **Rust 2024 Edition Note:** When using `impl Trait` in return position with
//! streams, you must add `+ use<>` to capture all generic parameters. This is
//! required by Rust 2024's stricter capture rules for opaque types.
//!
//! # Generated Methods
//!
//! - `http_router() -> axum::Router` - Complete router with all endpoints
//! - `http_routes() -> Vec<&'static str>` - List of route paths
//!
//! # Example
//!
//! ```ignore
//! use server_less::http;
//!
//! #[derive(Clone)]
//! struct UserService;
//!
//! #[http]
//! impl UserService {
//!     /// Create a new user
//!     async fn create_user(&self, name: String, email: String) -> User {
//!         // POST /users with JSON body
//!     }
//!
//!     /// Get user by ID
//!     async fn get_user(&self, id: String) -> Option<User> {
//!         // GET /users/{id}
//!     }
//!
//!     /// List all users
//!     async fn list_users(&self) -> Vec<User> {
//!         // GET /users
//!     }
//! }
//!
//! // Use it:
//! let service = UserService;
//! let app = service.http_router();
//! ```

use heck::ToKebabCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use server_less_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};
use syn::{GenericArgument, ItemImpl, PathArguments, Token, Type, parse::Parse};

// Import Context helpers
use crate::context::{
    generate_http_context_extraction, has_qualified_context, partition_context_params,
    should_inject_context,
};

/// Extract the inner type T from Option<T>
fn extract_option_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }
    None
}

/// Per-method HTTP attribute overrides
#[derive(Default, Clone)]
pub struct HttpMethodOverride {
    pub method: Option<String>,
    pub path: Option<String>,
    pub skip: bool,
    pub hidden: bool,
}

impl HttpMethodOverride {
    pub fn parse_from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("route") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    result.skip = true;
                    Ok(())
                } else if meta.path.is_ident("hidden") {
                    result.hidden = true;
                    Ok(())
                } else if meta.path.is_ident("method") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    result.method = Some(value.value().to_uppercase());
                    Ok(())
                } else if meta.path.is_ident("path") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    result.path = Some(value.value());
                    Ok(())
                } else {
                    Err(meta.error("unknown attribute. Valid: method, path, skip, hidden"))
                }
            })?;
        }

        Ok(result)
    }
}

/// Per-method response customization
#[derive(Default, Clone)]
pub struct ResponseOverride {
    pub status: Option<u16>,
    pub content_type: Option<String>,
    pub headers: Vec<(String, String)>,
}

impl ResponseOverride {
    pub fn parse_from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();
        let mut pending_header_name: Option<String> = None;

        for attr in attrs {
            if !attr.path().is_ident("response") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("status") {
                    let value: syn::LitInt = meta.value()?.parse()?;
                    result.status = Some(value.base10_parse()?);
                    Ok(())
                } else if meta.path.is_ident("content_type") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    result.content_type = Some(value.value());
                    Ok(())
                } else if meta.path.is_ident("header") {
                    let name: syn::LitStr = meta.value()?.parse()?;
                    pending_header_name = Some(name.value());
                    Ok(())
                } else if meta.path.is_ident("value") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    if let Some(name) = pending_header_name.take() {
                        result.headers.push((name, value.value()));
                    }
                    Ok(())
                } else {
                    Err(meta.error(
                        "unknown attribute\n\
                         \n\
                         Valid attributes: status, content_type, header, value\n\
                         \n\
                         Examples:\n\
                         - #[response(status = 201)]\n\
                         - #[response(content_type = \"application/octet-stream\")]\n\
                         - #[response(header = \"X-Custom\", value = \"foo\")]",
                    ))
                }
            })?;
        }

        Ok(result)
    }
}

/// Arguments for the #[http] attribute
#[derive(Default)]
pub(crate) struct HttpArgs {
    pub prefix: Option<String>,
    /// Whether to generate OpenAPI spec (default: true)
    pub openapi: Option<bool>,
}

impl Parse for HttpArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = HttpArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "prefix" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.prefix = Some(lit.value());
                }
                "openapi" => {
                    let lit: syn::LitBool = input.parse()?;
                    args.openapi = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`\n\
                             Valid arguments: prefix, openapi\n\
                             Examples:\n\
                             - #[http(prefix = \"/api/v1\")]\n\
                             - #[http(openapi = false)]"
                        ),
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

pub(crate) fn expand_http(args: HttpArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    // PASS 1: Scan for qualified server_less::Context usage
    // This determines collision detection behavior
    let has_qualified = has_qualified_context(&methods);

    let prefix = args.prefix.unwrap_or_default();
    let generate_openapi = args.openapi.unwrap_or(true);

    let mut handlers = Vec::new();
    let mut routes = Vec::new();
    let mut openapi_methods = Vec::new();
    // Maps normalized route signature (e.g., "GET /users/{*}") to (method_name, original_path)
    let mut route_signatures: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();

    for method in &methods {
        let overrides = HttpMethodOverride::parse_from_attrs(&method.method.attrs)?;
        let response_overrides = ResponseOverride::parse_from_attrs(&method.method.attrs)?;

        if overrides.skip {
            continue;
        }

        // Check for duplicate routes
        let http_method_enum = if let Some(ref m) = overrides.method {
            match m.as_str() {
                "GET" => HttpMethod::Get,
                "POST" => HttpMethod::Post,
                "PUT" => HttpMethod::Put,
                "PATCH" => HttpMethod::Patch,
                "DELETE" => HttpMethod::Delete,
                _ => infer_http_method(&method.name.to_string()),
            }
        } else {
            infer_http_method(&method.name.to_string())
        };

        let path = if let Some(ref p) = overrides.path {
            p.clone()
        } else {
            infer_path(&method.name.to_string(), &http_method_enum, &method.params)
        };
        let full_path = format!("{}{}", prefix, path);

        // Normalize path for duplicate detection (e.g., /users/{id} and /users/{user_id} are the same)
        let normalized_path = normalize_path_for_duplicate_check(&full_path);
        let route_sig = format!("{} {}", http_method_enum.as_str(), normalized_path);

        if let Some((existing_method, existing_path)) = route_signatures.get(&route_sig) {
            let hint_msg = if existing_path != &full_path {
                format!(
                    "Duplicate route: {} {} is structurally identical to {} defined by method '{}'\n\
                     \n\
                     Note: These paths have the same structure (different parameter names don't matter):\n\
                     - Method '{}': {}\n\
                     - Method '{}': {}\n\
                     \n\
                     Hint: You can either:\n\
                     1. Use #[route(skip)] to exclude one method from HTTP routing\n\
                     2. Use #[route(path = \"/custom\")] to use a completely different path\n\
                     3. Use #[route(method = \"PATCH\")] to use a different HTTP method",
                    http_method_enum.as_str(),
                    full_path,
                    existing_path,
                    existing_method,
                    existing_method,
                    existing_path,
                    method.name,
                    full_path
                )
            } else {
                format!(
                    "Duplicate route: {} {} is already defined by method '{}'\n\
                     \n\
                     Hint: You can either:\n\
                     1. Use #[route(skip)] to exclude this method from HTTP routing\n\
                     2. Use #[route(path = \"/custom\")] to use a different path\n\
                     3. Use #[route(method = \"PATCH\")] to use a different HTTP method",
                    http_method_enum.as_str(),
                    full_path,
                    existing_method
                )
            };

            return Err(syn::Error::new_spanned(&method.method.sig, hint_msg));
        }
        route_signatures.insert(route_sig, (method.name.to_string(), full_path.clone()));

        let handler = generate_handler(&struct_name, method, &response_overrides, has_qualified)?;
        handlers.push(handler);

        let route = generate_route(&prefix, method, &overrides, &struct_name)?;
        routes.push(route);

        if generate_openapi && !overrides.hidden {
            openapi_methods.push((
                method.clone(),
                overrides.clone(),
                response_overrides.clone(),
            ));
        }
    }

    // Conditionally generate OpenAPI spec method
    let openapi_method = if generate_openapi {
        let openapi_fn =
            generate_openapi_spec(&struct_name, &prefix, &openapi_methods, has_qualified)?;
        quote! {
            /// Get OpenAPI specification for this service
            pub fn openapi_spec() -> ::server_less::serde_json::Value {
                #openapi_fn
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Create an axum Router for this service
            pub fn http_router(self) -> ::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                use ::axum::routing::{get, post, put, patch, delete};

                let state = ::std::sync::Arc::new(self);
                ::axum::Router::new()
                    #(#routes)*
                    .with_state(state)
            }

            #openapi_method
        }

        #(#handlers)*
    })
}

fn generate_handler(
    struct_name: &syn::Ident,
    method: &MethodInfo,
    response_overrides: &ResponseOverride,
    has_qualified: bool,
) -> syn::Result<TokenStream2> {
    let method_name = &method.name;
    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_http_{}_{}", struct_name_snake, method_name);

    let (param_extractions, param_calls) = generate_param_handling(method, has_qualified)?;

    let call = if method.is_async {
        quote! { state.#method_name(#(#param_calls),*).await }
    } else {
        quote! { state.#method_name(#(#param_calls),*) }
    };

    let response = generate_response_handling(method, &call, response_overrides)?;

    let handler = quote! {
        async fn #handler_name(
            state_extractor: ::axum::extract::State<::std::sync::Arc<#struct_name>>,
            #(#param_extractions),*
        ) -> impl ::axum::response::IntoResponse {
            let state = state_extractor.0;
            #response
        }
    };

    Ok(handler)
}

fn generate_param_handling(
    method: &MethodInfo,
    has_qualified: bool,
) -> syn::Result<(Vec<TokenStream2>, Vec<TokenStream2>)> {
    use server_less_parse::ParamLocation;

    let mut extractions = Vec::new();
    let mut calls = Vec::new();

    let http_method = infer_http_method(&method.name.to_string());
    let default_has_body = matches!(
        http_method,
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch
    );

    // Partition Context vs regular parameters
    let (context_param, regular_params) = partition_context_params(&method.params, has_qualified)?;

    // Generate Context extraction (if needed)
    if context_param.is_some() {
        let (extraction, call) = generate_http_context_extraction();
        extractions.push(extraction);
        calls.push(call);
    }

    // Group regular parameters by their actual location (respecting overrides)
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut body_params = Vec::new();
    let mut header_params = Vec::new();

    for param in regular_params {
        match param.location.as_ref() {
            Some(ParamLocation::Path) => path_params.push(param),
            Some(ParamLocation::Query) => query_params.push(param),
            Some(ParamLocation::Body) => body_params.push(param),
            Some(ParamLocation::Header) => header_params.push(param),
            None => {
                // Infer location based on conventions
                if param.is_id {
                    path_params.push(param);
                } else if default_has_body {
                    body_params.push(param);
                } else {
                    query_params.push(param);
                }
            }
        }
    }

    // Generate path parameter extraction
    if !path_params.is_empty() {
        for param in &path_params {
            let ty = &param.ty;
            extractions.push(quote! {
                path_extractor: ::axum::extract::Path<#ty>
            });
            calls.push(quote! { path_extractor.0 });
        }
    }

    // Generate body parameter extraction
    if !body_params.is_empty() {
        extractions.push(quote! {
            body_extractor: ::axum::extract::Json<::server_less::serde_json::Value>
        });

        for param in &body_params {
            // Use wire_name if provided, otherwise use the parameter name
            let name_str = param
                .wire_name
                .clone()
                .unwrap_or_else(|| param.name.to_string());
            let ty = &param.ty;
            if param.is_optional {
                let inner_ty = extract_option_inner(ty).unwrap_or_else(|| ty.clone());
                calls.push(quote! {
                        body_extractor.0.get(#name_str).and_then(|v| ::server_less::serde_json::from_value::<#inner_ty>(v.clone()).ok())
                    });
            } else {
                calls.push(quote! {
                        ::server_less::serde_json::from_value::<#ty>(body_extractor.0.get(#name_str).cloned().unwrap_or_default()).unwrap_or_default()
                    });
            }
        }
    }

    // Generate query parameter extraction
    if !query_params.is_empty() {
        extractions.push(quote! {
            query_extractor: ::axum::extract::Query<::std::collections::HashMap<String, String>>
        });

        for param in &query_params {
            // Use wire_name if provided, otherwise use the parameter name
            let name_str = param
                .wire_name
                .clone()
                .unwrap_or_else(|| param.name.to_string());
            let ty = &param.ty;

            // Handle default values
            if param.is_optional {
                let inner_ty = extract_option_inner(ty).unwrap_or_else(|| ty.clone());
                calls.push(quote! {
                    query_extractor.0.get(#name_str).and_then(|v| v.parse::<#inner_ty>().ok())
                });
            } else if let Some(ref default_val) = param.default_value {
                // Parse the default value at compile time
                let default_expr: proc_macro2::TokenStream = default_val.parse().unwrap();
                calls.push(quote! {
                    query_extractor.0.get(#name_str)
                        .and_then(|v| v.parse::<#ty>().ok())
                        .unwrap_or(#default_expr)
                });
            } else {
                calls.push(quote! {
                    query_extractor.0.get(#name_str).and_then(|v| v.parse::<#ty>().ok()).unwrap_or_default()
                });
            }
        }
    }

    // Generate header parameter extraction
    if !header_params.is_empty() {
        extractions.push(quote! {
            headers: ::axum::http::HeaderMap
        });

        for param in &header_params {
            // Use wire_name if provided, otherwise use the parameter name
            let name_str = param
                .wire_name
                .clone()
                .unwrap_or_else(|| param.name.to_string());
            let ty = &param.ty;

            if param.is_optional {
                let inner_ty = extract_option_inner(ty).unwrap_or_else(|| ty.clone());
                calls.push(quote! {
                    headers.get(#name_str)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<#inner_ty>().ok())
                });
            } else {
                calls.push(quote! {
                    headers.get(#name_str)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<#ty>().ok())
                        .unwrap_or_default()
                });
            }
        }
    }

    Ok((extractions, calls))
}

fn generate_response_handling(
    method: &MethodInfo,
    call: &TokenStream2,
    response_overrides: &ResponseOverride,
) -> syn::Result<TokenStream2> {
    let ret = &method.return_info;

    let base_response = if ret.is_unit {
        quote! {
            {
                #call;
                ::axum::http::StatusCode::NO_CONTENT
            }
        }
    } else if ret.is_result {
        quote! {
            {
                use ::axum::response::IntoResponse;
                match #call {
                    Ok(value) => ::axum::Json(value).into_response(),
                    Err(err) => {
                        let code = ::server_less::ErrorCode::infer_from_name(&format!("{:?}", err));
                        let status = ::axum::http::StatusCode::from_u16(code.http_status())
                            .unwrap_or(::axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                        let body = ::server_less::serde_json::json!({
                            "error": format!("{:?}", err),
                            "message": format!("{}", err)
                        });
                        (status, ::axum::Json(body)).into_response()
                    }
                }
            }
        }
    } else if ret.is_option {
        quote! {
            {
                use ::axum::response::IntoResponse;
                match #call {
                    Some(value) => ::axum::Json(value).into_response(),
                    None => ::axum::http::StatusCode::NOT_FOUND.into_response(),
                }
            }
        }
    } else if ret.is_stream {
        quote! {
            {
                use ::server_less::futures::StreamExt;
                let stream = #call;
                let boxed_stream = Box::pin(stream);
                ::axum::response::sse::Sse::new(
                    boxed_stream.map(|item| {
                        Ok::<_, std::convert::Infallible>(
                            ::axum::response::sse::Event::default()
                                .json_data(item)
                                .unwrap()
                        )
                    })
                )
            }
        }
    } else {
        quote! {
            {
                let result = #call;
                ::axum::Json(result)
            }
        }
    };

    // Apply response overrides if any are specified
    if response_overrides.status.is_some()
        || response_overrides.content_type.is_some()
        || !response_overrides.headers.is_empty()
    {
        apply_response_overrides(base_response, response_overrides)
    } else {
        Ok(base_response)
    }
}

/// Apply response overrides (status, headers, content-type) to a base response
fn apply_response_overrides(
    base_response: TokenStream2,
    overrides: &ResponseOverride,
) -> syn::Result<TokenStream2> {
    let status_code = if let Some(status) = overrides.status {
        quote! {
            ::axum::http::StatusCode::from_u16(#status)
                .unwrap_or(::axum::http::StatusCode::OK)
        }
    } else {
        quote! { ::axum::http::StatusCode::OK }
    };

    let header_insertions: Vec<TokenStream2> = overrides
        .headers
        .iter()
        .map(|(name, value)| {
            quote! {
                headers.insert(
                    ::axum::http::header::HeaderName::from_static(#name),
                    ::axum::http::header::HeaderValue::from_static(#value)
                );
            }
        })
        .collect();

    let content_type_insertion = if let Some(ref ct) = overrides.content_type {
        quote! {
            headers.insert(
                ::axum::http::header::CONTENT_TYPE,
                ::axum::http::header::HeaderValue::from_static(#ct)
            );
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        {
            use ::axum::response::IntoResponse;
            let base_response = #base_response;
            let mut headers = ::axum::http::HeaderMap::new();
            #(#header_insertions)*
            #content_type_insertion
            (#status_code, headers, base_response).into_response()
        }
    })
}

fn generate_route(
    prefix: &str,
    method: &MethodInfo,
    overrides: &HttpMethodOverride,
    struct_name: &syn::Ident,
) -> syn::Result<TokenStream2> {
    let method_name = &method.name;
    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_http_{}_{}", struct_name_snake, method_name);

    let http_method = if let Some(ref m) = overrides.method {
        match m.as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            "DELETE" => HttpMethod::Delete,
            _ => infer_http_method(&method_name.to_string()),
        }
    } else {
        infer_http_method(&method_name.to_string())
    };

    let path = if let Some(ref p) = overrides.path {
        validate_http_path(p)?;
        p.clone()
    } else {
        infer_path(&method_name.to_string(), &http_method, &method.params)
    };
    let full_path = format!("{}{}", prefix, path);

    let method_fn = match http_method {
        HttpMethod::Get => quote! { get },
        HttpMethod::Post => quote! { post },
        HttpMethod::Put => quote! { put },
        HttpMethod::Patch => quote! { patch },
        HttpMethod::Delete => quote! { delete },
    };

    Ok(quote! {
        .route(#full_path, #method_fn(#handler_name))
    })
}

pub fn generate_openapi_spec(
    struct_name: &syn::Ident,
    prefix: &str,
    methods_with_overrides: &[(MethodInfo, HttpMethodOverride, ResponseOverride)],
    has_qualified: bool,
) -> syn::Result<TokenStream2> {
    let mut operation_data = Vec::new();

    for (method, overrides, response_overrides) in methods_with_overrides {
        let method_name = method.name.to_string();

        let http_method = if let Some(ref m) = overrides.method {
            match m.as_str() {
                "GET" => HttpMethod::Get,
                "POST" => HttpMethod::Post,
                "PUT" => HttpMethod::Put,
                "PATCH" => HttpMethod::Patch,
                "DELETE" => HttpMethod::Delete,
                _ => infer_http_method(&method_name),
            }
        } else {
            infer_http_method(&method_name)
        };

        let path = if let Some(ref p) = overrides.path {
            p.clone()
        } else {
            infer_path(&method_name, &http_method, &method.params)
        };
        let full_path = format!("{}{}", prefix, path);
        let http_method_str = http_method.as_str().to_lowercase();

        let summary = method.docs.clone().unwrap_or_else(|| method_name.clone());
        let operation_id = method_name.clone();

        let default_has_body = matches!(
            http_method,
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch
        );

        // Group parameters by their actual location (respecting overrides)
        // Filter out Context parameters - they're injected and not part of the API contract
        use server_less_parse::ParamLocation;
        let mut path_params = Vec::new();
        let mut query_params = Vec::new();
        let mut body_params = Vec::new();
        let mut header_params = Vec::new();

        for param in &method.params {
            // Skip Context parameters - they're injected by the framework
            if should_inject_context(&param.ty, has_qualified) {
                continue;
            }

            match param.location.as_ref() {
                Some(ParamLocation::Path) => path_params.push(param),
                Some(ParamLocation::Query) => query_params.push(param),
                Some(ParamLocation::Body) => body_params.push(param),
                Some(ParamLocation::Header) => header_params.push(param),
                None => {
                    // Infer location based on conventions
                    if param.is_id {
                        path_params.push(param);
                    } else if default_has_body {
                        body_params.push(param);
                    } else {
                        query_params.push(param);
                    }
                }
            }
        }

        let path_param_specs: Vec<_> = path_params
            .iter()
            .map(|p| {
                // Use wire_name if provided, otherwise use the parameter name
                let name = p.wire_name.clone().unwrap_or_else(|| p.name.to_string());
                let json_type = server_less_rpc::infer_json_type(&p.ty);
                quote! { (#name, "path", #json_type, true) }
            })
            .collect();

        let query_param_specs: Vec<TokenStream2> = query_params
            .iter()
            .map(|p| {
                // Use wire_name if provided, otherwise use the parameter name
                let name = p.wire_name.clone().unwrap_or_else(|| p.name.to_string());
                let json_type = server_less_rpc::infer_json_type(&p.ty);
                let required = !p.is_optional && p.default_value.is_none();
                quote! { (#name, "query", #json_type, #required) }
            })
            .collect();

        let header_param_specs: Vec<TokenStream2> = header_params
            .iter()
            .map(|p| {
                // Use wire_name if provided, otherwise use the parameter name
                let name = p.wire_name.clone().unwrap_or_else(|| p.name.to_string());
                let json_type = server_less_rpc::infer_json_type(&p.ty);
                let required = !p.is_optional && p.default_value.is_none();
                quote! { (#name, "header", #json_type, #required) }
            })
            .collect();

        let body_props: Vec<TokenStream2> = body_params
            .iter()
            .map(|p| {
                // Use wire_name if provided, otherwise use the parameter name
                let name = p.wire_name.clone().unwrap_or_else(|| p.name.to_string());
                let json_type = server_less_rpc::infer_json_type(&p.ty);
                let required = !p.is_optional && p.default_value.is_none();
                quote! { (#name, #json_type, #required) }
            })
            .collect();
        let has_body_props = !body_props.is_empty();

        let ret = &method.return_info;

        // Determine success code - use override if provided, otherwise infer
        let inferred_code = if ret.is_unit { "204" } else { "200" };
        let success_code = response_overrides
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| inferred_code.to_string());

        let error_responses = ret.is_result;

        // Build custom response metadata at macro expansion time
        let has_content_type = response_overrides.content_type.is_some();
        let content_type_value = response_overrides.content_type.as_deref().unwrap_or("");
        let header_insertions: Vec<TokenStream2> = response_overrides
            .headers
            .iter()
            .map(|(name, _)| {
                quote! {
                    headers_obj.insert(#name.to_string(), ::server_less::serde_json::json!({
                        "description": format!("Custom header: {}", #name),
                        "schema": {
                            "type": "string"
                        }
                    }));
                }
            })
            .collect();
        let has_custom_headers = !response_overrides.headers.is_empty();

        operation_data.push(quote! {
            {
                let path = #full_path;
                let method = #http_method_str;
                let summary = #summary;
                let operation_id = #operation_id;
                let success_code = #success_code;
                let has_error_responses = #error_responses;
                let has_body = #has_body_props;

                let mut parameters: Vec<::server_less::serde_json::Value> = Vec::new();
                #(
                    {
                        let (name, location, schema_type, required): (&str, &str, &str, bool) = #path_param_specs;
                        parameters.push(::server_less::serde_json::json!({
                            "name": name,
                            "in": location,
                            "required": required,
                            "schema": { "type": schema_type }
                        }));
                    }
                )*
                #(
                    {
                        let (name, location, schema_type, required): (&str, &str, &str, bool) = #query_param_specs;
                        parameters.push(::server_less::serde_json::json!({
                            "name": name,
                            "in": location,
                            "required": required,
                            "schema": { "type": schema_type }
                        }));
                    }
                )*
                #(
                    {
                        let (name, location, schema_type, required): (&str, &str, &str, bool) = #header_param_specs;
                        parameters.push(::server_less::serde_json::json!({
                            "name": name,
                            "in": location,
                            "required": required,
                            "schema": { "type": schema_type }
                        }));
                    }
                )*

                let request_body: Option<::server_less::serde_json::Value> = if has_body {
                    let mut properties = ::server_less::serde_json::Map::new();
                    let mut required_props: Vec<String> = Vec::new();
                    #(
                        {
                            let (name, schema_type, required): (&str, &str, bool) = #body_props;
                            properties.insert(name.to_string(), ::server_less::serde_json::json!({
                                "type": schema_type
                            }));
                            if required {
                                required_props.push(name.to_string());
                            }
                        }
                    )*
                    Some(::server_less::serde_json::json!({
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": properties,
                                    "required": required_props
                                }
                            }
                        }
                    }))
                } else {
                    None
                };

                let mut responses = ::server_less::serde_json::Map::new();

                // Build success response with optional content type and headers
                let mut success_response = ::server_less::serde_json::json!({
                    "description": "Successful response"
                });

                // Add content type if specified
                if #has_content_type {
                    let content_obj = ::server_less::serde_json::json!({
                        #content_type_value: {
                            "schema": {
                                "type": "string"
                            }
                        }
                    });
                    success_response.as_object_mut().unwrap()
                        .insert("content".to_string(), content_obj);
                }

                // Add custom headers if specified
                if #has_custom_headers {
                    let mut headers_obj = ::server_less::serde_json::Map::new();
                    #(#header_insertions)*
                    success_response.as_object_mut().unwrap()
                        .insert("headers".to_string(), ::server_less::serde_json::Value::Object(headers_obj));
                }

                responses.insert(success_code.to_string(), success_response);

                if has_error_responses {
                    responses.insert("400".to_string(), ::server_less::serde_json::json!({
                        "description": "Bad request"
                    }));
                    responses.insert("500".to_string(), ::server_less::serde_json::json!({
                        "description": "Internal server error"
                    }));
                }

                let mut operation = ::server_less::serde_json::json!({
                    "summary": summary,
                    "operationId": operation_id,
                    "responses": responses
                });

                if !parameters.is_empty() {
                    operation.as_object_mut().unwrap()
                        .insert("parameters".to_string(), ::server_less::serde_json::Value::Array(parameters));
                }

                if let Some(body) = request_body {
                    operation.as_object_mut().unwrap()
                        .insert("requestBody".to_string(), body);
                }

                (path.to_string(), method.to_string(), operation)
            }
        });
    }

    Ok(quote! {
        {
            let mut paths = ::server_less::serde_json::Map::new();

            #(
                {
                    let (path, method, operation): (String, String, ::server_less::serde_json::Value) = #operation_data;
                    let path_item = paths.entry(path)
                        .or_insert_with(|| ::server_less::serde_json::json!({}));
                    if let ::server_less::serde_json::Value::Object(map) = path_item {
                        map.insert(method, operation);
                    }
                }
            )*

            ::server_less::serde_json::json!({
                "openapi": "3.0.0",
                "info": {
                    "title": stringify!(#struct_name),
                    "version": "0.1.0"
                },
                "paths": paths
            })
        }
    })
}

#[derive(Debug, Clone, Copy)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }
}

fn infer_http_method(name: &str) -> HttpMethod {
    if name.starts_with("get_")
        || name.starts_with("fetch_")
        || name.starts_with("read_")
        || name.starts_with("list_")
        || name.starts_with("find_")
        || name.starts_with("search_")
    {
        HttpMethod::Get
    } else if name.starts_with("create_") || name.starts_with("add_") || name.starts_with("new_") {
        HttpMethod::Post
    } else if name.starts_with("update_") || name.starts_with("set_") {
        HttpMethod::Put
    } else if name.starts_with("patch_") || name.starts_with("modify_") {
        HttpMethod::Patch
    } else if name.starts_with("delete_") || name.starts_with("remove_") {
        HttpMethod::Delete
    } else {
        HttpMethod::Post
    }
}

/// Normalize a path for duplicate detection by replacing all path parameters with a placeholder
///
/// This ensures that paths like `/users/{id}` and `/users/{user_id}` are detected as duplicates,
/// since they have the same routing structure even though parameter names differ.
fn normalize_path_for_duplicate_check(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "{*}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn infer_path(method_name: &str, http_method: &HttpMethod, params: &[ParamInfo]) -> String {
    let resource = method_name
        .strip_prefix("get_")
        .or_else(|| method_name.strip_prefix("fetch_"))
        .or_else(|| method_name.strip_prefix("read_"))
        .or_else(|| method_name.strip_prefix("list_"))
        .or_else(|| method_name.strip_prefix("find_"))
        .or_else(|| method_name.strip_prefix("search_"))
        .or_else(|| method_name.strip_prefix("create_"))
        .or_else(|| method_name.strip_prefix("add_"))
        .or_else(|| method_name.strip_prefix("new_"))
        .or_else(|| method_name.strip_prefix("update_"))
        .or_else(|| method_name.strip_prefix("set_"))
        .or_else(|| method_name.strip_prefix("patch_"))
        .or_else(|| method_name.strip_prefix("modify_"))
        .or_else(|| method_name.strip_prefix("delete_"))
        .or_else(|| method_name.strip_prefix("remove_"))
        .unwrap_or(method_name);

    let resource_kebab = resource.to_kebab_case();
    let path_resource = if resource_kebab.ends_with('s') {
        resource_kebab
    } else {
        format!("{}s", resource_kebab)
    };

    let has_id = params.iter().any(|p| p.is_id);

    match http_method {
        HttpMethod::Post => format!("/{}", path_resource),
        HttpMethod::Get
            if method_name.starts_with("list_")
                || method_name.starts_with("search_")
                || method_name.starts_with("find_") =>
        {
            format!("/{}", path_resource)
        }
        HttpMethod::Get | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Delete if has_id => {
            format!("/{}/{{id}}", path_resource)
        }
        _ => format!("/{}", path_resource),
    }
}

/// Validate HTTP path at compile time
fn validate_http_path(path: &str) -> syn::Result<()> {
    // Check that path starts with /
    if !path.starts_with('/') {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "HTTP path must start with '/'. Got: '{}'\n\
                 \n\
                 Hint: Change to '/{}'",
                path, path
            ),
        ));
    }

    // Check for multiple consecutive slashes
    if path.contains("//") {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "HTTP path contains consecutive slashes. Path: '{}'\n\
                 \n\
                 Hint: Use single slashes to separate path segments, e.g., /users/posts",
                path
            ),
        ));
    }

    // Warn about trailing slashes (can cause routing issues)
    if path.len() > 1 && path.ends_with('/') {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "HTTP path has trailing slash. Path: '{}'\n\
                 \n\
                 Hint: Remove trailing slash: '{}'\n\
                 Trailing slashes can cause routing inconsistencies.",
                path,
                path.trim_end_matches('/')
            ),
        ));
    }

    // Check for invalid characters
    let invalid_chars = ['<', '>', '"', '`', ' ', '\t', '\n', '?', '#'];
    if let Some(ch) = invalid_chars.iter().find(|&&c| path.contains(c)) {
        let hint = if *ch == '<' || *ch == '>' {
            "\n\nHint: Use curly braces for path parameters, e.g., /users/{id}"
        } else if *ch == ' ' {
            "\n\nHint: Use hyphens or underscores instead of spaces, e.g., /my-resource"
        } else if *ch == '?' {
            "\n\nHint: Query parameters are added automatically from method parameters"
        } else if *ch == '#' {
            "\n\nHint: Fragment identifiers are not supported in server routes"
        } else {
            ""
        };
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "HTTP path contains invalid character '{}'. Path: '{}'{}",
                ch, path, hint
            ),
        ));
    }

    // Check for malformed path parameters
    let open_braces = path.matches('{').count();
    let close_braces = path.matches('}').count();
    if open_braces != close_braces {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "HTTP path has mismatched braces. Path: '{}'\n\
                 \n\
                 Found {} opening '{{' and {} closing '}}'\n\
                 Hint: Each path parameter should be wrapped in braces, e.g., /users/{{id}}",
                path, open_braces, close_braces
            ),
        ));
    }

    // Extract and validate path parameter names
    let mut param_names = std::collections::HashSet::new();
    for (idx, part) in path.split('/').enumerate() {
        if part.starts_with('{') && part.ends_with('}') {
            let param_name = &part[1..part.len() - 1];

            // Check for empty parameter name
            if param_name.is_empty() {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "HTTP path has empty path parameter at segment {}. Path: '{}'\n\
                         \n\
                         Hint: Path parameters need names, e.g., /users/{{id}} or /posts/{{post_id}}",
                        idx, path
                    ),
                ));
            }

            // Check for valid parameter name (alphanumeric, underscore, hyphen)
            if !param_name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "HTTP path parameter '{}' contains invalid characters. Path: '{}'\n\
                         \n\
                         Hint: Parameter names should only contain alphanumeric characters, underscores, and hyphens",
                        param_name, path
                    ),
                ));
            }

            // Check for duplicate parameter names
            if !param_names.insert(param_name.to_string()) {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "HTTP path has duplicate parameter '{{{}}}'. Path: '{}'\n\
                         \n\
                         Hint: Each path parameter must have a unique name\n\
                         Consider using names like {{user_id}} and {{post_id}} instead of multiple {{id}}",
                        param_name, path
                    ),
                ));
            }
        } else if part.contains('{') || part.contains('}') {
            // Malformed segment with partial braces
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "HTTP path has malformed path parameter at segment {}. Path: '{}'\n\
                     \n\
                     Hint: Path parameters must be complete segments, e.g., /users/{{id}}/posts\n\
                     Not: /users/user-{{id}}/posts",
                    idx, path
                ),
            ));
        }
    }

    Ok(())
}

/// Arguments for the #[serve] attribute
#[derive(Default)]
pub(crate) struct ServeArgs {
    /// Protocols to serve (http, ws)
    pub protocols: Vec<String>,
    /// Health check path (default: /health)
    pub health_path: Option<String>,
}

impl Parse for ServeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ServeArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let ident_str = ident.to_string();

            match ident_str.as_str() {
                "http" | "ws" | "jsonrpc" | "graphql" => {
                    args.protocols.push(ident_str);
                }
                "health" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitStr = input.parse()?;
                    args.health_path = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown protocol `{other}`\n\
                             \n\
                             Valid protocols: http, ws, jsonrpc, graphql\n\
                             Valid options: health\n\
                             \n\
                             Example: #[serve(http, ws, health = \"/status\")]"
                        ),
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

/// Coordinate multiple protocol handlers into a single server.
pub(crate) fn expand_serve(args: ServeArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;

    let health_path = args.health_path.unwrap_or_else(|| "/health".to_string());

    // Build router combination based on protocols
    let router_setup = generate_router_setup(&args.protocols);

    // Generate the serve method
    let serve_impl = quote! {
        impl #struct_name {
            /// Start serving all configured protocols.
            pub async fn serve(self, addr: impl ::std::convert::AsRef<str>) -> ::std::io::Result<()>
            where
                Self: Clone + Send + Sync + 'static,
            {
                #router_setup

                // Add health check
                let router = router.route(
                    #health_path,
                    ::axum::routing::get(|| async { "ok" })
                );

                let listener = ::tokio::net::TcpListener::bind(addr.as_ref()).await?;
                ::axum::serve(listener, router).await
            }

            /// Build the combined router without starting the server.
            pub fn router(self) -> ::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                #router_setup

                router.route(
                    #health_path,
                    ::axum::routing::get(|| async { "ok" })
                )
            }
        }
    };

    Ok(quote! {
        #impl_block

        #serve_impl
    })
}

/// Generate router setup code based on enabled protocols
fn generate_router_setup(protocols: &[String]) -> TokenStream2 {
    let has_http = protocols.contains(&"http".to_string());
    let has_ws = protocols.contains(&"ws".to_string());
    let has_jsonrpc = protocols.contains(&"jsonrpc".to_string());
    let has_graphql = protocols.contains(&"graphql".to_string());

    // Build list of merge operations
    let mut parts = Vec::new();

    if has_http {
        parts.push(quote! { self.clone().http_router() });
    }
    if has_ws {
        parts.push(quote! { self.clone().ws_router() });
    }
    if has_jsonrpc {
        parts.push(quote! { self.clone().jsonrpc_router() });
    }
    if has_graphql {
        parts.push(quote! { self.clone().graphql_router() });
    }

    if parts.is_empty() {
        quote! {
            let router = ::axum::Router::new();
        }
    } else if parts.len() == 1 {
        let first = &parts[0];
        quote! {
            let router = #first;
        }
    } else {
        let first = &parts[0];
        let rest = &parts[1..];
        quote! {
            let router = #first #(.merge(#rest))*;
        }
    }
}
