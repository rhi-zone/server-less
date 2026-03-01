//! Helpers for cross-protocol `#[server(...)]` method attributes.
//!
//! `#[server(skip)]` excludes a method from every protocol derive.
//! `#[server(hidden)]` exposes the method but hides it from help/docs in
//! protocols that have such a concept (CLI help, OpenAPI spec).
//!
//! Each protocol macro calls these helpers in addition to its own per-protocol
//! checks. Because `#[server]` is registered as a pass-through when applied to
//! non-impl-block items, it survives being re-emitted without causing errors.

use server_less_parse::MethodInfo;

/// Returns `true` if the method has `#[server(skip)]`.
pub(crate) fn has_server_skip(method: &MethodInfo) -> bool {
    has_server_flag(method, "skip")
}

/// Returns `true` if the method has `#[server(hidden)]`.
pub(crate) fn has_server_hidden(method: &MethodInfo) -> bool {
    has_server_flag(method, "hidden")
}

fn has_server_flag(method: &MethodInfo, flag: &str) -> bool {
    for attr in &method.method.attrs {
        if attr.path().is_ident("server") {
            let mut found = false;
            // Accept any combination of keys; just look for the one we care about.
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(flag) {
                    found = true;
                }
                // Consume optional `= value` so multi-key attrs don't stall parsing.
                if meta.input.peek(syn::Token![=]) {
                    let _: proc_macro2::TokenStream = meta.value()?.parse()?;
                }
                Ok(())
            });
            if found {
                return true;
            }
        }
    }
    false
}
