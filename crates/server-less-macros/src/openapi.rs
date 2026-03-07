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
use server_less_parse::{MethodInfo, extract_groups, extract_methods, get_impl_name, resolve_method_group};
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
    let generics_clone = impl_block.generics.clone();
    let (impl_generics, _ty_generics, where_clause) = generics_clone.split_for_impl();
    let self_ty = impl_block.self_ty.clone();
    let struct_name_str = struct_name.to_string();
    let generics_clone = impl_block.generics.clone();
    let (impl_generics, _ty_generics, where_clause) = generics_clone.split_for_impl();
    let self_ty = impl_block.self_ty.clone();

    // Detect sibling protocol attributes
    let protocols = DetectedProtocols::from_attrs(&impl_block.attrs);

    if protocols.any_detected() {
        // Protocol-aware mode: merge paths from detected protocols
        let merges = protocols.generate_merges();

        let mut detected_list = Vec::new();
        if protocols.http {
            detected_list.push("HTTP");
        }
        if protocols.jsonrpc {
            detected_list.push("JSON-RPC");
        }
        if protocols.ws {
            detected_list.push("WebSocket");
        }
        if protocols.graphql {
            detected_list.push("GraphQL");
        }
        let openapi_doc = format!(
            "Get combined OpenAPI 3.0 specification.\n\n\
             Composed from {} protocol{}: {}.",
            detected_list.len(),
            if detected_list.len() == 1 { "" } else { "s" },
            detected_list.join(", ")
        );

        // In protocol-aware mode, #[openapi] is always the outermost attribute (placed
        // first in source).  It must re-emit the impl block unconditionally so that the
        // sibling protocol macros (#[http], #[jsonrpc], etc.) can process it afterward.
        // is_protocol_impl_emitter does NOT apply here; the sibling macros themselves
        // handle deduplication via their own is_protocol_impl_emitter checks.
        Ok(quote! {
            #impl_block

            impl #impl_generics #self_ty #where_clause {
                #[doc = #openapi_doc]
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

        let group_registry = extract_groups(&impl_block)?;
        let mut openapi_methods: Vec<(MethodInfo, RouteOverride, ResponseOverride)> = Vec::new();

        for method in &methods {
            let mut overrides = RouteOverride::parse_from_attrs(&method.method.attrs)?;
            let response_overrides = ResponseOverride::parse_from_attrs(&method.method.attrs)?;

            if overrides.skip || overrides.hidden {
                continue;
            }

            // Prepend group display name to OpenAPI tags
            if let Some(group_name) = resolve_method_group(method, &group_registry)? {
                overrides.tags.insert(0, group_name);
            }

            openapi_methods.push((method.clone(), overrides, response_overrides));
        }

        let openapi_fn =
            generate_openapi_spec(&struct_name, &prefix, &openapi_methods, has_qualified)?;

        let standalone_doc = format!(
            "Get OpenAPI 3.0 specification for this service ({} endpoint{}).",
            openapi_methods.len(),
            if openapi_methods.len() == 1 { "" } else { "s" }
        );

        // Strip #[server(...)] from impl-level attrs (e.g. groups(...))
        // Also strip #[param(...)] from function parameter attrs so rustc
        // doesn't see them as unknown attributes in the emitted impl block.
        let mut clean_impl = impl_block;
        clean_impl
            .attrs
            .retain(|attr| !attr.path().is_ident("server"));
        for item in &mut clean_impl.items {
            if let syn::ImplItem::Fn(method) = item {
                // Strip method-level HTTP attributes forwarded from parse stage.
                method
                    .attrs
                    .retain(|attr| !attr.path().is_ident("route") && !attr.path().is_ident("response"));
                // Strip #[param(...)] from function parameters.
                for input in &mut method.sig.inputs {
                    if let syn::FnArg::Typed(pat_type) = input {
                        pat_type.attrs.retain(|attr| !attr.path().is_ident("param"));
                    }
                }
            }
        }

        // Only emit the impl block if no higher-priority protocol sibling is present.
        let maybe_impl = if crate::is_protocol_impl_emitter(&clean_impl, "openapi") {
            quote! { #clean_impl }
        } else {
            quote! {}
        };

        Ok(quote! {
            #maybe_impl

            impl #impl_generics #self_ty #where_clause {
                #[doc = #standalone_doc]
                pub fn openapi_spec() -> ::server_less::serde_json::Value {
                    #openapi_fn
                }
            }
        })
    }
}
