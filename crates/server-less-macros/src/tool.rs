//! Blessed `#[tool]` preset macro.
//!
//! Expands to `#[mcp]` + `#[jsonschema]` (if feature enabled).

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Token, parse::Parse};

use crate::mcp::{self, McpArgs};
use crate::strip_first_impl;

/// Arguments for the #[tool] preset attribute
#[derive(Default)]
pub(crate) struct ToolArgs {
    /// MCP namespace (forwarded to McpArgs)
    pub namespace: Option<String>,
    /// JSON Schema toggle (default: true)
    pub jsonschema: Option<bool>,
}

impl Parse for ToolArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ToolArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "namespace" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.namespace = Some(lit.value());
                }
                "jsonschema" => {
                    let lit: syn::LitBool = input.parse()?;
                    args.jsonschema = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`. Valid arguments: namespace, jsonschema"
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

pub(crate) fn expand_tool(args: ToolArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let mcp_args = McpArgs::with_namespace(args.namespace);
    let mcp_tokens = mcp::expand_mcp(mcp_args, impl_block.clone())?;

    #[cfg(feature = "jsonschema")]
    let schema_tokens = if args.jsonschema.unwrap_or(true) {
        strip_first_impl(crate::jsonschema::expand_jsonschema(
            crate::jsonschema::JsonSchemaArgs::default(),
            impl_block,
        )?)
    } else {
        quote! {}
    };
    #[cfg(not(feature = "jsonschema"))]
    let schema_tokens = quote! {};

    Ok(quote! { #mcp_tokens #schema_tokens })
}
