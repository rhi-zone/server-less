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
//! # Generated Methods
//!
//! - `http_router() -> axum::Router` - Complete router with all endpoints
//! - `http_routes() -> Vec<&'static str>` - List of route paths
//!
//! # Example
//!
//! ```ignore
//! use rhizome_trellis::http;
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
use rhizome_trellis_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};
use syn::{GenericArgument, ItemImpl, PathArguments, Token, Type, parse::Parse};

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
pub(crate) struct HttpMethodOverride {
    pub method: Option<String>,
    pub path: Option<String>,
    pub skip: bool,
    pub hidden: bool,
}

impl HttpMethodOverride {
    fn parse_from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
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

/// Arguments for the #[http] attribute
#[derive(Default)]
pub(crate) struct HttpArgs {
    pub prefix: Option<String>,
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
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`\n\
                             Valid arguments: prefix\n\
                             Example: #[http(prefix = \"/api/v1\")]"
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

    let prefix = args.prefix.unwrap_or_default();

    let mut handlers = Vec::new();
    let mut routes = Vec::new();
    let mut openapi_methods = Vec::new();
    let mut route_signatures = std::collections::HashMap::new();

    for method in &methods {
        let overrides = HttpMethodOverride::parse_from_attrs(&method.method.attrs)?;

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
        let route_sig = format!("{} {}", http_method_enum.as_str(), full_path);

        if let Some(existing_method) = route_signatures.get(&route_sig) {
            return Err(syn::Error::new_spanned(
                &method.method.sig,
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
                ),
            ));
        }
        route_signatures.insert(route_sig, method.name.to_string());

        let handler = generate_handler(&struct_name, method)?;
        handlers.push(handler);

        let route = generate_route(&prefix, method, &overrides, &struct_name)?;
        routes.push(route);

        if !overrides.hidden {
            openapi_methods.push((method.clone(), overrides.clone()));
        }
    }

    let openapi_fn = generate_openapi_spec(&struct_name, &prefix, &openapi_methods)?;

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

            /// Get OpenAPI specification for this service
            pub fn openapi_spec() -> ::rhizome_trellis::serde_json::Value {
                #openapi_fn
            }
        }

        #(#handlers)*
    })
}

fn generate_handler(struct_name: &syn::Ident, method: &MethodInfo) -> syn::Result<TokenStream2> {
    let method_name = &method.name;
    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_http_{}_{}", struct_name_snake, method_name);

    let (param_extractions, param_calls) = generate_param_handling(method)?;

    let call = if method.is_async {
        quote! { state.#method_name(#(#param_calls),*).await }
    } else {
        quote! { state.#method_name(#(#param_calls),*) }
    };

    let response = generate_response_handling(method, &call)?;

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
) -> syn::Result<(Vec<TokenStream2>, Vec<TokenStream2>)> {
    let mut extractions = Vec::new();
    let mut calls = Vec::new();

    let http_method = infer_http_method(&method.name.to_string());
    let has_body = matches!(
        http_method,
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch
    );

    let id_params: Vec<_> = method.params.iter().filter(|p| p.is_id).collect();
    let other_params: Vec<_> = method.params.iter().filter(|p| !p.is_id).collect();

    if !id_params.is_empty() {
        for param in &id_params {
            let ty = &param.ty;
            extractions.push(quote! {
                path_extractor: ::axum::extract::Path<#ty>
            });
            calls.push(quote! { path_extractor.0 });
        }
    }

    if !other_params.is_empty() {
        if has_body {
            extractions.push(quote! {
                body_extractor: ::axum::extract::Json<::rhizome_trellis::serde_json::Value>
            });

            for param in &other_params {
                let name_str = param.name.to_string();
                let ty = &param.ty;
                if param.is_optional {
                    let inner_ty = extract_option_inner(ty).unwrap_or_else(|| ty.clone());
                    calls.push(quote! {
                        body_extractor.0.get(#name_str).and_then(|v| ::rhizome_trellis::serde_json::from_value::<#inner_ty>(v.clone()).ok())
                    });
                } else {
                    calls.push(quote! {
                        ::rhizome_trellis::serde_json::from_value::<#ty>(body_extractor.0.get(#name_str).cloned().unwrap_or_default()).unwrap_or_default()
                    });
                }
            }
        } else {
            extractions.push(quote! {
                query_extractor: ::axum::extract::Query<::std::collections::HashMap<String, String>>
            });

            for param in &other_params {
                let name_str = param.name.to_string();
                let ty = &param.ty;
                if param.is_optional {
                    let inner_ty = extract_option_inner(ty).unwrap_or_else(|| ty.clone());
                    calls.push(quote! {
                        query_extractor.0.get(#name_str).and_then(|v| v.parse::<#inner_ty>().ok())
                    });
                } else {
                    calls.push(quote! {
                        query_extractor.0.get(#name_str).and_then(|v| v.parse::<#ty>().ok()).unwrap_or_default()
                    });
                }
            }
        }
    }

    Ok((extractions, calls))
}

fn generate_response_handling(
    method: &MethodInfo,
    call: &TokenStream2,
) -> syn::Result<TokenStream2> {
    let ret = &method.return_info;

    if ret.is_unit {
        Ok(quote! {
            #call;
            ::axum::http::StatusCode::NO_CONTENT
        })
    } else if ret.is_result {
        Ok(quote! {
            use ::axum::response::IntoResponse;
            match #call {
                Ok(value) => ::axum::Json(value).into_response(),
                Err(err) => {
                    let code = ::rhizome_trellis::ErrorCode::infer_from_name(&format!("{:?}", err));
                    let status = ::axum::http::StatusCode::from_u16(code.http_status())
                        .unwrap_or(::axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                    let body = ::rhizome_trellis::serde_json::json!({
                        "error": format!("{:?}", err),
                        "message": format!("{}", err)
                    });
                    (status, ::axum::Json(body)).into_response()
                }
            }
        })
    } else if ret.is_option {
        Ok(quote! {
            use ::axum::response::IntoResponse;
            match #call {
                Some(value) => ::axum::Json(value).into_response(),
                None => ::axum::http::StatusCode::NOT_FOUND.into_response(),
            }
        })
    } else if ret.is_stream {
        Ok(quote! {
            use ::rhizome_trellis::futures::StreamExt;
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
        })
    } else {
        Ok(quote! {
            let result = #call;
            ::axum::Json(result)
        })
    }
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

fn generate_openapi_spec(
    struct_name: &syn::Ident,
    prefix: &str,
    methods_with_overrides: &[(MethodInfo, HttpMethodOverride)],
) -> syn::Result<TokenStream2> {
    let mut operation_data = Vec::new();

    for (method, overrides) in methods_with_overrides {
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

        let has_body = matches!(
            http_method,
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch
        );
        let id_params: Vec<_> = method.params.iter().filter(|p| p.is_id).collect();
        let other_params: Vec<_> = method.params.iter().filter(|p| !p.is_id).collect();

        let path_param_specs: Vec<_> = id_params
            .iter()
            .map(|p| {
                let name = p.name.to_string();
                let json_type = rhizome_trellis_rpc::infer_json_type(&p.ty);
                quote! { (#name, "path", #json_type, true) }
            })
            .collect();

        let query_param_specs: Vec<TokenStream2> = if !has_body {
            other_params
                .iter()
                .map(|p| {
                    let name = p.name.to_string();
                    let json_type = rhizome_trellis_rpc::infer_json_type(&p.ty);
                    let required = !p.is_optional;
                    quote! { (#name, "query", #json_type, #required) }
                })
                .collect()
        } else {
            vec![]
        };

        let body_props: Vec<TokenStream2> = if has_body {
            other_params
                .iter()
                .map(|p| {
                    let name = p.name.to_string();
                    let json_type = rhizome_trellis_rpc::infer_json_type(&p.ty);
                    let required = !p.is_optional;
                    quote! { (#name, #json_type, #required) }
                })
                .collect()
        } else {
            vec![]
        };
        let has_body_props = !body_props.is_empty();

        let ret = &method.return_info;
        let (success_code, error_responses) = if ret.is_result {
            ("200", true)
        } else if ret.is_option {
            ("200", false)
        } else if ret.is_unit {
            ("204", false)
        } else {
            ("200", false)
        };

        operation_data.push(quote! {
            {
                let path = #full_path;
                let method = #http_method_str;
                let summary = #summary;
                let operation_id = #operation_id;
                let success_code = #success_code;
                let has_error_responses = #error_responses;
                let has_body = #has_body_props;

                let mut parameters: Vec<::rhizome_trellis::serde_json::Value> = Vec::new();
                #(
                    {
                        let (name, location, schema_type, required): (&str, &str, &str, bool) = #path_param_specs;
                        parameters.push(::rhizome_trellis::serde_json::json!({
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
                        parameters.push(::rhizome_trellis::serde_json::json!({
                            "name": name,
                            "in": location,
                            "required": required,
                            "schema": { "type": schema_type }
                        }));
                    }
                )*

                let request_body: Option<::rhizome_trellis::serde_json::Value> = if has_body {
                    let mut properties = ::rhizome_trellis::serde_json::Map::new();
                    let mut required_props: Vec<String> = Vec::new();
                    #(
                        {
                            let (name, schema_type, required): (&str, &str, bool) = #body_props;
                            properties.insert(name.to_string(), ::rhizome_trellis::serde_json::json!({
                                "type": schema_type
                            }));
                            if required {
                                required_props.push(name.to_string());
                            }
                        }
                    )*
                    Some(::rhizome_trellis::serde_json::json!({
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

                let mut responses = ::rhizome_trellis::serde_json::Map::new();
                responses.insert(success_code.to_string(), ::rhizome_trellis::serde_json::json!({
                    "description": "Successful response"
                }));
                if has_error_responses {
                    responses.insert("400".to_string(), ::rhizome_trellis::serde_json::json!({
                        "description": "Bad request"
                    }));
                    responses.insert("500".to_string(), ::rhizome_trellis::serde_json::json!({
                        "description": "Internal server error"
                    }));
                }

                let mut operation = ::rhizome_trellis::serde_json::json!({
                    "summary": summary,
                    "operationId": operation_id,
                    "responses": responses
                });

                if !parameters.is_empty() {
                    operation.as_object_mut().unwrap()
                        .insert("parameters".to_string(), ::rhizome_trellis::serde_json::Value::Array(parameters));
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
            let mut paths = ::rhizome_trellis::serde_json::Map::new();

            #(
                {
                    let (path, method, operation): (String, String, ::rhizome_trellis::serde_json::Value) = #operation_data;
                    let path_item = paths.entry(path)
                        .or_insert_with(|| ::rhizome_trellis::serde_json::json!({}));
                    if let ::rhizome_trellis::serde_json::Value::Object(map) = path_item {
                        map.insert(method, operation);
                    }
                }
            )*

            ::rhizome_trellis::serde_json::json!({
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

    // Check for invalid characters
    let invalid_chars = ['<', '>', '"', '`', ' ', '\t', '\n'];
    if let Some(ch) = invalid_chars.iter().find(|&&c| path.contains(c)) {
        let hint = if *ch == '<' || *ch == '>' {
            "\n\nHint: Use curly braces for path parameters, e.g., /users/{id}"
        } else if *ch == ' ' {
            "\n\nHint: Use hyphens or underscores instead of spaces, e.g., /my-resource"
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

    // Check that path parameters have names
    for segment in path.split('/') {
        if segment == "{}" || segment == "{" || segment == "}" {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "HTTP path has empty or malformed path parameter. Path: '{}'\n\
                     \n\
                     Hint: Path parameters need names, e.g., /users/{{id}} or /posts/{{post_id}}",
                    path
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
