//! GraphQL handler generation using async-graphql dynamic schemas.
//!
//! Generates GraphQL schemas and resolvers from impl blocks using async-graphql.
//!
//! # Query vs Mutation
//!
//! Methods are classified based on naming conventions:
//! - Queries: `get_*`, `fetch_*`, `read_*`, `list_*`, `find_*`, `search_*`, `count_*`, `exists_*`, `is_*`, `has_*`
//! - Mutations: Everything else (create, update, delete, etc.)
//!
//! # Field Naming
//!
//! Method names are converted to camelCase for GraphQL fields:
//! - `get_user` → `getUser`
//! - `create_user` → `createUser`
//!
//! # Type Mapping
//!
//! Rust types are mapped to GraphQL types:
//! - `String` → String
//! - `i32`, `i64` → Int
//! - `f32`, `f64` → Float
//! - `bool` → Boolean
//! - `Vec<T>` → [T]
//! - `Option<T>` → T (nullable)
//!
//! # Custom Scalars
//!
//! async-graphql provides built-in support for common custom scalars:
//! - `chrono::DateTime<Utc>` → DateTime
//! - `uuid::Uuid` → UUID
//! - `url::Url` → Url
//! - `serde_json::Value` → JSON
//!
//! These work automatically - just use them in your method signatures:
//!
//! ```ignore
//! use chrono::{DateTime, Utc};
//! use uuid::Uuid;
//!
//! #[graphql]
//! impl UserService {
//!     async fn get_user(&self, user_id: Uuid) -> Option<User> { /* ... */ }
//!     async fn list_events(&self, since: DateTime<Utc>) -> Vec<Event> { /* ... */ }
//! }
//! ```
//!
//! # Generated Methods
//!
//! - `graphql_schema(self) -> async_graphql::dynamic::Schema` - Dynamic schema
//! - `graphql_router(self) -> axum::Router` - HTTP + Playground server
//! - `graphql_sdl(self) -> String` - Schema Definition Language
//!
//! # Example
//!
//! ```ignore
//! use server_less::graphql;
//!
//! #[derive(Clone)]
//! struct UserService;
//!
//! #[graphql(name = "UserAPI")]
//! impl UserService {
//!     /// Get user by ID (Query)
//!     async fn get_user(&self, id: i32) -> Option<String> {
//!         Some(format!("User {}", id))
//!     }
//!
//!     /// Create a new user (Mutation)
//!     async fn create_user(&self, name: String) -> String {
//!         format!("Created: {}", name)
//!     }
//! }
//!
//! // Use it:
//! let service = UserService;
//! let app = service.graphql_router();  // Serves GraphQL + Playground at /graphql
//! ```

use heck::ToLowerCamelCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, extract_methods, get_impl_name};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[graphql] attribute
#[derive(Default)]
pub(crate) struct GraphqlArgs {
    pub name: Option<String>,
    /// Enum types to register with the schema (from #[graphql_enum])
    pub enums: Vec<syn::Ident>,
}

