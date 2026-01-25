//! Shared OpenAPI generation utilities.
//!
//! This module contains pure functions and types for OpenAPI generation
//! that can be used independently of any HTTP runtime (like axum).
//!
//! Used by both `#[http]` and `#[openapi]` macros.

use heck::ToKebabCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, ParamInfo, ParamLocation};

use crate::context::should_inject_context;

/// HTTP method enum for OpenAPI generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "PATCH" => Some(HttpMethod::Patch),
            "DELETE" => Some(HttpMethod::Delete),
            _ => None,
        }
    }
}

/// Per-method HTTP attribute overrides
#[derive(Default, Clone)]
pub struct RouteOverride {
    pub method: Option<String>,
    pub path: Option<String>,
    pub skip: bool,
    pub hidden: bool,
    /// Tags for grouping operations in documentation
    pub tags: Vec<String>,
    /// Mark this operation as deprecated
    pub deprecated: bool,
    /// Extended description (separate from doc-comment summary)
    pub description: Option<String>,
}

impl RouteOverride {
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
                } else if meta.path.is_ident("deprecated") {
                    // Support both `deprecated` and `deprecated = true`
                    if meta.input.peek(syn::Token![=]) {
                        let value: syn::LitBool = meta.value()?.parse()?;
                        result.deprecated = value.value();
                    } else {
                        result.deprecated = true;
                    }
                    Ok(())
                } else if meta.path.is_ident("method") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    result.method = Some(value.value().to_uppercase());
                    Ok(())
                } else if meta.path.is_ident("path") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    result.path = Some(value.value());
                    Ok(())
                } else if meta.path.is_ident("tags") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    // Support comma-separated tags: tags = "users,admin"
                    result.tags = value
                        .value()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    Ok(())
                } else if meta.path.is_ident("description") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    result.description = Some(value.value());
                    Ok(())
                } else {
                    Err(meta.error(
                        "unknown attribute\n\
                         \n\
                         Valid attributes: method, path, skip, hidden, tags, deprecated, description\n\
                         \n\
                         Examples:\n\
                         - #[route(method = \"POST\")]\n\
                         - #[route(path = \"/custom\")]\n\
                         - #[route(skip)] or #[route(hidden)]\n\
                         - #[route(tags = \"users,admin\")]\n\
                         - #[route(deprecated)]\n\
                         - #[route(description = \"Extended description\")]",
                    ))
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
    /// Custom description for the response
    pub description: Option<String>,
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
                } else if meta.path.is_ident("description") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    result.description = Some(value.value());
                    Ok(())
                } else {
                    Err(meta.error(
                        "unknown attribute\n\
                         \n\
                         Valid attributes: status, content_type, header, value, description\n\
                         \n\
                         Examples:\n\
                         - #[response(status = 201)]\n\
                         - #[response(content_type = \"application/octet-stream\")]\n\
                         - #[response(header = \"X-Custom\", value = \"foo\")]\n\
                         - #[response(description = \"User created successfully\")]",
                    ))
                }
            })?;
        }

        Ok(result)
    }
}

