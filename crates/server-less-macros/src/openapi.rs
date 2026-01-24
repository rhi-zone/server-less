//! Standalone OpenAPI specification generation.
//!
//! Generates OpenAPI 3.0 specifications from impl blocks using the same
//! naming conventions as the HTTP macro, but without generating route handlers.
//!
//! # When to Use
//!
//! Use `#[openapi]` when you want:
//! - Schema-first development: define your API shape before implementation
//! - Documentation only: generate specs without runtime code
//! - Separate concerns: OpenAPI generation separate from HTTP routing
//!
//! Use `#[http]` (with default `openapi = true`) when you want:
//! - Full HTTP routing with automatic OpenAPI generation
//! - Everything in one macro
//!
//! # Naming Conventions
//!
//! Same as `#[http]` - method prefixes determine HTTP methods and paths:
//! - `get_*`, `list_*` → GET
//! - `create_*`, `add_*` → POST
//! - `update_*`, `set_*` → PUT
//! - `delete_*`, `remove_*` → DELETE
//!
//! # Generated Methods
//!
//! - `openapi_spec() -> serde_json::Value` - OpenAPI 3.0 specification
//!
//! # Example
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
//! // Generate spec:
//! let spec = UserService::openapi_spec();
//! println!("{}", serde_json::to_string_pretty(&spec).unwrap());
//! ```
//!
//! # Combining with #[http]
//!
//! If you want OpenAPI generation separate from HTTP routing:
//!
//! ```ignore
//! // Option 1: Disable OpenAPI in http, use separate macro
//! #[http(openapi = false)]
//! #[openapi(prefix = "/api")]
//! impl MyService { ... }
//!
//! // Option 2: Just use http with default (openapi = true)
//! #[http(prefix = "/api")]
//! impl MyService { ... }
//! ```

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

pub(crate) fn expand_openapi(args: OpenApiArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    // Scan for qualified server_less::Context usage
    let has_qualified = has_qualified_context(&methods);

    let prefix = args.prefix.unwrap_or_default();

    // Collect method information for OpenAPI generation
    let mut openapi_methods: Vec<(MethodInfo, RouteOverride, ResponseOverride)> = Vec::new();

    for method in &methods {
        let overrides = RouteOverride::parse_from_attrs(&method.method.attrs)?;
        let response_overrides = ResponseOverride::parse_from_attrs(&method.method.attrs)?;

        if overrides.skip || overrides.hidden {
            continue;
        }

        openapi_methods.push((method.clone(), overrides, response_overrides));
    }

    // Generate OpenAPI spec
    let openapi_fn = generate_openapi_spec(&struct_name, &prefix, &openapi_methods, has_qualified)?;

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
