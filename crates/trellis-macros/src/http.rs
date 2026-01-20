//! HTTP handler generation.

use heck::ToKebabCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parse, ItemImpl, Token};

use crate::parse::{extract_methods, get_impl_name, MethodInfo, ParamInfo};

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
                    return Err(syn::Error::new(ident.span(), format!("unknown argument: {other}")));
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

    for method in &methods {
        let handler = generate_handler(&struct_name, method)?;
        handlers.push(handler);

        let route = generate_route(&prefix, method)?;
        routes.push(route);
    }

    // Generate the router function
    let _router_fn = generate_router(&struct_name, &routes);

    // Generate OpenAPI spec function
    let openapi_fn = generate_openapi_spec(&struct_name, &prefix, &methods)?;

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
    let handler_name = format_ident!("__trellis_http_{}", method_name);

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
                    calls.push(quote! {
                        body_extractor.0.get(#name_str).and_then(|v| ::trellis::serde_json::from_value::<#ty>(v.clone()).ok())
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
                    calls.push(quote! {
                        query_extractor.0.get(#name_str).and_then(|v| v.parse::<#ty>().ok())
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
fn generate_route(prefix: &str, method: &MethodInfo) -> syn::Result<TokenStream> {
    let method_name = &method.name;
    let handler_name = format_ident!("__trellis_http_{}", method_name);

    let http_method = infer_http_method(&method_name.to_string());
    let path = infer_path(&method_name.to_string(), &http_method, &method.params);
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
    methods: &[MethodInfo],
) -> syn::Result<TokenStream> {
    let mut paths = Vec::new();

    for method in methods {
        let method_name = method.name.to_string();
        let http_method = infer_http_method(&method_name);
        let path = infer_path(&method_name, &http_method, &method.params);
        let full_path = format!("{}{}", prefix, path);
        let http_method_str = http_method.as_str().to_lowercase();

        let summary = method.docs.clone().unwrap_or_else(|| method_name.clone());
        let operation_id = method_name.clone();

        paths.push(quote! {
            (#full_path, #http_method_str, #summary, #operation_id)
        });
    }

    Ok(quote! {
        {
            let mut paths = ::trellis::serde_json::Map::new();

            #(
                {
                    let (path, method, summary, operation_id): (&str, &str, &str, &str) = #paths;
                    let path_item = paths.entry(path.to_string())
                        .or_insert_with(|| ::trellis::serde_json::json!({}));
                    if let ::trellis::serde_json::Value::Object(map) = path_item {
                        map.insert(method.to_string(), ::trellis::serde_json::json!({
                            "summary": summary,
                            "operationId": operation_id,
                            "responses": {
                                "200": {
                                    "description": "Successful response"
                                }
                            }
                        }));
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
