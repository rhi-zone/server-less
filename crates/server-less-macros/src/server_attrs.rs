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

/// Known flags accepted by `#[server(...)]` on methods.
const KNOWN_SERVER_FLAGS: &[&str] = &["skip", "hidden", "name", "group"];

/// Returns `true` if the method has `#[server(skip)]`.
pub(crate) fn has_server_skip(method: &MethodInfo) -> bool {
    has_server_flag(method, "skip")
}

/// Returns `true` if the method has `#[server(hidden)]`.
pub(crate) fn has_server_hidden(method: &MethodInfo) -> bool {
    has_server_flag(method, "hidden")
}

/// Validate all `#[server(...)]` attributes on a method, returning an error for
/// unknown keys.  Call this once per method in each protocol's expand function,
/// before using the bool-returning helpers, so that typos like `#[server(skiip)]`
/// produce a proper diagnostic rather than silently having no effect.
pub(crate) fn validate_server_attrs(method: &MethodInfo) -> syn::Result<()> {
    for attr in &method.method.attrs {
        if attr.path().is_ident("server") {
            attr.parse_nested_meta(|meta| {
                // Consume optional `= value`.
                if meta.input.peek(syn::Token![=]) {
                    let _: proc_macro2::TokenStream = meta.value()?.parse()?;
                    return Ok(());
                }
                let key = meta
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default();
                if !KNOWN_SERVER_FLAGS.iter().any(|&k| k == key) {
                    let suggestion = crate::did_you_mean(&key, KNOWN_SERVER_FLAGS)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(meta.error(format!(
                        "unknown `#[server]` attribute `{key}`{suggestion}\n\
                         \n\
                         Valid attributes: skip, hidden, name, group"
                    )));
                }
                Ok(())
            })?;
        }
    }
    Ok(())
}

fn has_server_flag(method: &MethodInfo, flag: &str) -> bool {
    for attr in &method.method.attrs {
        if attr.path().is_ident("server") {
            let mut found = false;
            // Accept any combination of keys; just look for the one we care about.
            // Unknown-key validation is handled by validate_server_attrs().
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
