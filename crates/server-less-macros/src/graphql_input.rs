//! GraphQL input type generation.
//!
//! Generates a GraphQL InputObject type definition from a Rust struct.
//!
//! # Example
//!
//! ```ignore
//! use server_less::graphql_input;
//!
//! #[graphql_input]
//! #[derive(Clone, Debug, serde::Deserialize)]
//! struct CreateUserInput {
//!     /// User's name
//!     name: String,
//!     /// User's email address
//!     email: String,
//!     /// Optional age
//!     age: Option<i32>,
//! }
//!
//! // Register with #[graphql]:
//! #[graphql(inputs(CreateUserInput))]
//! impl UserService {
//!     pub fn create_user(&self, input: CreateUserInput) -> User { /* ... */ }
//! }
//! ```

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Fields, ItemStruct};

pub(crate) fn expand_graphql_input(item: ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &item.ident;
    let struct_name_str = struct_name.to_string();

    // Only support named fields
    let fields = match &item.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return Err(syn::Error::new_spanned(
                &item,
                "GraphQL input types must have named fields\n\
                 \n\
                 Example:\n\
                 #[graphql_input]\n\
                 struct CreateUserInput {\n\
                     name: String,\n\
                     email: String,\n\
                 }",
            ));
        }
    };

    let mut field_registrations = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let graphql_name = field_name_str.to_lower_camel_case();
        let ty = &field.ty;

        // Extract doc comment
        let doc = field
            .attrs
            .iter()
            .filter_map(|attr| {
                if attr.path().is_ident("doc")
                    && let syn::Meta::NameValue(nv) = &attr.meta
                    && let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                {
                    return Some(s.value().trim().to_string());
                }
                None
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Infer GraphQL type from Rust type
        let type_str = quote!(#ty).to_string();
        let is_optional = type_str.contains("Option");
        let is_list = type_str.contains("Vec");

        // Determine base type
        let base_type = if type_str.contains("String") || type_str.contains("& str") {
            quote! { TypeRef::STRING }
        } else if type_str.contains("i32") || type_str.contains("i64") {
            quote! { TypeRef::INT }
        } else if type_str.contains("f32") || type_str.contains("f64") {
            quote! { TypeRef::FLOAT }
        } else if type_str.contains("bool") {
            quote! { TypeRef::BOOLEAN }
        } else {
            // Default to JSON for complex types
            quote! { TypeRef::named("JSON") }
        };

        // Build the type reference
        let type_ref = if is_list && is_optional {
            quote! { TypeRef::named_list(#base_type) }
        } else if is_list {
            quote! { TypeRef::named_nn_list(#base_type) }
        } else if is_optional {
            quote! { TypeRef::named(#base_type) }
        } else {
            quote! { TypeRef::named_nn(#base_type) }
        };

        let registration = if doc.is_empty() {
            quote! {
                .field(::async_graphql::dynamic::InputValue::new(#graphql_name, #type_ref))
            }
        } else {
            quote! {
                .field(::async_graphql::dynamic::InputValue::new(#graphql_name, #type_ref).description(#doc))
            }
        };
        field_registrations.push(registration);
    }

    Ok(quote! {
        #item

        impl #struct_name {
            /// Get the GraphQL InputObject type definition for this struct.
            ///
            /// Used by `#[graphql(inputs(...))]` to register the input type in the schema.
            pub fn __graphql_input_type() -> ::async_graphql::dynamic::InputObject {
                use ::async_graphql::dynamic::TypeRef;
                ::async_graphql::dynamic::InputObject::new(#struct_name_str)
                    #(#field_registrations)*
            }

            /// Parse this input type from a GraphQL InputValue.
            ///
            /// Uses serde_json for conversion since the struct must implement Deserialize.
            pub fn __from_graphql_value(value: ::async_graphql::Value) -> ::std::result::Result<Self, String>
            where
                Self: ::serde::de::DeserializeOwned,
            {
                // Convert async_graphql::Value to serde_json::Value
                let json_str = value.to_string();
                ::serde_json::from_str(&json_str)
                    .map_err(|e| format!("Failed to parse input: {}", e))
            }
        }
    })
}
