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

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use server_less_parse::{MethodInfo, extract_methods, get_impl_name, partition_methods};
use syn::{GenericArgument, ItemImpl, PathArguments, Token, Type, parse::Parse};

use crate::app::extract_app_meta;
use crate::server_attrs::{has_server_hidden, has_server_skip};

// Import Context helpers
use crate::context::{
    generate_http_context_extraction, has_qualified_context, partition_context_params,
};

// Re-export shared OpenAPI types for backward compatibility
pub use crate::openapi_gen::{
    HttpMethod, ResponseOverride, RouteOverride, infer_http_method,
    infer_path,
};

// Type alias for backward compatibility
pub type HttpMethodOverride = RouteOverride;

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

/// Arguments for the #[http] attribute
#[derive(Default)]
pub(crate) struct HttpArgs {
    pub prefix: Option<String>,
    /// Whether to generate OpenAPI spec (default: true)
    pub openapi: Option<bool>,
    /// Application name (used as OpenAPI info.title, overrides struct name)
    pub name: Option<String>,
    /// Human-readable description (used as OpenAPI info.description)
    pub description: Option<String>,
    /// Application version (used as OpenAPI info.version, defaults to CARGO_PKG_VERSION)
    pub version: Option<String>,
    /// Homepage URL (used as OpenAPI info.contact.url)
    pub homepage: Option<String>,
    /// Whether to emit debug logging in generated handlers (default: false).
    /// When true, each handler emits `eprintln!` lines before and after the
    /// method call. Set on the impl block to enable for all methods, or on a
    /// specific method via `#[http(debug = true)]`.
    pub debug: bool,
    /// Whether to emit per-parameter trace logging in generated handlers (default: false).
    /// When true, each handler emits an `eprintln!` line after each parameter is extracted,
    /// showing the parameter name and its `{:?}` value. Set on the impl block to enable for
    /// all methods, or on a specific method via `#[http(trace = true)]`.
    pub trace: bool,
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
                "debug" => {
                    let lit: syn::LitBool = input.parse()?;
                    args.debug = lit.value();
                }
                "trace" => {
                    let lit: syn::LitBool = input.parse()?;
                    args.trace = lit.value();
                }
                "name" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.name = Some(lit.value());
                }
                "description" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.description = Some(lit.value());
                }
                "version" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                "homepage" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.homepage = Some(lit.value());
                }
                other => {
                    const VALID: &[&str] =
                        &["prefix", "openapi", "name", "description", "version", "homepage", "debug", "trace"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}\n\
                             Valid arguments: prefix, openapi, name, description, version, homepage, debug, trace\n\
                             Examples:\n\
                             - #[http(prefix = \"/api/v1\")]\n\
                             - #[http(openapi = false)]\n\
                             - #[http(name = \"My API\", description = \"Does the thing\")]\n\
                             - #[http(debug = true)]\n\
                             \n\
                             Related: #[serve] (multi-protocol), #[openapi] (standalone API docs), #[server] (blessed preset)"
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

/// Strip `#[param]`, `#[route]`, `#[response]`, and per-method `#[http]` attributes
/// from the impl block before re-emitting it, so rustc does not encounter unknown
/// or macro attributes on function parameters / methods in the generated output.
fn strip_http_attrs(impl_block: &ItemImpl) -> ItemImpl {
    let mut block = impl_block.clone();
    for item in &mut block.items {
        if let syn::ImplItem::Fn(method) = item {
            // Strip method-level HTTP attributes (route, response, and per-method http).
            method.attrs.retain(|attr| {
                !attr.path().is_ident("route")
                    && !attr.path().is_ident("response")
                    && !attr.path().is_ident("http")
            });
            // Strip #[param(...)] from function parameters.
            for input in &mut method.sig.inputs {
                if let syn::FnArg::Typed(pat_type) = input {
                    pat_type.attrs.retain(|attr| !attr.path().is_ident("param"));
                }
            }
        }
    }
    block
}

pub(crate) fn expand_http(args: HttpArgs, mut impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let app_meta = extract_app_meta(&mut impl_block.attrs);
    let args = HttpArgs {
        name: args.name.or(app_meta.name),
        description: args.description.or(app_meta.description),
        version: args.version.or_else(|| app_meta.version.and_then(|v| v)),
        homepage: args.homepage.or(app_meta.homepage),
        ..args
    };

    let struct_name = get_impl_name(&impl_block)?;
    let (impl_generics, _ty_generics, where_clause) = impl_block.generics.split_for_impl();
    let self_ty = &impl_block.self_ty;
    let methods = extract_methods(&impl_block)?;

    // PASS 1: Scan for qualified server_less::Context usage
    // This determines collision detection behavior
    let has_qualified = has_qualified_context(&methods);

    let prefix = args.prefix.unwrap_or_default();
    let generate_openapi = args.openapi.unwrap_or(true);
    let impl_debug = args.debug;
    let impl_trace = args.trace;
    let openapi_title = args.name.unwrap_or_else(|| struct_name.to_string());
    let openapi_version = match args.version {
        Some(ref v) => quote! { #v },
        None => quote! { ::std::env!("CARGO_PKG_VERSION") },
    };
    let openapi_description_entry = match args.description {
        Some(ref d) => quote! { , "description": #d },
        None => quote! {},
    };
    let openapi_contact_entry = match args.homepage {
        Some(ref hp) => quote! { , "contact": { "url": #hp } },
        None => quote! {},
    };

    let partitioned = partition_methods(&methods, has_server_skip);

    // Generate mount routes (static mounts only)
    let mut mount_routes = Vec::new();
    let mut mount_openapi_calls = Vec::new();
    for mount in &partitioned.static_mounts {
        let mount_name = mount.name_str();
        let mount_path = format!("/{}", mount_name);
        let method_name = &mount.name;
        let inner_ty = mount.return_info.reference_inner.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(
                &mount.method.sig,
                "BUG: mount method must have a reference return type (&T)",
            )
        })?;

        mount_routes.push(quote! {
            .nest_service(#mount_path, <#inner_ty as ::server_less::HttpMount>::http_mount_router(
                ::std::sync::Arc::new(state.#method_name().clone())
            ))
        });

        // Collect child OpenAPI paths prefixed with the mount path.
        mount_openapi_calls.push(quote! {
            for mut child_path in <#inner_ty as ::server_less::HttpMount>::http_mount_openapi_paths() {
                child_path.path = format!("{}{}", #mount_path, child_path.path);
                paths.push(child_path);
            }
        });
    }

    let mut handlers = Vec::new();
    let mut routes = Vec::new();
    let mut openapi_methods = Vec::new();
    let mut route_docs: Vec<String> = Vec::new();
    // Maps normalized route signature (e.g., "GET /users/{*}") to (method_name, original_path)
    let mut route_signatures: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();

    for method in &partitioned.leaf {
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
                _ => infer_http_method(&method.name_str()),
            }
        } else {
            infer_http_method(&method.name_str())
        };

        let path = if let Some(ref p) = overrides.path {
            p.clone()
        } else {
            infer_path(&method.name_str(), &http_method_enum, &method.params)
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
        route_signatures.insert(
            route_sig.clone(),
            (method.name_str(), full_path.clone()),
        );
        route_docs.push(format!("- `{}`", route_sig));

        // Per-method debug flag: method-level `#[http(debug = true)]` OR impl-level flag.
        let method_debug = impl_debug || has_http_debug(method);
        // Per-method trace flag: method-level `#[http(trace = true)]` OR impl-level flag.
        let method_trace = impl_trace || has_http_trace(method);
        let handler = generate_handler(&struct_name, self_ty, method, &response_overrides, has_qualified, method_debug, method_trace)?;
        handlers.push(handler);

        let route = generate_route(&prefix, method, &overrides, &struct_name)?;
        routes.push(route);

        // Always collect for http_openapi_paths() (used by #[openapi] and #[serve])
        // Exclude from OpenAPI if hidden via #[route(hidden)] or #[server(hidden)]
        if !overrides.hidden && !has_server_hidden(method) {
            openapi_methods.push((
                (*method).clone(),
                overrides.clone(),
                response_overrides.clone(),
            ));
        }
    }

    // Build route documentation
    let router_doc = if route_docs.is_empty() {
        "Create an axum Router for this service.".to_string()
    } else {
        format!(
            "Create an axum Router for this service.\n\n# Routes\n\n{}",
            route_docs.join("\n")
        )
    };

    // Generate OpenAPI paths method (always available for composition)
    let openapi_paths_fn =
        crate::openapi_gen::generate_openapi_paths(&prefix, &openapi_methods, has_qualified)?;
    let openapi_paths_doc = format!(
        "Get OpenAPI paths for this service ({} route{}).",
        route_docs.len(),
        if route_docs.len() == 1 { "" } else { "s" }
    );
    let openapi_paths_method = quote! {
        #[doc = #openapi_paths_doc]
        pub fn http_openapi_paths() -> ::std::vec::Vec<::server_less::OpenApiPath> {
            let mut paths = #openapi_paths_fn;
            #(#mount_openapi_calls)*
            paths
        }
    };

    // Conditionally generate OpenAPI spec method.
    // Builds from http_openapi_paths() so mounted child paths are automatically included.
    let openapi_method = if generate_openapi {
        let openapi_doc = "Get OpenAPI 3.0 specification for this service.\n\n\
             Includes all paths (own + mounted children). Use `http_openapi_paths()` for composable path fragments.";
        quote! {
            #[doc = #openapi_doc]
            pub fn openapi_spec() -> ::server_less::serde_json::Value {
                let mut paths = ::server_less::serde_json::Map::new();
                for path_info in Self::http_openapi_paths() {
                    let path_item = paths.entry(path_info.path.clone())
                        .or_insert_with(|| ::server_less::serde_json::json!({}));
                    if let Some(map) = path_item.as_object_mut() {
                        let op = ::server_less::serde_json::to_value(&path_info.operation)
                            .expect("BUG: OpenApiOperation must be serializable");
                        map.insert(path_info.method.clone(), op);
                    }
                }
                ::server_less::serde_json::json!({
                    "openapi": "3.0.0",
                    "info": {
                        "title": #openapi_title,
                        "version": #openapi_version
                        #openapi_description_entry
                        #openapi_contact_entry
                    },
                    "paths": paths
                })
            }
        }
    } else {
        quote! {}
    };

    let clean_impl = if crate::is_protocol_impl_emitter(&impl_block, "http") {
        let stripped = strip_http_attrs(&impl_block);
        quote! { #stripped }
    } else {
        quote! {}
    };

    Ok(quote! {
        #clean_impl

        impl #impl_generics ::server_less::HttpMount for #self_ty #where_clause {
            fn http_mount_router(self: ::std::sync::Arc<Self>) -> ::server_less::axum::Router {
                use ::server_less::axum::routing::{get, post, put, patch, delete};

                let state = self;
                ::server_less::axum::Router::new()
                    #(#routes)*
                    #(#mount_routes)*
                    .with_state(state)
            }

            fn http_mount_openapi_paths() -> Vec<::server_less::OpenApiPath> {
                Self::http_openapi_paths()
            }
        }

        impl #impl_generics #self_ty #where_clause {
            #[doc = #router_doc]
            pub fn http_router(self) -> ::server_less::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                use ::server_less::axum::routing::{get, post, put, patch, delete};

                let state = ::std::sync::Arc::new(self);
                ::server_less::axum::Router::new()
                    #(#routes)*
                    #(#mount_routes)*
                    .with_state(state)
            }

            #openapi_paths_method

            #openapi_method
        }

        #(#handlers)*
    })
}

