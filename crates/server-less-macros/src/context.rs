//! Shared Context injection logic for protocol macros.
//!
//! This module provides helpers for detecting and injecting server_less::Context
//! parameters across different protocol implementations (HTTP, WebSocket, CLI, etc.).

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, ParamInfo};
use syn::Type;

/// Check if a type is server_less::Context (fully qualified)
///
/// This matches paths like:
/// - `server_less::Context`
/// - `::server_less::Context`
/// - `crate::server_less::Context`
pub fn is_qualified_context(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        let path = &type_path.path;

        // Check if path contains "server_less" and ends with "Context"
        let segments: Vec<_> = path.segments.iter().collect();

        if segments.len() >= 2 {
            // Look for server_less::Context pattern
            for i in 0..segments.len() - 1 {
                if segments[i].ident == "server_less" && segments[i + 1].ident == "Context" {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a type is bare `Context` (unqualified)
pub fn is_bare_context(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && type_path.path.segments.len() == 1
    {
        return type_path.path.segments[0].ident == "Context";
    }
    false
}

/// Check if this type should be treated as server_less::Context for injection
///
/// Two-pass detection strategy:
/// - If `has_qualified` is true: Only qualified `server_less::Context` is injected
/// - If `has_qualified` is false: Both bare `Context` and qualified are injected
///
/// This allows users to disambiguate if they have their own Context type:
/// ```ignore
/// // No collision - bare Context works
/// fn foo(&self, ctx: Context) { }  // ✅ Injected
///
/// // Collision - use qualified form
/// struct Context { /* user's type */ }
/// fn foo(&self, ctx: server_less::Context) { }  // ✅ Injected
/// fn bar(&self, ctx: Context) { }  // ❌ NOT injected (user's type)
/// ```
pub fn should_inject_context(ty: &Type, has_qualified: bool) -> bool {
    if is_qualified_context(ty) {
        true
    } else if is_bare_context(ty) {
        // Only inject bare Context if no qualified version exists in the impl block
        !has_qualified
    } else {
        false
    }
}

/// Scan all methods to detect if any use qualified server_less::Context
///
/// This is the "first pass" of the two-pass detection strategy.
///
/// - If any method uses `server_less::Context`, we assume bare `Context` is a user type
/// - If no method uses qualified form, bare `Context` is assumed to be ours
pub fn has_qualified_context(methods: &[MethodInfo]) -> bool {
    methods.iter().any(|method| {
        method
            .params
            .iter()
            .any(|param| is_qualified_context(&param.ty))
    })
}

/// Partition parameters into Context and non-Context groups
///
/// Returns `(context_param, other_params)` where:
/// - `context_param` is `Some(param)` if a Context parameter was found
/// - `other_params` contains all non-Context parameters
///
/// Returns an error if multiple Context parameters are found.
pub fn partition_context_params(
    params: &[ParamInfo],
    has_qualified: bool,
) -> syn::Result<(Option<&ParamInfo>, Vec<&ParamInfo>)> {
    let mut context_param: Option<&ParamInfo> = None;
    let mut other_params = Vec::new();

    for param in params {
        if should_inject_context(&param.ty, has_qualified) {
            if context_param.is_some() {
                return Err(syn::Error::new_spanned(
                    &param.ty,
                    "only one Context parameter allowed per method\n\
                     \n\
                     Hint: server_less::Context is automatically injected from request metadata.\n\
                     Remove the duplicate Context parameter.",
                ));
            }
            context_param = Some(param);
        } else {
            other_params.push(param);
        }
    }

    Ok((context_param, other_params))
}

/// Generate Context extraction code for HTTP-based protocols
///
/// This creates code that:
/// 1. Extracts the HeaderMap
/// 2. Populates a Context with all headers
/// 3. Extracts standard fields (x-request-id, etc.)
///
/// Returns `(extraction, call)` where:
/// - `extraction` is the axum extractor token (e.g., `headers: HeaderMap`)
/// - `call` is the code to create and populate the Context
pub fn generate_http_context_extraction() -> (TokenStream2, TokenStream2) {
    let extraction = quote! {
        __context_headers: ::axum::http::HeaderMap
    };

    let call = quote! {
        {
            let mut __ctx = ::server_less::Context::new();
            // Populate context with headers
            for (name, value) in __context_headers.iter() {
                if let Ok(value_str) = value.to_str() {
                    __ctx.set(name.as_str(), value_str);
                }
            }
            // Extract standard fields
            if let Some(request_id) = __context_headers.get("x-request-id")
                .and_then(|v| v.to_str().ok())
            {
                __ctx.set_request_id(request_id);
            }
            __ctx
        }
    };

    (extraction, call)
}

/// Generate Context extraction code for CLI-based protocols
///
/// This creates code that populates Context from environment variables.
///
/// Returns `(extraction, call)` - though for CLI there's no extractor needed,
/// just the call to create the Context.
#[allow(dead_code)] // Will be used when CLI macro adds Context support
pub fn generate_cli_context_extraction() -> (TokenStream2, TokenStream2) {
    let extraction = quote! {}; // No extractor needed for CLI

    let call = quote! {
        {
            let mut __ctx = ::server_less::Context::new();
            // Populate from environment variables
            for (key, value) in ::std::env::vars() {
                __ctx.set(format!("env:{}", key), value);
            }
            __ctx
        }
    };

    (extraction, call)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_is_qualified_context() {
        let ty: Type = parse_quote! { server_less::Context };
        assert!(is_qualified_context(&ty));

        let ty: Type = parse_quote! { ::server_less::Context };
        assert!(is_qualified_context(&ty));
    }

    #[test]
    fn test_is_bare_context() {
        let ty: Type = parse_quote! { Context };
        assert!(is_bare_context(&ty));

        let ty: Type = parse_quote! { server_less::Context };
        assert!(!is_bare_context(&ty));
    }

    #[test]
    fn test_should_inject_context() {
        let bare_ctx: Type = parse_quote! { Context };
        let qualified_ctx: Type = parse_quote! { server_less::Context };

        // No qualified in impl block - inject both
        assert!(should_inject_context(&bare_ctx, false));
        assert!(should_inject_context(&qualified_ctx, false));

        // Qualified exists in impl block - only inject qualified
        assert!(!should_inject_context(&bare_ctx, true));
        assert!(should_inject_context(&qualified_ctx, true));
    }
}
