//! Blessed `#[rpc]` preset macro.
//!
//! Expands to `#[jsonrpc]` + `#[openrpc]` (if feature enabled) + `#[serve(jsonrpc)]` (if http feature enabled).

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Token, parse::Parse};

use crate::app::extract_app_meta;
use crate::jsonrpc::{self, JsonRpcArgs};
use crate::strip_first_impl;

/// Arguments for the #[rpc] preset attribute
#[derive(Default)]
pub(crate) struct RpcArgs {
    /// JSON-RPC path (forwarded to JsonRpcArgs)
    pub path: Option<String>,
    /// OpenRPC toggle (default: true)
    pub openrpc: Option<bool>,
    /// Health check path (forwarded to ServeArgs)
    pub health: Option<String>,
    /// Application name (inline shorthand for `#[app(name)]`)
    pub name: Option<String>,
    /// Human-readable description (inline shorthand for `#[app(description)]`)
    pub description: Option<String>,
    /// Version string (inline shorthand for `#[app(version)]`)
    pub version: Option<String>,
    /// Homepage URL (inline shorthand for `#[app(homepage)]`)
    pub homepage: Option<String>,
}

impl Parse for RpcArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = RpcArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "path" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.path = Some(lit.value());
                }
                "openrpc" => {
                    let lit: syn::LitBool = input.parse()?;
                    args.openrpc = Some(lit.value());
                }
                "health" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.health = Some(lit.value());
                }
                "name" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.name = Some(lit.value());
                }
                "description" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.description = Some(lit.value());
                }
                "version" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                "homepage" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.homepage = Some(lit.value());
                }
                other => {
                    const VALID_ARGS: &[&str] = &[
                        "path", "openrpc", "health", "name", "description", "version", "homepage",
                    ];
                    let suggestion = crate::did_you_mean(other, VALID_ARGS)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}. \
                             Valid arguments: path, openrpc, health, name, description, version, homepage"
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

pub(crate) fn expand_rpc(args: RpcArgs, mut impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    // Extract #[__app_meta] from attrs and use as fallback for unset fields.
    let app_meta = extract_app_meta(&mut impl_block.attrs);
    let name = args.name.or(app_meta.name);
    let description = args.description.or(app_meta.description);
    let version = args.version.or_else(|| app_meta.version.into_explicit());
    let homepage = args.homepage.or(app_meta.homepage);

    let jsonrpc_args = JsonRpcArgs { path: args.path };
    let jsonrpc_tokens = jsonrpc::expand_jsonrpc(jsonrpc_args, impl_block.clone())?;

    #[cfg(feature = "openrpc")]
    let openrpc_tokens = if args.openrpc.unwrap_or(true) {
        // Inject app metadata into the impl block attrs so expand_openrpc can consume it.
        let mut openrpc_block = impl_block.clone();
        if name.is_some() || description.is_some() || version.is_some() || homepage.is_some() {
            let meta = crate::app::AppMeta {
                name: name.clone(),
                description: description.clone(),
                version: version.as_ref()
                    .map(|v| crate::app::VersionSpec::Explicit(v.clone()))
                    .unwrap_or(crate::app::VersionSpec::Auto),
                homepage: homepage.clone(),
            };
            let attr_tokens = crate::app::build_meta_attr(&meta);
            // Parse via a dummy struct wrapper to extract the `syn::Attribute`.
            let dummy: syn::ItemStruct = syn::parse2(quote! {
                #attr_tokens struct __Dummy;
            })
            .map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), format!("BUG: {e}")))?;
            if let Some(attr) = dummy.attrs.into_iter().next() {
                openrpc_block.attrs.insert(0, attr);
            }
        }
        strip_first_impl(crate::openrpc::expand_openrpc(
            crate::openrpc::OpenRpcArgs::default(),
            openrpc_block,
        )?)
    } else {
        quote! {}
    };
    #[cfg(not(feature = "openrpc"))]
    let openrpc_tokens = quote! {};

    #[cfg(feature = "http")]
    let serve_tokens = {
        let serve_args = crate::http::ServeArgs {
            protocols: vec!["jsonrpc".into()],
            health_path: args.health,
            openapi: Some(false),
            name: name.clone(),
            description: description.clone(),
            version: version.clone(),
            homepage: homepage.clone(),
        };
        strip_first_impl(crate::http::expand_serve(serve_args, impl_block)?)
    };
    #[cfg(not(feature = "http"))]
    let serve_tokens = quote! {};

    Ok(quote! { #jsonrpc_tokens #openrpc_tokens #serve_tokens })
}
