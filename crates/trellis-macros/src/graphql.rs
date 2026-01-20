//! GraphQL handler generation using async-graphql dynamic schemas.
//!
//! Uses async-graphql's dynamic schema API to avoid proc macro limitations.

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, ItemImpl, Token};

use crate::parse::{extract_methods, get_impl_name, MethodInfo};
use crate::rpc;

/// Arguments for the #[graphql] attribute
#[derive(Default)]
pub struct GraphqlArgs {
    /// Schema name (defaults to struct name + "Schema")
    pub name: Option<String>,
}

impl Parse for GraphqlArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = GraphqlArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "name" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.name = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: name"),
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

/// Expand the #[graphql] attribute macro
pub fn expand_graphql(_args: GraphqlArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    // Separate methods into queries and mutations
    let (query_methods, mutation_methods): (Vec<_>, Vec<_>) = methods
        .iter()
        .partition(|m| is_query_method(&m.name.to_string()));

    // Generate field registrations for query
    let query_fields = generate_field_registrations(&query_methods);
    let mutation_fields = generate_field_registrations(&mutation_methods);

    // Generate resolver dispatch
    let query_resolvers = generate_resolver_dispatch(&struct_name, &query_methods);
    let mutation_resolvers = generate_resolver_dispatch(&struct_name, &mutation_methods);

    let query_type_name = format!("{}Query", struct_name);
    let mutation_type_name = format!("{}Mutation", struct_name);

    let has_mutations = !mutation_methods.is_empty();

    // Build schema construction code based on whether we have mutations
    let schema_build = if has_mutations {
        quote! {
            // Build Mutation type
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
                .finish()
                .expect("Failed to build GraphQL schema")
        }
    } else {
        quote! {
            Schema::build(#query_type_name, None::<&str>, None)
                .register(query)
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

                // Build Query type
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

            // Internal resolver dispatch for queries
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

            // Internal resolver dispatch for mutations
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

/// Check if a method should be a Query (vs Mutation)
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

/// Generate field registrations for dynamic Object
fn generate_field_registrations(methods: &[&MethodInfo]) -> Vec<TokenStream> {
    methods.iter().map(|m| generate_field_registration(m)).collect()
}

/// Generate a single field registration
fn generate_field_registration(method: &MethodInfo) -> TokenStream {
    let method_name = method.name.to_string();
    let method_ident = &method.name;
    let field_name = method_name.to_lower_camel_case();
    let description = method.docs.clone().unwrap_or_default();

    // Determine return type for GraphQL
    let ret = &method.return_info;
    let (type_ref, is_list) = infer_graphql_type_ref(ret);

    // Generate argument registrations
    let arg_registrations: Vec<_> = method.params.iter().map(|p| {
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
    }).collect();

    // Generate argument extraction
    let arg_extractions: Vec<_> = method.params.iter().map(|p| {
        let arg_name = p.name.to_string();
        let param_name = &p.name;
        let ty = &p.ty;
        if p.is_optional {
            // For optional params, try to get and parse, return None if missing
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

    // Generate method call
    let method_call = if method.is_async {
        quote! { service.#method_ident(#(#param_names),*).await }
    } else {
        quote! { service.#method_ident(#(#param_names),*) }
    };

    // Generate result conversion based on return type
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
                            .map(|item| ::async_graphql::Value::String(format!("{:?}", item)))
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
                .map(|item| ::async_graphql::Value::String(format!("{:?}", item)))
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
        // Helper to convert Rust values to GraphQL values
        fn value_to_graphql<T: std::fmt::Debug>(v: T) -> ::async_graphql::Value {
            // For now, convert to string representation
            // In production, you'd want proper serialization
            ::async_graphql::Value::String(format!("{:?}", v))
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

/// Infer GraphQL TypeRef from return info
fn infer_graphql_type_ref(ret: &crate::parse::ReturnInfo) -> (TokenStream, bool) {
    if ret.is_unit {
        (quote! { TypeRef::named_nn(TypeRef::BOOLEAN) }, false)
    } else if let Some(ref ty) = ret.ty {
        let type_str = quote!(#ty).to_string();

        // Check for Vec/array types
        let is_list = type_str.contains("Vec");

        // Determine base type
        let base_type = if type_str.contains("String") || type_str.contains("str") {
            quote! { TypeRef::STRING }
        } else if type_str.contains("i32") || type_str.contains("i64") || type_str.contains("u32") || type_str.contains("u64") || type_str.contains("usize") {
            quote! { TypeRef::INT }
        } else if type_str.contains("f32") || type_str.contains("f64") {
            quote! { TypeRef::FLOAT }
        } else if type_str.contains("bool") {
            quote! { TypeRef::BOOLEAN }
        } else {
            quote! { TypeRef::STRING } // fallback
        };

        if ret.is_option {
            if is_list {
                (quote! { TypeRef::named(TypeRef::named_list(#base_type)) }, true)
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

/// Generate resolver dispatch match arms
fn generate_resolver_dispatch(struct_name: &syn::Ident, methods: &[&MethodInfo]) -> Vec<TokenStream> {
    methods.iter().map(|m| generate_resolver_arm(struct_name, m)).collect()
}

/// Generate a single resolver match arm
fn generate_resolver_arm(_struct_name: &syn::Ident, method: &MethodInfo) -> TokenStream {
    let method_name_str = method.name.to_string();

    quote! {
        #method_name_str => {
            Ok(::async_graphql::Value::String("todo".to_string()))
        }
    }
}

/// Convert Rust type to GraphQL type name
fn rust_type_to_graphql(ty: &syn::Type) -> &'static str {
    let type_str = rpc::infer_json_type(ty);
    match type_str {
        "integer" => "Int",
        "number" => "Float",
        "boolean" => "Boolean",
        "string" => "String",
        "array" => "String", // simplified
        "object" => "String", // simplified
        _ => "String",
    }
}
