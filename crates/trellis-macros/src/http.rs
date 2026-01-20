//! HTTP handler generation.

use heck::ToKebabCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parse, GenericArgument, ItemImpl, PathArguments, Token, Type};

use crate::parse::{extract_methods, get_impl_name, MethodInfo, ParamInfo};
use crate::rpc;

/// Extract the inner type T from Option<T>
fn extract_option_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
    }
    None
}

/// Per-method HTTP attribute overrides
#[derive(Default, Clone)]
pub struct HttpMethodOverride {
    /// Override HTTP method (GET, POST, etc.)
    pub method: Option<String>,
    /// Override path
    pub path: Option<String>,
    /// Skip this method (don't generate HTTP handler)
    pub skip: bool,
    /// Hide from OpenAPI spec
    pub hidden: bool,
}

impl HttpMethodOverride {
    /// Parse #[route(...)] attributes from a method
    ///
    /// Note: We use `#[route(...)]` instead of `#[http(...)]` for method-level
    /// attributes to avoid conflict with the impl-level `#[http]` macro.
    fn parse_from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("route") {
                continue;
            }

            // Handle #[route(skip)] style
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
pub struct HttpArgs {
    /// Base path prefix (e.g., "/api/v1")
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
                        format!("unknown argument `{other}`. Valid arguments: prefix"),
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

/// Expand the #[http] attribute macro
pub fn expand_http(args: HttpArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    let prefix = args.prefix.unwrap_or_default();

    // Generate handler functions and route registrations
    let mut handlers = Vec::new();
    let mut routes = Vec::new();
    let mut openapi_methods = Vec::new();

    for method in &methods {
        // Parse per-method overrides
        let overrides = HttpMethodOverride::parse_from_attrs(&method.method.attrs)?;

        // Skip methods marked with #[http(skip)]
        if overrides.skip {
            continue;
        }

        let handler = generate_handler(&struct_name, method)?;
        handlers.push(handler);

        let route = generate_route(&prefix, method, &overrides, &struct_name)?;
        routes.push(route);

        // Track methods for OpenAPI (unless hidden)
        if !overrides.hidden {
            openapi_methods.push((method.clone(), overrides.clone()));
        }
    }

    // Generate the router function
    let _router_fn = generate_router(&struct_name, &routes);

    // Generate OpenAPI spec function (only non-hidden methods)
    let openapi_fn = generate_openapi_spec(&struct_name, &prefix, &openapi_methods)?;

    Ok(quote! {
        #impl_block

        // Generated HTTP handlers
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
            pub fn openapi_spec() -> ::trellis::serde_json::Value {
                #openapi_fn
            }
        }

        #(#handlers)*
    })
}

/// Generate a handler function for a method
fn generate_handler(struct_name: &syn::Ident, method: &MethodInfo) -> syn::Result<TokenStream> {
    let method_name = &method.name;
    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_http_{}_{}", struct_name_snake, method_name);

    // Determine parameter extraction
    let (param_extractions, param_calls) = generate_param_handling(method)?;

    // Determine how to call the method and handle the response
    let call = if method.is_async {
        quote! { state.#method_name(#(#param_calls),*).await }
    } else {
        quote! { state.#method_name(#(#param_calls),*) }
    };

    // Generate response handling based on return type
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

/// Generate parameter extraction and call arguments
fn generate_param_handling(method: &MethodInfo) -> syn::Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut extractions = Vec::new();
    let mut calls = Vec::new();

    let http_method = infer_http_method(&method.name.to_string());
    let has_body = matches!(http_method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch);

    // Collect ID params (path params) and other params
    let id_params: Vec<_> = method.params.iter().filter(|p| p.is_id).collect();
    let other_params: Vec<_> = method.params.iter().filter(|p| !p.is_id).collect();

    // Path parameters
    if !id_params.is_empty() {
        // For now, assume a single ID param named "id" in the path
        for param in &id_params {
            let _name = &param.name;
            let ty = &param.ty;
            extractions.push(quote! {
                path_extractor: ::axum::extract::Path<#ty>
            });
            calls.push(quote! { path_extractor.0 });
        }
    }

    // Query vs Body params
    if !other_params.is_empty() {
        if has_body {
            // POST/PUT/PATCH: use JSON body
            extractions.push(quote! {
                body_extractor: ::axum::extract::Json<::trellis::serde_json::Value>
            });

            for param in &other_params {
                let _name = &param.name;
                let name_str = param.name.to_string();
                let ty = &param.ty;
                if param.is_optional {
                    // Extract inner type from Option<T> to deserialize as T, result wrapped in Option
                    let inner_ty = extract_option_inner(ty).unwrap_or_else(|| ty.clone());
                    calls.push(quote! {
                        body_extractor.0.get(#name_str).and_then(|v| ::trellis::serde_json::from_value::<#inner_ty>(v.clone()).ok())
                    });
                } else {
                    calls.push(quote! {
                        ::trellis::serde_json::from_value::<#ty>(body_extractor.0.get(#name_str).cloned().unwrap_or_default()).unwrap_or_default()
                    });
                }
            }
        } else {
            // GET/DELETE: use query parameters
            extractions.push(quote! {
                query_extractor: ::axum::extract::Query<::std::collections::HashMap<String, String>>
            });

            for param in &other_params {
                let name_str = param.name.to_string();
                let ty = &param.ty;
                if param.is_optional {
                    // Extract inner type from Option<T> to parse as T, result wrapped in Option
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

/// Generate response handling based on return type
fn generate_response_handling(method: &MethodInfo, call: &TokenStream) -> syn::Result<TokenStream> {
    let ret = &method.return_info;

    if ret.is_unit {
        // Returns () -> 204 No Content
        Ok(quote! {
            #call;
            ::axum::http::StatusCode::NO_CONTENT
        })
    } else if ret.is_result {
        // Returns Result<T, E> -> 200 or error
        Ok(quote! {
            use ::axum::response::IntoResponse;
            match #call {
                Ok(value) => ::axum::Json(value).into_response(),
                Err(err) => {
                    let code = ::trellis::ErrorCode::infer_from_name(&format!("{:?}", err));
                    let status = ::axum::http::StatusCode::from_u16(code.http_status())
                        .unwrap_or(::axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                    let body = ::trellis::serde_json::json!({
                        "error": format!("{:?}", err),
                        "message": format!("{}", err)
                    });
                    (status, ::axum::Json(body)).into_response()
                }
            }
        })
    } else if ret.is_option {
        // Returns Option<T> -> 200 or 404
        Ok(quote! {
            use ::axum::response::IntoResponse;
            match #call {
                Some(value) => ::axum::Json(value).into_response(),
                None => ::axum::http::StatusCode::NOT_FOUND.into_response(),
            }
        })
    } else if ret.is_stream {
        // Returns impl Stream<Item=T> -> SSE
        // Box::pin converts the stream to Pin<Box<dyn Stream>> which:
        // 1. Erases the concrete type, allowing different stream impls
        // 2. Makes the stream 'static by owning the data
        // Note: In Rust 2024, users may need `+ use<>` on their method
        // return types to avoid implicit lifetime capture issues.
        Ok(quote! {
            use ::trellis::futures::StreamExt;
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
        // Returns T -> 200 with JSON
        Ok(quote! {
            let result = #call;
            ::axum::Json(result)
        })
    }
}

/// Generate route registration
fn generate_route(prefix: &str, method: &MethodInfo, overrides: &HttpMethodOverride, struct_name: &syn::Ident) -> syn::Result<TokenStream> {
    let method_name = &method.name;
    let struct_name_snake = struct_name.to_string().to_lowercase();
    let handler_name = format_ident!("__trellis_http_{}_{}", struct_name_snake, method_name);

    // Use override or infer HTTP method
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

    // Use override or infer path
    let path = if let Some(ref p) = overrides.path {
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

/// Generate router function (placeholder - routes added in expand_http)
fn generate_router(_struct_name: &syn::Ident, _routes: &[TokenStream]) -> TokenStream {
    quote! {}
}

/// Generate OpenAPI spec
fn generate_openapi_spec(
    struct_name: &syn::Ident,
    prefix: &str,
    methods_with_overrides: &[(MethodInfo, HttpMethodOverride)],
) -> syn::Result<TokenStream> {
    let mut operation_data = Vec::new();

    for (method, overrides) in methods_with_overrides {
        let method_name = method.name.to_string();

        // Use override or infer HTTP method
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

        // Use override or infer path
        let path = if let Some(ref p) = overrides.path {
            p.clone()
        } else {
            infer_path(&method_name, &http_method, &method.params)
        };
        let full_path = format!("{}{}", prefix, path);
        let http_method_str = http_method.as_str().to_lowercase();

        let summary = method.docs.clone().unwrap_or_else(|| method_name.clone());
        let operation_id = method_name.clone();

        // Generate parameter specs
        let has_body = matches!(http_method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch);
        let id_params: Vec<_> = method.params.iter().filter(|p| p.is_id).collect();
        let other_params: Vec<_> = method.params.iter().filter(|p| !p.is_id).collect();

        // Path parameters
        let path_param_specs: Vec<_> = id_params.iter().map(|p| {
            let name = p.name.to_string();
            let json_type = rpc::infer_json_type(&p.ty);
            quote! { (#name, "path", #json_type, true) }
        }).collect();

        // Query parameters (for GET/DELETE) or body schema (for POST/PUT/PATCH)
        let query_param_specs: Vec<TokenStream> = if !has_body {
            other_params.iter().map(|p| {
                let name = p.name.to_string();
                let json_type = rpc::infer_json_type(&p.ty);
                let required = !p.is_optional;
                quote! { (#name, "query", #json_type, #required) }
            }).collect()
        } else {
            vec![]
        };

        // Request body properties for POST/PUT/PATCH
        let body_props: Vec<TokenStream> = if has_body {
            other_params.iter().map(|p| {
                let name = p.name.to_string();
                let json_type = rpc::infer_json_type(&p.ty);
                let required = !p.is_optional;
                quote! { (#name, #json_type, #required) }
            }).collect()
        } else {
            vec![]
        };
        let has_body_props = !body_props.is_empty();

        // Response type info
        let ret = &method.return_info;
        let (success_code, error_responses) = if ret.is_result {
            ("200", true)
        } else if ret.is_option {
            ("200", false) // 404 handled implicitly
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

                // Build parameters array
                let mut parameters: Vec<::trellis::serde_json::Value> = Vec::new();
                #(
                    {
                        let (name, location, schema_type, required): (&str, &str, &str, bool) = #path_param_specs;
                        parameters.push(::trellis::serde_json::json!({
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
                        parameters.push(::trellis::serde_json::json!({
                            "name": name,
                            "in": location,
                            "required": required,
                            "schema": { "type": schema_type }
                        }));
                    }
                )*

                // Build request body if needed
                let request_body: Option<::trellis::serde_json::Value> = if has_body {
                    let mut properties = ::trellis::serde_json::Map::new();
                    let mut required_props: Vec<String> = Vec::new();
                    #(
                        {
                            let (name, schema_type, required): (&str, &str, bool) = #body_props;
                            properties.insert(name.to_string(), ::trellis::serde_json::json!({
                                "type": schema_type
                            }));
                            if required {
                                required_props.push(name.to_string());
                            }
                        }
                    )*
                    Some(::trellis::serde_json::json!({
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

                // Build responses
                let mut responses = ::trellis::serde_json::Map::new();
                responses.insert(success_code.to_string(), ::trellis::serde_json::json!({
                    "description": "Successful response"
                }));
                if has_error_responses {
                    responses.insert("400".to_string(), ::trellis::serde_json::json!({
                        "description": "Bad request"
                    }));
                    responses.insert("500".to_string(), ::trellis::serde_json::json!({
                        "description": "Internal server error"
                    }));
                }

                // Build operation object
                let mut operation = ::trellis::serde_json::json!({
                    "summary": summary,
                    "operationId": operation_id,
                    "responses": responses
                });

                if !parameters.is_empty() {
                    operation.as_object_mut().unwrap()
                        .insert("parameters".to_string(), ::trellis::serde_json::Value::Array(parameters));
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
            let mut paths = ::trellis::serde_json::Map::new();

            #(
                {
                    let (path, method, operation): (String, String, ::trellis::serde_json::Value) = #operation_data;
                    let path_item = paths.entry(path)
                        .or_insert_with(|| ::trellis::serde_json::json!({}));
                    if let ::trellis::serde_json::Value::Object(map) = path_item {
                        map.insert(method, operation);
                    }
                }
            )*

            ::trellis::serde_json::json!({
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

// Helper types and functions

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
        HttpMethod::Post // RPC fallback
    }
}

fn infer_path(method_name: &str, http_method: &HttpMethod, params: &[ParamInfo]) -> String {
    // Extract resource name from method name
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

    // Convert to kebab-case and pluralize
    let resource_kebab = resource.to_kebab_case();
    let path_resource = if resource_kebab.ends_with('s') {
        resource_kebab
    } else {
        format!("{}s", resource_kebab)
    };

    // Check if we have ID parameters
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
