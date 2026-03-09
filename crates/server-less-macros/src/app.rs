//! Implementation of `#[app(...)]` and `#[__app_meta(...)]`.
//!
//! # How the passthrough works
//!
//! `#[app(...)]` is a protocol-neutral metadata attribute.  It doesn't generate
//! code by itself; it passes its arguments to downstream protocol macros
//! (`#[server]`, `#[cli]`, `#[http]`, etc.) via a helper attribute.
//!
//! When `#[app(name = "myapp", version = "1.0")]` is applied to an impl block,
//! it emits the block with `#[__app_meta(name = "myapp", version = "1.0")]`
//! prepended to its attribute list.  The downstream macro (which runs next)
//! calls [`extract_app_meta`] to pull the values out before generating code.
//!
//! `#[__app_meta]` is a registered no-op passthrough: if no downstream macro
//! consumed it, it strips itself and emits the item unchanged, preventing a
//! "unknown attribute" compile error.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, parse::Parser, punctuated::Punctuated};

/// Parsed contents of `#[app(...)]` or `#[__app_meta(...)]`.
#[derive(Debug, Clone, Default)]
pub struct AppMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    /// `None` = use CARGO_PKG_VERSION, `Some(None)` = disabled, `Some(Some(v))` = explicit
    pub version: Option<Option<String>>,
    pub homepage: Option<String>,
}

/// Parse `#[app(name = "...", description = "...", version = "...", homepage = "...")]`.
fn parse_app_args(args: proc_macro2::TokenStream) -> syn::Result<AppMeta> {
    let mut meta = AppMeta::default();

    if args.is_empty() {
        return Ok(meta);
    }

    let parser = Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    let items = parser.parse2(args)?;

    const VALID: &[&str] = &["name", "description", "version", "homepage"];

    for item in items {
        match &item {
            syn::Meta::NameValue(nv) if nv.path.is_ident("name") => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    meta.name = Some(s.value());
                } else {
                    return Err(syn::Error::new_spanned(&nv.value, "`name` must be a string literal"));
                }
            }
            syn::Meta::NameValue(nv) if nv.path.is_ident("description") => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    meta.description = Some(s.value());
                } else {
                    return Err(syn::Error::new_spanned(&nv.value, "`description` must be a string literal"));
                }
            }
            syn::Meta::NameValue(nv) if nv.path.is_ident("version") => {
                match &nv.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => {
                        meta.version = Some(Some(s.value()));
                    }
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Bool(b),
                        ..
                    }) if !b.value => {
                        // version = false → disabled
                        meta.version = Some(None);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &nv.value,
                            "`version` must be a string literal or `false`",
                        ));
                    }
                }
            }
            syn::Meta::NameValue(nv) if nv.path.is_ident("homepage") => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    meta.homepage = Some(s.value());
                } else {
                    return Err(syn::Error::new_spanned(&nv.value, "`homepage` must be a string literal"));
                }
            }
            other => {
                let ident = other
                    .path()
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default();
                let suggestion = crate::did_you_mean(&ident, VALID)
                    .map(|s| format!(" — did you mean `{s}`?"))
                    .unwrap_or_default();
                return Err(syn::Error::new_spanned(
                    other,
                    format!(
                        "unknown `#[app]` argument `{ident}`{suggestion}\n\
                         \n\
                         Valid arguments: name, description, version, homepage\n\
                         \n\
                         Example: #[app(name = \"myapp\", description = \"Does the thing\", version = \"1.0.0\")]"
                    ),
                ));
            }
        }
    }

    Ok(meta)
}

/// Expand `#[app(...)]`: prepend `#[__app_meta(...)]` to the impl block's attrs,
/// then emit the block unchanged (including its remaining macros).
pub fn expand_app(args: TokenStream2, item: ItemImpl) -> syn::Result<TokenStream2> {
    let meta = parse_app_args(args.clone())?;

    // Build the __app_meta attribute tokens mirroring the original args so
    // downstream macros can re-parse them with parse_app_args.
    let meta_attr = build_meta_attr(&meta);

    Ok(quote! {
        #meta_attr
        #item
    })
}

/// Build `#[__app_meta(name = "...", ...)]` tokens from a parsed [`AppMeta`].
pub fn build_meta_attr(meta: &AppMeta) -> TokenStream2 {
    let mut parts = Vec::<TokenStream2>::new();

    if let Some(name) = &meta.name {
        parts.push(quote! { name = #name });
    }
    if let Some(desc) = &meta.description {
        parts.push(quote! { description = #desc });
    }
    match &meta.version {
        Some(Some(v)) => parts.push(quote! { version = #v }),
        Some(None) => parts.push(quote! { version = false }),
        None => {}
    }
    if let Some(hp) = &meta.homepage {
        parts.push(quote! { homepage = #hp });
    }

    if parts.is_empty() {
        quote! { #[__app_meta()] }
    } else {
        quote! { #[__app_meta(#(#parts),*)] }
    }
}

/// Extract and remove `#[__app_meta(...)]` from an attribute list.
#[allow(dead_code)] // used by consuming macros once wired up
///
/// Called by consuming macros before they generate code.  Returns a default
/// `AppMeta` if no `#[__app_meta]` attribute is present.
pub fn extract_app_meta(attrs: &mut Vec<syn::Attribute>) -> AppMeta {
    let mut result = AppMeta::default();
    attrs.retain(|attr| {
        if attr.path().is_ident("__app_meta") {
            // Parse the args out of the attribute tokens.
            let tokens = match &attr.meta {
                syn::Meta::List(list) => list.tokens.clone(),
                _ => return true, // malformed, leave in place
            };
            if let Ok(parsed) = parse_app_args(tokens) {
                result = parsed;
            }
            false // remove the attribute
        } else {
            true
        }
    });
    result
}

/// Expand `#[__app_meta(...)]`: no-op passthrough.
///
/// This fires when no downstream macro consumed the attribute — e.g. the user
/// wrote `#[app(...)]` without any protocol macro below it.  We just strip the
/// attribute and emit the item unchanged.
pub fn expand_app_meta_passthrough(_args: TokenStream2, item: ItemImpl) -> TokenStream2 {
    quote! { #item }
}
