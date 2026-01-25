//! GraphQL enum type generation.
//!
//! Generates a GraphQL Enum type definition from a Rust enum.
//!
//! # Example
//!
//! ```ignore
//! use server_less::graphql_enum;
//!
//! #[graphql_enum]
//! #[derive(Clone, Debug)]
//! enum Status {
//!     /// User is active
//!     Active,
//!     /// User is inactive
//!     Inactive,
//!     /// Awaiting approval
//!     Pending,
//! }
//!
//! // Register with #[graphql]:
//! #[graphql(enums(Status))]
//! impl MyService {
//!     pub fn get_status(&self) -> Status { Status::Active }
//! }
//! ```

use heck::ToShoutySnakeCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemEnum;

pub(crate) fn expand_graphql_enum(item: ItemEnum) -> syn::Result<TokenStream2> {
    let enum_name = &item.ident;
    let enum_name_str = enum_name.to_string();

    let mut variant_registrations = Vec::new();
    let mut to_value_arms = Vec::new();

    for variant in &item.variants {
        // Only support unit variants (no fields)
        if !variant.fields.is_empty() {
            return Err(syn::Error::new_spanned(
                variant,
                "GraphQL enums only support unit variants (no fields)\n\
                 \n\
                 Example:\n\
                 #[graphql_enum]\n\
                 enum Status {\n\
                     Active,\n\
                     Inactive,\n\
                 }",
            ));
        }

        let variant_name = &variant.ident;
        let graphql_name = variant_name.to_string().to_shouty_snake_case();

        // Extract doc comment
        let doc = variant
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

        let registration = if doc.is_empty() {
            quote! {
                .item(::async_graphql::dynamic::EnumItem::new(#graphql_name))
            }
        } else {
            quote! {
                .item(::async_graphql::dynamic::EnumItem::new(#graphql_name).description(#doc))
            }
        };
        variant_registrations.push(registration);

        // to_value arm: variant => "GRAPHQL_NAME"
        to_value_arms.push(quote! {
            #enum_name::#variant_name => ::async_graphql::Value::Enum(
                ::async_graphql::Name::new(#graphql_name)
            ),
        });
    }

    Ok(quote! {
        #item

        impl #enum_name {
            /// Get the GraphQL Enum type definition for this enum.
            ///
            /// Used by `#[graphql(enums(...))]` to register the enum in the schema.
            pub fn __graphql_enum_type() -> ::async_graphql::dynamic::Enum {
                ::async_graphql::dynamic::Enum::new(#enum_name_str)
                    #(#variant_registrations)*
            }

            /// Convert this enum value to a GraphQL Value.
            pub fn __to_graphql_value(&self) -> ::async_graphql::Value {
                match self {
                    #(#to_value_arms)*
                }
            }
        }
    })
}
