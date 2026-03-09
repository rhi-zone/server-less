//! Blessed `#[program]` preset macro.
//!
//! Expands to `#[cli]` + `#[markdown]` (if feature enabled).

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Token, parse::Parse};

use crate::cli::{self, CliArgs};
use crate::strip_first_impl;

/// Arguments for the #[program] preset attribute
#[derive(Default)]
pub(crate) struct ProgramArgs {
    /// CLI name (forwarded to CliArgs)
    pub name: Option<String>,
    /// CLI version (forwarded to CliArgs)
    pub version: Option<String>,
    /// Human-readable description (forwarded to CliArgs).
    pub description: Option<String>,
    /// Homepage URL (forwarded to CliArgs)
    pub homepage: Option<String>,
    /// Markdown toggle (default: true)
    pub markdown: Option<bool>,
}

impl Parse for ProgramArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ProgramArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "name" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.name = Some(lit.value());
                }
                "version" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                "description" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.description = Some(lit.value());
                }
                "homepage" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.homepage = Some(lit.value());
                }
                "markdown" => {
                    let lit: syn::LitBool = input.parse()?;
                    args.markdown = Some(lit.value());
                }
                other => {
                    const VALID: &[&str] =
                        &["name", "version", "description", "homepage", "markdown"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}\n\
                             Valid arguments: name, version, description, homepage, markdown"
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

pub(crate) fn expand_program(args: ProgramArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let cli_args = CliArgs {
        name: args.name,
        version: args.version,
        description: args.description,
        homepage: args.homepage,
        global: Vec::new(),
        defaults: None,
        no_sync: false,
        no_async: false,
    };
    let cli_tokens = cli::expand_cli(cli_args, impl_block.clone())?;

    #[cfg(feature = "markdown")]
    let md_tokens = if args.markdown.unwrap_or(true) {
        strip_first_impl(crate::markdown::expand_markdown(
            crate::markdown::MarkdownArgs {
                title: None,
                types: true,
            },
            impl_block,
        )?)
    } else {
        quote! {}
    };
    #[cfg(not(feature = "markdown"))]
    let md_tokens = quote! {};

    Ok(quote! { #cli_tokens #md_tokens })
}