impl Parse for GraphqlArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = GraphqlArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;

            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitStr = input.parse()?;
                    args.name = Some(lit.value());
                }
                "enums" => {
                    // Parse enums(Type1, Type2, ...)
                    let content;
                    syn::parenthesized!(content in input);
                    let enum_types = content.parse_terminated(syn::Ident::parse, Token![,])?;
                    args.enums = enum_types.into_iter().collect();
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`\n\
                             \n\
                             Valid arguments: name, enums\n\
                             \n\
                             Examples:\n\
                             - #[graphql(name = \"UserAPI\")]\n\
                             - #[graphql(enums(Status, Priority))]\n\
                             - #[graphql(name = \"MyAPI\", enums(Status))]"
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

pub(crate) fn expand_graphql(args: GraphqlArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    let (query_methods, mutation_methods): (Vec<_>, Vec<_>) = methods
        .iter()
        .partition(|m| is_query_method(&m.name.to_string()));

    let query_fields = generate_field_registrations(&query_methods);
    let mutation_fields = generate_field_registrations(&mutation_methods);

    let query_resolvers = generate_resolver_dispatch(&struct_name, &query_methods);
    let mutation_resolvers = generate_resolver_dispatch(&struct_name, &mutation_methods);

    let query_type_name = format!("{}Query", struct_name);
    let mutation_type_name = format!("{}Mutation", struct_name);

    let has_mutations = !mutation_methods.is_empty();

    // Collect custom scalars used across all methods
    let custom_scalars = collect_custom_scalars(&methods);
    let scalar_registrations: Vec<_> = custom_scalars
        .iter()
        .map(|name| {
            quote! {
                .register(Scalar::new(#name))
            }
        })
        .collect();

    // Generate enum type registrations from #[graphql(enums(...))]
    let enum_registrations: Vec<_> = args
        .enums
        .iter()
        .map(|enum_type| {
            quote! {
                .register(#enum_type::__graphql_enum_type())
            }
        })
        .collect();

    let schema_build = if has_mutations {
        quote! {
            let mutation = {
                let service = service.clone();
                let mut obj = Object::new(#mutation_type_name);
                #(
                    {
                        let service = service.clone();
                        #mutation_fields
                    }
                )*
                obj
            };

            Schema::build(#query_type_name, Some(#mutation_type_name), None)
                .register(query)
                .register(mutation)
                #(#scalar_registrations)*
                #(#enum_registrations)*
                .finish()
                .expect("Failed to build GraphQL schema")
        }
    } else {
        quote! {
            Schema::build(#query_type_name, None::<&str>, None)
                .register(query)
                #(#scalar_registrations)*
                #(#enum_registrations)*
                .finish()
                .expect("Failed to build GraphQL schema")
        }
    };

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Build the GraphQL dynamic schema
            pub fn graphql_schema(self) -> ::async_graphql::dynamic::Schema
            where
                Self: Clone + Send + Sync + 'static,
            {
                use ::async_graphql::dynamic::*;

                let service = ::std::sync::Arc::new(self);

                let query = {
                    let service = service.clone();
                    let mut obj = Object::new(#query_type_name);
                    #(
                        {
                            let service = service.clone();
                            #query_fields
                        }
                    )*
                    obj
                };

                #schema_build
            }

            /// Create an axum router with GraphQL endpoint
            pub fn graphql_router(self) -> ::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                use ::axum::routing::{get, post};
                use ::axum::response::IntoResponse;

                let schema = self.graphql_schema();

                async fn graphql_handler(
                    schema: ::axum::extract::State<::async_graphql::dynamic::Schema>,
                    req: ::async_graphql_axum::GraphQLRequest,
                ) -> ::async_graphql_axum::GraphQLResponse {
                    schema.execute(req.into_inner()).await.into()
                }

                async fn playground() -> impl IntoResponse {
                    ::axum::response::Html(
                        ::async_graphql::http::playground_source(
                            ::async_graphql::http::GraphQLPlaygroundConfig::new("/graphql")
                        )
                    )
                }

                ::axum::Router::new()
                    .route("/graphql", get(playground).post(graphql_handler))
                    .with_state(schema)
            }

            /// Get the GraphQL SDL schema
            pub fn graphql_sdl(self) -> String
            where
                Self: Clone + Send + Sync + 'static,
            {
                self.graphql_schema().sdl()
            }

            /// Get OpenAPI paths for this GraphQL service (for composition with OpenApiBuilder)
            ///
            /// Returns endpoints for GraphQL query execution and playground.
            pub fn graphql_openapi_paths() -> ::std::vec::Vec<::server_less::OpenApiPath> {
                vec![
                    ::server_less::OpenApiPath {
                        path: "/graphql".to_string(),
                        method: "post".to_string(),
                        operation: ::server_less::OpenApiOperation {
                            summary: Some("GraphQL query endpoint".to_string()),
                            operation_id: Some("graphql_query".to_string()),
                            parameters: vec![],
                            request_body: Some(::server_less::serde_json::json!({
                                "required": true,
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "required": ["query"],
                                            "properties": {
                                                "query": {
                                                    "type": "string",
                                                    "description": "GraphQL query string"
                                                },
                                                "operationName": {
                                                    "type": "string",
                                                    "description": "Optional operation name"
                                                },
                                                "variables": {
                                                    "type": "object",
                                                    "description": "Optional query variables"
                                                }
                                            }
                                        }
                                    }
                                }
                            })),
                            responses: {
                                let mut r = ::server_less::serde_json::Map::new();
                                r.insert("200".to_string(), ::server_less::serde_json::json!({
                                    "description": "GraphQL response",
                                    "content": {
                                        "application/json": {
                                            "schema": {
                                                "type": "object",
                                                "properties": {
                                                    "data": {},
                                                    "errors": {
                                                        "type": "array",
                                                        "items": {
                                                            "type": "object",
                                                            "properties": {
                                                                "message": {"type": "string"},
                                                                "locations": {"type": "array"},
                                                                "path": {"type": "array"}
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }));
                                r
                            },
                            extra: ::server_less::serde_json::Map::new(),
                        },
                    },
                    ::server_less::OpenApiPath {
                        path: "/graphql".to_string(),
                        method: "get".to_string(),
                        operation: ::server_less::OpenApiOperation {
                            summary: Some("GraphQL Playground".to_string()),
                            operation_id: Some("graphql_playground".to_string()),
                            parameters: vec![],
                            request_body: None,
                            responses: {
                                let mut r = ::server_less::serde_json::Map::new();
                                r.insert("200".to_string(), ::server_less::serde_json::json!({
                                    "description": "GraphQL Playground HTML page",
                                    "content": {
                                        "text/html": {
                                            "schema": {"type": "string"}
                                        }
                                    }
                                }));
                                r
                            },
                            extra: ::server_less::serde_json::Map::new(),
                        },
                    }
                ]
            }

            fn __graphql_resolve_query(
                service: &::std::sync::Arc<Self>,
                method: &str,
                args: &::async_graphql::dynamic::ResolverContext,
            ) -> ::async_graphql::Result<::async_graphql::Value>
            where
                Self: Send + Sync,
            {
                match method {
                    #(#query_resolvers)*
                    _ => Err(::async_graphql::Error::new(format!("Unknown query: {}", method))),
                }
            }

            fn __graphql_resolve_mutation(
                service: &::std::sync::Arc<Self>,
                method: &str,
                args: &::async_graphql::dynamic::ResolverContext,
            ) -> ::async_graphql::Result<::async_graphql::Value>
            where
                Self: Send + Sync,
            {
                match method {
                    #(#mutation_resolvers)*
                    _ => Err(::async_graphql::Error::new(format!("Unknown mutation: {}", method))),
                }
            }
        }
    })
}

fn is_query_method(name: &str) -> bool {
    name.starts_with("get_")
        || name.starts_with("fetch_")
        || name.starts_with("read_")
        || name.starts_with("list_")
        || name.starts_with("find_")
        || name.starts_with("search_")
        || name.starts_with("count_")
        || name.starts_with("exists_")
        || name.starts_with("is_")
        || name.starts_with("has_")
}

fn generate_field_registrations(methods: &[&MethodInfo]) -> Vec<TokenStream2> {
    methods
        .iter()
        .map(|m| generate_field_registration(m))
        .collect()
}

fn generate_field_registration(method: &MethodInfo) -> TokenStream2 {
    let method_name = method.name.to_string();
    let method_ident = &method.name;
    let field_name = method_name.to_lower_camel_case();
    let description = method.docs.clone().unwrap_or_default();

    let ret = &method.return_info;
    let (type_ref, is_list) = infer_graphql_type_ref(ret);

    let arg_registrations: Vec<_> = method
        .params
        .iter()
        .map(|p| {
            let arg_name = p.name.to_string();
            let gql_type = rust_type_to_graphql(&p.ty);
            let is_required = !p.is_optional;
            if is_required {
                quote! {
                    .argument(InputValue::new(#arg_name, TypeRef::named_nn(#gql_type)))
                }
            } else {
                quote! {
                    .argument(InputValue::new(#arg_name, TypeRef::named(#gql_type)))
                }
            }
        })
        .collect();

    let arg_extractions: Vec<_> = method.params.iter().map(|p| {
        let arg_name = p.name.to_string();
        let param_name = &p.name;
        let ty = &p.ty;
        if p.is_optional {
            quote! {
                let #param_name: #ty = ctx.args.try_get(#arg_name).ok()
                    .and_then(|v| v.deserialize().ok());
            }
        } else {
            quote! {
                let #param_name: #ty = ctx.args.try_get(#arg_name)
                    .map_err(|_| ::async_graphql::Error::new(format!("Missing argument: {}", #arg_name)))?
                    .deserialize()
                    .map_err(|_| ::async_graphql::Error::new(format!("Invalid argument: {}", #arg_name)))?;
            }
        }
    }).collect();

    let param_names: Vec<_> = method.params.iter().map(|p| &p.name).collect();

    let method_call = if method.is_async {
        quote! { service.#method_ident(#(#param_names),*).await }
    } else {
        quote! { service.#method_ident(#(#param_names),*) }
    };

    let result_conversion = if ret.is_unit {
        quote! {
            #method_call;
            Ok(Some(::async_graphql::Value::Boolean(true)))
        }
    } else if ret.is_result {
        if is_list {
            quote! {
                match #method_call {
                    Ok(items) => {
                        let values: Vec<_> = items.into_iter()
                            .map(|item| value_to_graphql(item))
                            .collect();
                        Ok(Some(::async_graphql::Value::List(values)))
                    }
                    Err(e) => Err(::async_graphql::Error::new(format!("{}", e))),
                }
            }
        } else {
            quote! {
                match #method_call {
                    Ok(value) => Ok(Some(value_to_graphql(value))),
                    Err(e) => Err(::async_graphql::Error::new(format!("{}", e))),
                }
            }
        }
    } else if ret.is_option {
        quote! {
            match #method_call {
                Some(value) => Ok(Some(value_to_graphql(value))),
                None => Ok(None),
            }
        }
    } else if is_list {
        quote! {
            let items = #method_call;
            let values: Vec<_> = items.into_iter()
                .map(|item| value_to_graphql(item))
                .collect();
            Ok(Some(::async_graphql::Value::List(values)))
        }
    } else {
        quote! {
            let result = #method_call;
            Ok(Some(value_to_graphql(result)))
        }
    };

    quote! {
        fn value_to_graphql<T>(v: T) -> ::async_graphql::Value
        where
            T: ::serde::Serialize + std::fmt::Debug,
        {
            // Try to serialize to JSON value first
            if let Ok(json_val) = ::serde_json::to_value(&v) {
                match json_val {
                    ::serde_json::Value::Null => ::async_graphql::Value::Null,
                    ::serde_json::Value::Bool(b) => ::async_graphql::Value::Boolean(b),
                    ::serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            ::async_graphql::Value::Number((i as i32).into())
                        } else if let Some(f) = n.as_f64() {
                            // Convert f64 to JSON value then to GraphQL value
                            match ::serde_json::to_value(f) {
                                Ok(::serde_json::Value::Number(num)) => {
                                    ::async_graphql::Value::Number(num.into())
                                }
                                _ => ::async_graphql::Value::String(f.to_string())
                            }
                        } else {
                            ::async_graphql::Value::String(n.to_string())
                        }
                    }
                    ::serde_json::Value::String(s) => ::async_graphql::Value::String(s),
                    ::serde_json::Value::Array(arr) => {
                        let values: Vec<_> = arr.into_iter()
                            .map(|item| match item {
                                ::serde_json::Value::Null => ::async_graphql::Value::Null,
                                ::serde_json::Value::Bool(b) => ::async_graphql::Value::Boolean(b),
                                ::serde_json::Value::Number(n) => {
                                    if let Some(i) = n.as_i64() {
                                        ::async_graphql::Value::Number((i as i32).into())
                                    } else {
                                        // Use the serde_json Number directly
                                        ::async_graphql::Value::Number(n.into())
                                    }
                                }
                                ::serde_json::Value::String(s) => ::async_graphql::Value::String(s),
                                other => ::async_graphql::Value::String(other.to_string()),
                            })
                            .collect();
                        ::async_graphql::Value::List(values)
                    }
                    ::serde_json::Value::Object(obj) => {
                        // Convert JSON object to GraphQL object
                        let mut fields = ::async_graphql::indexmap::IndexMap::new();
                        for (key, value) in obj {
                            let gql_value = match value {
                                ::serde_json::Value::Null => ::async_graphql::Value::Null,
                                ::serde_json::Value::Bool(b) => ::async_graphql::Value::Boolean(b),
                                ::serde_json::Value::Number(n) => {
                                    if let Some(i) = n.as_i64() {
                                        ::async_graphql::Value::Number((i as i32).into())
                                    } else {
                                        ::async_graphql::Value::Number(n.into())
                                    }
                                }
                                ::serde_json::Value::String(s) => ::async_graphql::Value::String(s),
                                ::serde_json::Value::Array(arr) => {
                                    let values: Vec<_> = arr.into_iter()
                                        .map(|item| match item {
                                            ::serde_json::Value::Null => ::async_graphql::Value::Null,
                                            ::serde_json::Value::Bool(b) => ::async_graphql::Value::Boolean(b),
                                            ::serde_json::Value::Number(n) => ::async_graphql::Value::Number(n.into()),
                                            ::serde_json::Value::String(s) => ::async_graphql::Value::String(s),
                                            other => ::async_graphql::Value::String(other.to_string()),
                                        })
                                        .collect();
                                    ::async_graphql::Value::List(values)
                                }
                                ::serde_json::Value::Object(_) => {
                                    // Recursively convert nested objects to strings for now
                                    ::async_graphql::Value::String(value.to_string())
                                }
                            };
                            fields.insert(::async_graphql::Name::new(key), gql_value);
                        }
                        ::async_graphql::Value::Object(fields)
                    }
                }
            } else {
                // Fallback to Debug formatting
                ::async_graphql::Value::String(format!("{:?}", v))
            }
        }

        let field = Field::new(#field_name, #type_ref, move |ctx| {
            let service = service.clone();
            FieldFuture::new(async move {
                #(#arg_extractions)*
                #result_conversion
            })
        })
        .description(#description)
        #(#arg_registrations)*;
        obj = obj.field(field);
    }
}

fn infer_graphql_type_ref(ret: &server_less_parse::ReturnInfo) -> (TokenStream2, bool) {
    if ret.is_unit {
        (quote! { TypeRef::named_nn(TypeRef::BOOLEAN) }, false)
    } else if let Some(ref ty) = ret.ty {
        let type_str = quote!(#ty).to_string();

        let is_list = type_str.contains("Vec");

        // Check for custom scalars first (async-graphql built-ins)
        let base_type = if type_str.contains("DateTime") {
            quote! { "DateTime" }
        } else if type_str.contains("Uuid") {
            quote! { "UUID" }
        } else if type_str.contains("Url") {
            quote! { "Url" }
        } else if type_str.contains("serde_json :: Value") || type_str == "Value" {
            quote! { "JSON" }
        } else if type_str.contains("String") || type_str.contains("str") {
            quote! { TypeRef::STRING }
        } else if type_str.contains("i32")
            || type_str.contains("i64")
            || type_str.contains("u32")
            || type_str.contains("u64")
            || type_str.contains("usize")
        {
            quote! { TypeRef::INT }
        } else if type_str.contains("f32") || type_str.contains("f64") {
            quote! { TypeRef::FLOAT }
        } else if type_str.contains("bool") {
            quote! { TypeRef::BOOLEAN }
        } else {
            quote! { TypeRef::STRING }
        };

        if ret.is_option {
            if is_list {
                (
                    quote! { TypeRef::named(TypeRef::named_list(#base_type)) },
                    true,
                )
            } else {
                (quote! { TypeRef::named(#base_type) }, false)
            }
        } else if ret.is_result {
            if is_list {
                (quote! { TypeRef::named_nn_list(#base_type) }, true)
            } else {
                (quote! { TypeRef::named_nn(#base_type) }, false)
            }
        } else if is_list {
            (quote! { TypeRef::named_nn_list(#base_type) }, true)
        } else {
            (quote! { TypeRef::named_nn(#base_type) }, false)
        }
    } else {
        (quote! { TypeRef::named_nn(TypeRef::BOOLEAN) }, false)
    }
}

fn generate_resolver_dispatch(
    struct_name: &syn::Ident,
    methods: &[&MethodInfo],
) -> Vec<TokenStream2> {
    methods
        .iter()
        .map(|m| generate_resolver_arm(struct_name, m))
        .collect()
}

fn generate_resolver_arm(_struct_name: &syn::Ident, method: &MethodInfo) -> TokenStream2 {
    let method_name_str = method.name.to_string();

    quote! {
        #method_name_str => {
            Ok(::async_graphql::Value::String("todo".to_string()))
        }
    }
}

fn rust_type_to_graphql(ty: &syn::Type) -> &'static str {
    let type_str = quote!(#ty).to_string();

    // Try to extract inner type for Vec<T>
    if type_str.contains("Vec") {
        return extract_vec_inner_type(&type_str);
    }

    // Check for custom scalars (async-graphql built-ins)
    if type_str.contains("DateTime") {
        return "DateTime";
    }
    if type_str.contains("Uuid") {
        return "UUID";
    }
    if type_str.contains("Url") {
        return "Url";
    }
    if type_str.contains("serde_json :: Value") || type_str == "Value" {
        return "JSON";
    }

    let json_type = server_less_rpc::infer_json_type(ty);
    match json_type {
        "integer" => "Int",
        "number" => "Float",
        "boolean" => "Boolean",
        "string" => "String",
        _ => "String", // Custom types default to String for now
    }
}

fn extract_vec_inner_type(type_str: &str) -> &'static str {
    // Try to extract T from Vec<T>
    if let Some(start) = type_str.find("Vec<") {
        let inner = &type_str[start + 4..];
        if let Some(end) = inner.find('>') {
            let inner_type = inner[..end].trim();
            return map_inner_type_to_graphql(inner_type);
        }
    }
    "String"
}

fn map_inner_type_to_graphql(inner: &str) -> &'static str {
    // Check for custom scalars first
    if inner.contains("DateTime") {
        return "DateTime";
    }
    if inner.contains("Uuid") {
        return "UUID";
    }
    if inner.contains("Url") {
        return "Url";
    }
    if inner.contains("serde_json :: Value") || inner == "Value" {
        return "JSON";
    }

    // Standard types
    if inner.contains("String") || inner.contains("str") {
        "String"
    } else if inner.contains("i32")
        || inner.contains("i64")
        || inner.contains("u32")
        || inner.contains("u64")
        || inner.contains("isize")
        || inner.contains("usize")
    {
        "Int"
    } else if inner.contains("f32") || inner.contains("f64") {
        "Float"
    } else if inner.contains("bool") {
        "Boolean"
    } else {
        "String" // Custom types default to String
    }
}

/// Collect custom scalar types used across all methods (parameters + return types).
///
/// Returns a deduplicated list of scalar names that need to be registered
/// with the dynamic schema builder.
fn collect_custom_scalars(methods: &[MethodInfo]) -> Vec<String> {
    let mut scalars = std::collections::BTreeSet::new();

    for method in methods {
        // Check parameters
        for param in &method.params {
            let ty = &param.ty;
            check_type_for_scalars(&quote!(#ty).to_string(), &mut scalars);
        }

        // Check return type
        if let Some(ref ty) = method.return_info.ty {
            check_type_for_scalars(&quote!(#ty).to_string(), &mut scalars);
        }
    }

    scalars.into_iter().collect()
}

/// Check a type string for custom scalar types and add them to the set.
fn check_type_for_scalars(type_str: &str, scalars: &mut std::collections::BTreeSet<String>) {
    if type_str.contains("DateTime") {
        scalars.insert("DateTime".to_string());
    }
    if type_str.contains("Uuid") {
        scalars.insert("UUID".to_string());
    }
    if type_str.contains("Url") && !type_str.contains("UrlError") {
        scalars.insert("Url".to_string());
    }
    if type_str.contains("serde_json :: Value") || type_str == "Value" {
        scalars.insert("JSON".to_string());
    }
}
