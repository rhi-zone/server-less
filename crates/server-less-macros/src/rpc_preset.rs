//! Blessed `#[rpc]` preset macro.
//!
//! Expands to `#[jsonrpc]` + `#[openrpc]` (if feature enabled) + `#[serve(jsonrpc)]` (if http feature enabled).

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Token, parse::Parse};

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
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`. Valid arguments: path, openrpc, health"
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

pub(crate) fn expand_rpc(args: RpcArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let jsonrpc_args = JsonRpcArgs { path: args.path };
    let jsonrpc_tokens = jsonrpc::expand_jsonrpc(jsonrpc_args, impl_block.clone())?;

    #[cfg(feature = "openrpc")]
    let openrpc_tokens = if args.openrpc.unwrap_or(true) {
        strip_first_impl(crate::openrpc::expand_openrpc(
            crate::openrpc::OpenRpcArgs::default(),
            impl_block.clone(),
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
        };
        strip_first_impl(crate::http::expand_serve(serve_args, impl_block)?)
    };
    #[cfg(not(feature = "http"))]
    let serve_tokens = quote! {};

    Ok(quote! { #jsonrpc_tokens #openrpc_tokens #serve_tokens })
}
