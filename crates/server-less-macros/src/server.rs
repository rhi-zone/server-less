//! Blessed `#[server]` preset macro.
//!
//! Expands to `#[http]` + `#[openapi]` + `#[serve(http)]`.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Token, parse::Parse};

use crate::app::extract_app_meta;
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
    /// Application name (forwarded to HttpArgs/ServeArgs)
    pub name: Option<String>,
    /// Human-readable description (forwarded to HttpArgs/ServeArgs)
    pub description: Option<String>,
    /// Application version (forwarded to HttpArgs/ServeArgs)
    pub version: Option<String>,
    /// Homepage URL (forwarded to HttpArgs/ServeArgs)
    pub homepage: Option<String>,
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
                    const VALID: &[&str] =
                        &["prefix", "openapi", "health", "name", "description", "version", "homepage"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}\n\
                             Valid arguments: prefix, openapi, health, name, description, version, homepage"
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

pub(crate) fn expand_server(args: ServerArgs, mut impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    // Extract #[__app_meta] from attrs and use as fallback for unset fields.
    let app_meta = extract_app_meta(&mut impl_block.attrs);
    let name = args.name.or(app_meta.name);
    let description = args.description.or(app_meta.description);
    let version = args.version.or_else(|| app_meta.version.and_then(|v| v));
    let homepage = args.homepage.or(app_meta.homepage);

    let http_args = HttpArgs {
        prefix: args.prefix,
        openapi: args.openapi,
        debug: false,
        trace: false,
        name: name.clone(),
        description: description.clone(),
        version: version.clone(),
        homepage: homepage.clone(),
    };
    let http_tokens = http::expand_http(http_args, impl_block.clone())?;

    let serve_args = ServeArgs {
        protocols: vec!["http".into()],
        health_path: args.health,
        openapi: args.openapi,
        name,
        description,
        version,
        homepage,
    };
    let serve_tokens = strip_first_impl(http::expand_serve(serve_args, impl_block)?);

    Ok(quote! { #http_tokens #serve_tokens })
}
