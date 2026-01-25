//! Standalone and protocol-aware OpenAPI specification generation.
//!
//! The `#[openapi]` macro has two modes:
//!
//! 1. **Protocol-aware mode**: When sibling protocol attributes are detected
//!    (`#[http]`, `#[jsonrpc]`, `#[ws]`, `#[graphql]`), it generates a combined
//!    OpenAPI spec by merging paths from all detected protocols.
//!
//! 2. **Standalone mode**: When no sibling protocols are present, it generates
//!    OpenAPI specs from method naming conventions (same as `#[http]`).
//!
//! # Protocol-Aware Mode
//!
//! When combined with protocol macros, `#[openapi]` automatically detects them
//! and generates a unified spec.
//!
//! **Important:** Place `#[openapi]` FIRST (before other protocol attributes)
//! so it can detect them before they're processed:
//!
//! ```ignore
//! use server_less::{http, jsonrpc, openapi};
//!
//! #[openapi]  // FIRST - detects sibling protocols below
//! #[http(prefix = "/api", openapi = false)]
//! #[jsonrpc(path = "/rpc")]
//! impl MyService {
//!     pub fn get_status(&self) -> String { "ok".into() }
//!     pub fn add(&self, a: i32, b: i32) -> i32 { a + b }
//! }
//!
//! // Generates combined spec with both HTTP and JSON-RPC endpoints
//! let spec = MyService::openapi_spec();
//! ```
//!
//! # Standalone Mode
//!
//! Without sibling protocols, uses method naming conventions:
//! - `get_*`, `list_*` → GET
//! - `create_*`, `add_*` → POST
//! - `update_*`, `set_*` → PUT
//! - `delete_*`, `remove_*` → DELETE
//!
//! ```ignore
//! use server_less::openapi;
//!
//! #[openapi(prefix = "/api/v1")]
//! impl UserService {
//!     /// Create a new user
//!     fn create_user(&self, name: String, email: String) -> User {}
//!
//!     /// Get user by ID
//!     fn get_user(&self, id: String) -> Option<User> {}
//!
//!     /// List all users
//!     fn list_users(&self) -> Vec<User> {}
//! }
//!
//! let spec = UserService::openapi_spec();
//! ```
//!
//! # Generated Methods
//!
//! - `openapi_spec() -> serde_json::Value` - OpenAPI 3.0 specification

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, extract_methods, get_impl_name};
use syn::{ItemImpl, Token, parse::Parse};

use crate::context::has_qualified_context;
use crate::openapi_gen::{ResponseOverride, RouteOverride, generate_openapi_spec};

/// Arguments for the #[openapi] attribute
#[derive(Default)]
pub(crate) struct OpenApiArgs {
    /// URL prefix for all paths (e.g., "/api/v1")
    pub prefix: Option<String>,
}

impl Parse for OpenApiArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = OpenApiArgs::default();

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
                             Example: #[openapi(prefix = \"/api/v1\")]"
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

/// Detected sibling protocol attributes on the impl block
struct DetectedProtocols {
    http: bool,
    jsonrpc: bool,
    ws: bool,
    graphql: bool,
}

impl DetectedProtocols {
    fn from_attrs(attrs: &[syn::Attribute]) -> Self {
        let mut detected = DetectedProtocols {
            http: false,
            jsonrpc: false,
            ws: false,
            graphql: false,
        };

        for attr in attrs {
            if let Some(ident) = attr.path().get_ident() {
                match ident.to_string().as_str() {
                    "http" => detected.http = true,
                    "jsonrpc" => detected.jsonrpc = true,
                    "ws" => detected.ws = true,
                    "graphql" => detected.graphql = true,
                    _ => {}
                }
            }
        }

        detected
    }

    fn any_detected(&self) -> bool {
        self.http || self.jsonrpc || self.ws || self.graphql
    }

    /// Generate merge calls for detected protocols
    fn generate_merges(&self) -> TokenStream2 {
        let mut merges = Vec::new();

        if self.http {
            merges.push(quote! { .merge_paths(Self::http_openapi_paths()) });
        }
        if self.jsonrpc {
            merges.push(quote! { .merge_paths(Self::jsonrpc_openapi_paths()) });
        }
        if self.graphql {
            merges.push(quote! { .merge_paths(Self::graphql_openapi_paths()) });
        }
        if self.ws {
            merges.push(quote! { .merge_paths(Self::ws_openapi_paths()) });
        }

        quote! { #(#merges)* }
    }
}

pub(crate) fn expand_openapi(args: OpenApiArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();

    // Detect sibling protocol attributes
    let protocols = DetectedProtocols::from_attrs(&impl_block.attrs);

    if protocols.any_detected() {
        // Protocol-aware mode: merge paths from detected protocols
        let merges = protocols.generate_merges();

        Ok(quote! {
            #impl_block

            impl #struct_name {
                /// Get combined OpenAPI specification from all detected protocols
                pub fn openapi_spec() -> ::server_less::serde_json::Value {
                    ::server_less::OpenApiBuilder::new()
                        .title(#struct_name_str)
                        .version("0.1.0")
                        #merges
                        .build()
                }
            }
        })
    } else {
        // Standalone mode: generate paths from method naming conventions
        let methods = extract_methods(&impl_block)?;
        let has_qualified = has_qualified_context(&methods);
        let prefix = args.prefix.unwrap_or_default();

        let mut openapi_methods: Vec<(MethodInfo, RouteOverride, ResponseOverride)> = Vec::new();

        for method in &methods {
            let overrides = RouteOverride::parse_from_attrs(&method.method.attrs)?;
            let response_overrides = ResponseOverride::parse_from_attrs(&method.method.attrs)?;

            if overrides.skip || overrides.hidden {
                continue;
            }

            openapi_methods.push((method.clone(), overrides, response_overrides));
        }

        let openapi_fn =
            generate_openapi_spec(&struct_name, &prefix, &openapi_methods, has_qualified)?;

        Ok(quote! {
            #impl_block

            impl #struct_name {
                /// Get OpenAPI specification for this service
                pub fn openapi_spec() -> ::server_less::serde_json::Value {
                    #openapi_fn
                }
            }
        })
    }
}
