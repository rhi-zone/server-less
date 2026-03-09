//! Blessed `#[server]` preset macro.
//!
//! Expands to `#[http]` + `#[openapi]` + `#[serve(http)]`.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Path, Token, parse::Parse};

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
    /// Config struct path for linked config management (`config = MyConfig`).
    pub config_ty: Option<Path>,
    /// Config subcommand name override (`config_cmd = "settings"`) or `false` to disable.
    pub config_cmd_name: Option<String>,
    /// Whether the config subcommand is enabled (default: true when config_ty is set).
    pub config_cmd: bool,
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
                "config" => {
                    let path: Path = input.parse()?;
                    args.config_ty = Some(path);
                    args.config_cmd = true;
                }
                "config_cmd" => {
                    if input.peek(syn::LitBool) {
                        let lit: syn::LitBool = input.parse()?;
                        args.config_cmd = lit.value();
                    } else {
                        let lit: syn::LitStr = input.parse()?;
                        args.config_cmd_name = Some(lit.value());
                        args.config_cmd = true;
                    }
                }
                other => {
                    const VALID: &[&str] = &[
                        "prefix", "openapi", "health", "name", "description", "version",
                        "homepage", "config", "config_cmd",
                    ];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}\n\
                             Valid arguments: prefix, openapi, health, name, description, version, homepage, config, config_cmd"
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
        name: name.clone(),
        description,
        version,
        homepage,
    };
    let serve_tokens = strip_first_impl(http::expand_serve(serve_args, impl_block.clone())?);

    #[cfg(feature = "config")]
    let config_methods = if args.config_cmd {
        if let Some(ref config_ty) = args.config_ty {
            let self_ty = &impl_block.self_ty;
            let cmd_name = args.config_cmd_name.as_deref().unwrap_or("config");
            let app_name = name.as_deref().unwrap_or("app");
            let (methods, _subcommand_addition, _dispatch_arm) =
                crate::config_cmd::generate_all(self_ty, config_ty, cmd_name, app_name);
            methods
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };
    #[cfg(not(feature = "config"))]
    let config_methods = quote! {};

    Ok(quote! { #http_tokens #serve_tokens #config_methods })
}