/// Infer HTTP method from function name prefix
pub fn infer_http_method(name: &str) -> HttpMethod {
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

/// Infer URL path from method name
pub fn infer_path(method_name: &str, http_method: &HttpMethod, params: &[ParamInfo]) -> String {
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

/// Generate typed OpenAPI paths (Vec<OpenApiPath>)
///
/// Used by protocols to return structured path data for composition.
pub fn generate_openapi_paths(
    prefix: &str,
    methods_with_overrides: &[(MethodInfo, RouteOverride, ResponseOverride)],
    has_qualified: bool,
) -> syn::Result<TokenStream2> {
    let mut path_constructors = Vec::new();

    for (method, overrides, response_overrides) in methods_with_overrides {
        let method_name = method.name.to_string();

        let http_method = if let Some(ref m) = overrides.method {
            HttpMethod::from_str(m).unwrap_or_else(|| infer_http_method(&method_name))
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

        // Collect parameters
        let mut param_constructors = Vec::new();

        for param in &method.params {
            // Skip Context parameters
            if should_inject_context(&param.ty, has_qualified) {
                continue;
            }

            let location = match param.location.as_ref() {
                Some(ParamLocation::Path) => "path",
                Some(ParamLocation::Query) => "query",
                Some(ParamLocation::Body) => continue, // Body params handled separately
                Some(ParamLocation::Header) => "header",
                None => {
                    if param.is_id {
                        "path"
                    } else if default_has_body {
                        continue; // Body params
                    } else {
                        "query"
                    }
                }
            };

            let name = param
                .wire_name
                .clone()
                .unwrap_or_else(|| param.name.to_string());
            let json_type = server_less_rpc::infer_json_type(&param.ty);
            let required =
                location == "path" || (!param.is_optional && param.default_value.is_none());

            param_constructors.push(quote! {
                ::server_less::OpenApiParameter {
                    name: #name.to_string(),
                    location: #location.to_string(),
                    required: #required,
                    schema: ::server_less::serde_json::json!({"type": #json_type}),
                    description: None,
                    extra: ::server_less::serde_json::Map::new(),
                }
            });
        }

        // Build request body if needed
        let mut body_props = Vec::new();
        for param in &method.params {
            if should_inject_context(&param.ty, has_qualified) {
                continue;
            }

            let is_body = match param.location.as_ref() {
                Some(ParamLocation::Body) => true,
                None if default_has_body && !param.is_id => true,
                _ => false,
            };

            if is_body {
                let name = param
                    .wire_name
                    .clone()
                    .unwrap_or_else(|| param.name.to_string());
                let json_type = server_less_rpc::infer_json_type(&param.ty);
                body_props.push((name, json_type));
            }
        }

        let request_body = if !body_props.is_empty() {
            let prop_insertions: Vec<_> = body_props.iter().map(|(name, ty)| {
                quote! {
                    props.insert(#name.to_string(), ::server_less::serde_json::json!({"type": #ty}));
                }
            }).collect();

            quote! {
                Some({
                    let mut props = ::server_less::serde_json::Map::new();
                    #(#prop_insertions)*
                    ::server_less::serde_json::json!({
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": props
                                }
                            }
                        }
                    })
                })
            }
        } else {
            quote! { None }
        };

        // Build responses
        let ret = &method.return_info;
        let inferred_code = if ret.is_unit { "204" } else { "200" };
        let success_code = response_overrides
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| inferred_code.to_string());
        let has_error = ret.is_result;
        let success_description = response_overrides
            .description
            .clone()
            .unwrap_or_else(|| "Successful response".to_string());

        let responses = if has_error {
            quote! {
                {
                    let mut r = ::server_less::serde_json::Map::new();
                    r.insert(#success_code.to_string(), ::server_less::serde_json::json!({"description": #success_description}));
                    r.insert("400".to_string(), ::server_less::serde_json::json!({"description": "Bad request"}));
                    r.insert("500".to_string(), ::server_less::serde_json::json!({"description": "Internal server error"}));
                    r
                }
            }
        } else {
            quote! {
                {
                    let mut r = ::server_less::serde_json::Map::new();
                    r.insert(#success_code.to_string(), ::server_less::serde_json::json!({"description": #success_description}));
                    r
                }
            }
        };

        // Extract new fields from overrides
        let tags = &overrides.tags;
        let deprecated = overrides.deprecated;
        let description = overrides.description.as_ref();
        let has_description = description.is_some();
        let description_str = description.cloned().unwrap_or_default();

        path_constructors.push(quote! {
            ::server_less::OpenApiPath {
                path: #full_path.to_string(),
                method: #http_method_str.to_string(),
                operation: ::server_less::OpenApiOperation {
                    summary: Some(#summary.to_string()),
                    description: if #has_description { Some(#description_str.to_string()) } else { None },
                    operation_id: Some(#operation_id.to_string()),
                    tags: vec![#(#tags.to_string()),*],
                    deprecated: #deprecated,
                    parameters: vec![#(#param_constructors),*],
                    request_body: #request_body,
                    responses: #responses,
                    extra: ::server_less::serde_json::Map::new(),
                },
            }
        });
    }

    Ok(quote! {
        vec![#(#path_constructors),*]
    })
}

/// Generate OpenAPI 3.0 specification
pub fn generate_openapi_spec(
    struct_name: &syn::Ident,
    prefix: &str,
    methods_with_overrides: &[(MethodInfo, RouteOverride, ResponseOverride)],
    has_qualified: bool,
) -> syn::Result<TokenStream2> {
    let mut operation_data = Vec::new();

    for (method, overrides, response_overrides) in methods_with_overrides {
        let method_name = method.name.to_string();

        let http_method = if let Some(ref m) = overrides.method {
            HttpMethod::from_str(m).unwrap_or_else(|| infer_http_method(&method_name))
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
                let name = p.wire_name.clone().unwrap_or_else(|| p.name.to_string());
                let json_type = server_less_rpc::infer_json_type(&p.ty);
                quote! { (#name, "path", #json_type, true) }
            })
            .collect();

        let query_param_specs: Vec<TokenStream2> = query_params
            .iter()
            .map(|p| {
                let name = p.wire_name.clone().unwrap_or_else(|| p.name.to_string());
                let json_type = server_less_rpc::infer_json_type(&p.ty);
                let required = !p.is_optional && p.default_value.is_none();
                quote! { (#name, "query", #json_type, #required) }
            })
            .collect();

        let header_param_specs: Vec<TokenStream2> = header_params
            .iter()
            .map(|p| {
                let name = p.wire_name.clone().unwrap_or_else(|| p.name.to_string());
                let json_type = server_less_rpc::infer_json_type(&p.ty);
                let required = !p.is_optional && p.default_value.is_none();
                quote! { (#name, "header", #json_type, #required) }
            })
            .collect();

        let body_props: Vec<TokenStream2> = body_params
            .iter()
            .map(|p| {
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

        // Extract new OpenAPI fields from overrides
        let tags = &overrides.tags;
        let deprecated = overrides.deprecated;
        let description = overrides.description.as_ref();
        let has_description = description.is_some();
        let description_str = description.cloned().unwrap_or_default();
        let success_description = response_overrides
            .description
            .clone()
            .unwrap_or_else(|| "Successful response".to_string());

        operation_data.push(quote! {
            {
                let path = #full_path;
                let method = #http_method_str;
                let summary = #summary;
                let operation_id = #operation_id;
                let success_code = #success_code;
                let has_error_responses = #error_responses;
                let has_body = #has_body_props;
                let tags: Vec<&str> = vec![#(#tags),*];
                let deprecated = #deprecated;
                let has_description = #has_description;
                let description_str = #description_str;
                let success_description = #success_description;

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
                    "description": success_description
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

                // Add description if specified
                if has_description {
                    operation.as_object_mut().unwrap()
                        .insert("description".to_string(), ::server_less::serde_json::Value::String(description_str.to_string()));
                }

                // Add tags if specified
                if !tags.is_empty() {
                    let tags_json: Vec<::server_less::serde_json::Value> = tags.iter()
                        .map(|t| ::server_less::serde_json::Value::String(t.to_string()))
                        .collect();
                    operation.as_object_mut().unwrap()
                        .insert("tags".to_string(), ::server_less::serde_json::Value::Array(tags_json));
                }

                // Add deprecated flag if true
                if deprecated {
                    operation.as_object_mut().unwrap()
                        .insert("deprecated".to_string(), ::server_less::serde_json::Value::Bool(true));
                }

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
