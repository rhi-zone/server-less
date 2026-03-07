//! Blessed `#[server]` preset macro.
//!
//! Expands to `#[http]` + `#[openapi]` + `#[serve(http)]`.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Token, parse::Parse};

use crate::http::{self, HttpArgs, ServeArgs};
use crate::strip_first_impl;

/// Arguments for the #[server] preset attribute
#[derive(Default)]
pub(crate) struct ServerArgs {
    /// URL prefix (forwarded to HttpArgs)
    pub prefix: Option<String>,
    /// OpenAPI toggle (forwarded to HttpArgs, default: true)
    pub openapi: Option<bool>,
    /// Health check path (forwarded to ServeArgs)
    pub health: Option<String>,
}

impl Parse for ServerArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ServerArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "prefix" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.prefix = Some(lit.value());
                }
                "openapi" => {
                    let lit: syn::LitBool = input.parse()?;
                    args.openapi = Some(lit.value());
                }
                "health" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.health = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`. Valid arguments: prefix, openapi, health"
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

pub(crate) fn expand_server(args: ServerArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let http_args = HttpArgs {
        prefix: args.prefix,
        openapi: args.openapi,
        debug: false,
    };
    let http_tokens = http::expand_http(http_args, impl_block.clone())?;

    let serve_args = ServeArgs {
        protocols: vec!["http".into()],
        health_path: args.health,
        openapi: args.openapi,
    };
    let serve_tokens = strip_first_impl(http::expand_serve(serve_args, impl_block)?);

    Ok(quote! { #http_tokens #serve_tokens })
}