fn generate_handler(
    struct_name: &syn::Ident,
    self_ty: &syn::Type,
    method: &MethodInfo,
    response_overrides: &ResponseOverride,
    has_qualified: bool,
    debug: bool,
    trace: bool,
) -> syn::Result<TokenStream2> {
    let method_name = &method.name;
    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__server_less_http_{}_{}", struct_name_snake, method_name);
    let method_name_str = method_name.to_string();

    let (param_extractions, param_calls, param_names) =
        generate_param_handling(method, has_qualified)?;

    // When tracing is enabled, bind each user-visible parameter to a named local variable
    // (`__sl_param_{name}`) so we can log its value immediately after extraction.
    let (call, param_trace_stmts) = if trace {
        let mut trace_stmts: Vec<proc_macro2::TokenStream> = Vec::new();
        let mut bound_calls: Vec<proc_macro2::TokenStream> = Vec::new();

        for (call_expr, maybe_name) in param_calls.iter().zip(param_names.iter()) {
            if let Some(name) = maybe_name {
                let var_ident = format_ident!("__sl_param_{}", name);
                let name_str = name.as_str();
                trace_stmts.push(quote! {
                    let #var_ident = #call_expr;
                    eprintln!("[server-less] trace: param `{}` = {:?}", #name_str, #var_ident);
                });
                bound_calls.push(quote! { #var_ident });
            } else {
                // Context or other injected params: use inline expression, no trace line.
                bound_calls.push(call_expr.clone());
            }
        }

        let method_call = if method.is_async {
            quote! { state.#method_name(#(#bound_calls),*).await }
        } else {
            quote! { state.#method_name(#(#bound_calls),*) }
        };

        (method_call, trace_stmts)
    } else {
        let method_call = if method.is_async {
            quote! { state.#method_name(#(#param_calls),*).await }
        } else {
            quote! { state.#method_name(#(#param_calls),*) }
        };
        (method_call, Vec::new())
    };

    let response = generate_response_handling(method, &call, response_overrides)?;

    let handler = if debug {
        quote! {
            async fn #handler_name(
                state_extractor: ::server_less::axum::extract::State<::std::sync::Arc<#self_ty>>,
                #(#param_extractions),*
            ) -> impl ::server_less::axum::response::IntoResponse {
                let state = state_extractor.0;
                eprintln!("[server-less] {} called", #method_name_str);
                #(#param_trace_stmts)*
                let __sl_response = #response;
                eprintln!("[server-less] {} returned", #method_name_str);
                __sl_response
            }
        }
    } else if trace {
        quote! {
            async fn #handler_name(
                state_extractor: ::server_less::axum::extract::State<::std::sync::Arc<#self_ty>>,
                #(#param_extractions),*
            ) -> impl ::server_less::axum::response::IntoResponse {
                let state = state_extractor.0;
                #(#param_trace_stmts)*
                #response
            }
        }
    } else {
        quote! {
            async fn #handler_name(
                state_extractor: ::server_less::axum::extract::State<::std::sync::Arc<#self_ty>>,
                #(#param_extractions),*
            ) -> impl ::server_less::axum::response::IntoResponse {
                let state = state_extractor.0;
                #response
            }
        }
    };

    Ok(handler)
}

/// Returns `true` if the method has `#[http(debug = true)]` on it directly.
fn has_http_debug(method: &MethodInfo) -> bool {
    for attr in &method.method.attrs {
        if attr.path().is_ident("http") {
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("debug") {
                    if meta.input.peek(syn::Token![=]) {
                        let value: syn::LitBool = meta.value()?.parse()?;
                        if value.value() {
                            found = true;
                        }
                    } else {
                        found = true;
                    }
                } else if meta.input.peek(syn::Token![=]) {
                    // Consume other key = value pairs to avoid parse errors.
                    let _: proc_macro2::TokenStream = meta.value()?.parse()?;
                }
                Ok(())
            });
            if found {
                return true;
            }
        }
    }
    false
}

/// Returns `true` if the method has `#[http(trace = true)]` on it directly.
fn has_http_trace(method: &MethodInfo) -> bool {
    for attr in &method.method.attrs {
        if attr.path().is_ident("http") {
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("trace") {
                    if meta.input.peek(syn::Token![=]) {
                        let value: syn::LitBool = meta.value()?.parse()?;
                        if value.value() {
                            found = true;
                        }
                    } else {
                        found = true;
                    }
                } else if meta.input.peek(syn::Token![=]) {
                    // Consume other key = value pairs to avoid parse errors.
                    let _: proc_macro2::TokenStream = meta.value()?.parse()?;
                }
                Ok(())
            });
            if found {
                return true;
            }
        }
    }
    false
}

#[allow(clippy::type_complexity)]
fn generate_param_handling(
    method: &MethodInfo,
    has_qualified: bool,
) -> syn::Result<(Vec<TokenStream2>, Vec<TokenStream2>, Vec<Option<String>>)> {
    use server_less_parse::ParamLocation;

    let mut extractions = Vec::new();
    let mut calls = Vec::new();
    // Parallel to `calls`: None for injected params (Context), Some(name) for user-visible params.
    let mut param_names: Vec<Option<String>> = Vec::new();

    let http_method = infer_http_method(&method.name_str());
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
        param_names.push(None); // Context is injected; not user-visible for tracing
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
                path_extractor: ::server_less::axum::extract::Path<#ty>
            });
            calls.push(quote! { path_extractor.0 });
            param_names.push(Some(param.name_str()));
        }
    }

    // Generate body parameter extraction
    if !body_params.is_empty() {
        extractions.push(quote! {
            body_extractor: ::server_less::axum::extract::Json<::server_less::serde_json::Value>
        });

        for param in &body_params {
            // Use wire_name if provided, otherwise use the parameter name
            let name_str = param
                .wire_name
                .clone()
                .unwrap_or_else(|| param.name_str());
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
            param_names.push(Some(param.name_str()));
        }
    }

    // Generate query parameter extraction
    if !query_params.is_empty() {
        extractions.push(quote! {
            query_extractor: ::server_less::axum::extract::Query<::std::collections::HashMap<String, String>>
        });

        for param in &query_params {
            // Use wire_name if provided, otherwise use the parameter name
            let name_str = param
                .wire_name
                .clone()
                .unwrap_or_else(|| param.name_str());
            let ty = &param.ty;

            // Handle default values
            if param.is_optional {
                let inner_ty = extract_option_inner(ty).unwrap_or_else(|| ty.clone());
                calls.push(quote! {
                    query_extractor.0.get(#name_str).and_then(|v| v.parse::<#inner_ty>().ok())
                });
            } else if let Some(ref default_val) = param.default_value {
                // Parse the default value at compile time
                let default_expr: proc_macro2::TokenStream = default_val.parse().map_err(|_| {
                    syn::Error::new(
                        method.name.span(),
                        format!(
                            "failed to parse default value `{}` as a Rust expression\n\
                                 \n\
                                 Hint: Default values must be valid Rust expressions, e.g., \
                                 #[param(default = 0)] or #[param(default = \"hello\")]",
                            default_val
                        ),
                    )
                })?;
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
            param_names.push(Some(param.name_str()));
        }
    }

    // Generate header parameter extraction
    if !header_params.is_empty() {
        extractions.push(quote! {
            headers: ::server_less::axum::http::HeaderMap
        });

        for param in &header_params {
            // Use wire_name if provided, otherwise use the parameter name
            let name_str = param
                .wire_name
                .clone()
                .unwrap_or_else(|| param.name_str());
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
            param_names.push(Some(param.name_str()));
        }
    }

    Ok((extractions, calls, param_names))
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
                ::server_less::axum::http::StatusCode::NO_CONTENT
            }
        }
    } else if ret.is_result {
        quote! {
            {
                use ::server_less::axum::response::IntoResponse;
                use ::server_less::HttpStatusFallback as _;
                match #call {
                    Ok(value) => ::server_less::axum::Json(value).into_response(),
                    Err(err) => {
                        let status_u16 = ::server_less::HttpStatusHelper(&err).http_status_code();
                        let status = ::server_less::axum::http::StatusCode::from_u16(status_u16)
                            .unwrap_or(::server_less::axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                        let body = ::server_less::serde_json::json!({
                            "error": format!("{:?}", err),
                            "message": format!("{}", err)
                        });
                        (status, ::server_less::axum::Json(body)).into_response()
                    }
                }
            }
        }
    } else if ret.is_option {
        quote! {
            {
                use ::server_less::axum::response::IntoResponse;
                match #call {
                    Some(value) => ::server_less::axum::Json(value).into_response(),
                    None => ::server_less::axum::http::StatusCode::NOT_FOUND.into_response(),
                }
            }
        }
    } else if ret.is_iterator {
        quote! {
            {
                use ::server_less::futures::StreamExt;
                let iter = #call;
                let stream = ::server_less::futures::stream::iter(iter);
                let boxed_stream = Box::pin(stream);
                ::server_less::axum::response::sse::Sse::new(
                    boxed_stream.map(|item| {
                        Ok::<_, std::convert::Infallible>(
                            ::server_less::axum::response::sse::Event::default()
                                .json_data(item)
                                .expect("BUG: failed to serialize SSE event as JSON — Iterator item type must implement serde::Serialize")
                        )
                    })
                )
            }
        }
    } else if ret.is_stream {
        quote! {
            {
                use ::server_less::futures::StreamExt;
                let stream = #call;
                let boxed_stream = Box::pin(stream);
                ::server_less::axum::response::sse::Sse::new(
                    boxed_stream.map(|item| {
                        Ok::<_, std::convert::Infallible>(
                            ::server_less::axum::response::sse::Event::default()
                                .json_data(item)
                                .expect("BUG: failed to serialize SSE event as JSON — the Stream item type must implement serde::Serialize")
                        )
                    })
                )
            }
        }
    } else {
        quote! {
            {
                let result = #call;
                ::server_less::axum::Json(result)
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
            ::server_less::axum::http::StatusCode::from_u16(#status)
                .unwrap_or(::server_less::axum::http::StatusCode::OK)
        }
    } else {
        quote! { ::server_less::axum::http::StatusCode::OK }
    };

    let header_insertions: Vec<TokenStream2> = overrides
        .headers
        .iter()
        .map(|(name, value)| {
            quote! {
                headers.insert(
                    ::server_less::axum::http::header::HeaderName::from_static(#name),
                    ::server_less::axum::http::header::HeaderValue::from_static(#value)
                );
            }
        })
        .collect();

    let content_type_insertion = if let Some(ref ct) = overrides.content_type {
        quote! {
            headers.insert(
                ::server_less::axum::http::header::CONTENT_TYPE,
                ::server_less::axum::http::header::HeaderValue::from_static(#ct)
            );
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        {
            use ::server_less::axum::response::IntoResponse;
            let base_response = #base_response;
            let mut headers = ::server_less::axum::http::HeaderMap::new();
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
    let handler_name = format_ident!("__server_less_http_{}_{}", struct_name_snake, method_name);

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
        validate_http_path(p, method_name.span())?;
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

/// Validate HTTP path at compile time
fn validate_http_path(path: &str, method_span: proc_macro2::Span) -> syn::Result<()> {
    // Check that path starts with /
    if !path.starts_with('/') {
        return Err(syn::Error::new(
            method_span,
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
            method_span,
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
            method_span,
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
            method_span,
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
            method_span,
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
                    method_span,
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
                    method_span,
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
                    method_span,
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
                method_span,
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
    /// Protocols to serve (http, ws, jsonrpc, graphql)
    pub protocols: Vec<String>,
    /// Health check path (default: /health)
    pub health_path: Option<String>,
    /// OpenAPI spec generation (default: true when protocols are present)
    /// Set to false with `openapi = false`
    pub openapi: Option<bool>,
    /// Application name (used as OpenAPI info.title, overrides struct name)
    pub name: Option<String>,
    /// Human-readable description (used as OpenAPI info.description)
    pub description: Option<String>,
    /// Application version (used as OpenAPI info.version, defaults to CARGO_PKG_VERSION)
    pub version: Option<String>,
    /// Homepage URL (used as OpenAPI info.contact.url)
    pub homepage: Option<String>,
}

impl ServeArgs {
    /// Whether OpenAPI spec should be generated.
    /// Default: true (opt-out with `openapi = false`)
    pub fn openapi_enabled(&self) -> bool {
        self.openapi.unwrap_or(true)
    }
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
                "openapi" => {
                    if input.peek(Token![=]) {
                        input.parse::<Token![=]>()?;
                        let lit: syn::LitBool = input.parse()?;
                        args.openapi = Some(lit.value());
                    } else {
                        // Bare `openapi` means enable
                        args.openapi = Some(true);
                    }
                }
                "name" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitStr = input.parse()?;
                    args.name = Some(lit.value());
                }
                "description" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitStr = input.parse()?;
                    args.description = Some(lit.value());
                }
                "version" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                "homepage" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitStr = input.parse()?;
                    args.homepage = Some(lit.value());
                }
                other => {
                    const VALID: &[&str] = &[
                        "http", "ws", "jsonrpc", "graphql", "health", "openapi",
                        "name", "description", "version", "homepage",
                    ];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}\n\
                             \n\
                             Valid protocols: http, ws, jsonrpc, graphql\n\
                             Valid options: health, openapi, name, description, version, homepage\n\
                             \n\
                             Examples:\n\
                             - #[serve(http, ws, health = \"/status\")]\n\
                             - #[serve(http, openapi = false)]\n\
                             - #[serve(http, name = \"My API\", description = \"Does the thing\")]"
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
    let (impl_generics, _ty_generics, where_clause) = impl_block.generics.split_for_impl();
    let self_ty = &impl_block.self_ty;

    let openapi_enabled = args.openapi_enabled();
    let health_path = args.health_path.unwrap_or_else(|| "/health".to_string());
    let serve_title = args.name.unwrap_or_else(|| struct_name.to_string());
    let serve_version = match args.version {
        Some(ref v) => quote! { #v },
        None => quote! { ::std::env!("CARGO_PKG_VERSION") },
    };

    // Build router combination based on protocols
    let router_setup = generate_router_setup(&args.protocols);

    // Generate OpenAPI spec method and route if enabled
    let (openapi_spec_method, openapi_route) = if openapi_enabled {
        let openapi_paths_merges = generate_openapi_merges(&args.protocols);

        let method = quote! {
            /// Get the combined OpenAPI spec for all configured protocols.
            ///
            /// Merges paths from HTTP, JSON-RPC, GraphQL, and/or WebSocket
            /// into a single OpenAPI 3.0 spec using OpenApiBuilder.
            ///
            /// Disable with `#[serve(http, openapi = false)]`.
            pub fn combined_openapi_spec() -> ::server_less::serde_json::Value {
                ::server_less::OpenApiBuilder::new()
                    .title(#serve_title)
                    .version(#serve_version)
                    #openapi_paths_merges
                    .build()
            }
        };

        let route = quote! {
            let router = router.route(
                "/openapi.json",
                ::server_less::axum::routing::get(|| async {
                    ::server_less::axum::Json(#struct_name::combined_openapi_spec())
                })
            );
        };

        (method, route)
    } else {
        (quote! {}, quote! {})
    };

    // Generate the serve method
    let serve_impl = quote! {
        impl #impl_generics #self_ty #where_clause {
            /// Start serving all configured protocols.
            pub async fn serve(self, addr: impl ::std::convert::AsRef<str>) -> ::std::io::Result<()>
            where
                Self: Clone + Send + Sync + 'static,
            {
                #router_setup

                // Add health check
                let router = router.route(
                    #health_path,
                    ::server_less::axum::routing::get(|| async { "ok" })
                );

                // Add OpenAPI spec endpoint
                #openapi_route

                let listener = ::server_less::tokio::net::TcpListener::bind(addr.as_ref()).await?;
                ::server_less::axum::serve(listener, router).await
            }

            /// Build the combined router without starting the server.
            pub fn router(self) -> ::server_less::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                #router_setup

                let router = router.route(
                    #health_path,
                    ::server_less::axum::routing::get(|| async { "ok" })
                );

                // Add OpenAPI spec endpoint
                #openapi_route

                router
            }

            #openapi_spec_method
        }
    };

    Ok(quote! {
        #impl_block

        #serve_impl
    })
}

/// Generate OpenAPI merge calls for each enabled protocol.
fn generate_openapi_merges(protocols: &[String]) -> TokenStream2 {
    let has_http = protocols.contains(&"http".to_string());
    let has_ws = protocols.contains(&"ws".to_string());
    let has_jsonrpc = protocols.contains(&"jsonrpc".to_string());
    let has_graphql = protocols.contains(&"graphql".to_string());

    let mut merges = Vec::new();

    if has_http {
        merges.push(quote! {
            .merge_paths(Self::http_openapi_paths())
        });
    }
    if has_jsonrpc {
        merges.push(quote! {
            .merge_paths(Self::jsonrpc_openapi_paths())
        });
    }
    if has_graphql {
        merges.push(quote! {
            .merge_paths(Self::graphql_openapi_paths())
        });
    }
    if has_ws {
        merges.push(quote! {
            .merge_paths(Self::ws_openapi_paths())
        });
    }

    quote! { #(#merges)* }
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
            let router = ::server_less::axum::Router::new();
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
