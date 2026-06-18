//! `#[derive(HealthCheck)]` — a standalone health-check endpoint.
//!
//! The `#[server]` preset already mounts a `/health` route; this derive exposes
//! the same capability for hand-rolled routers that don't use `#[server]`. It
//! generates a `health_router()` method returning an `axum::Router` with a single
//! `GET` route that responds with a fixed status string.
//!
//! ```ignore
//! #[derive(HealthCheck)]
//! struct MyService;
//!
//! // default: GET /health -> "ok"
//! let app = MyService.health_router().merge(other_routes);
//!
//! // override path and body:
//! #[derive(HealthCheck)]
//! #[health(path = "/healthz", status = "alive")]
//! struct Probe;
//! ```

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, LitStr};

struct HealthArgs {
    path: String,
    status: String,
}

impl Default for HealthArgs {
    fn default() -> Self {
        HealthArgs {
            path: "/health".to_string(),
            status: "ok".to_string(),
        }
    }
}

/// Parse the struct-level `#[health(path = "...", status = "...")]` helper attribute.
fn parse_health_args(input: &DeriveInput) -> syn::Result<HealthArgs> {
    let mut args = HealthArgs::default();
    for attr in &input.attrs {
        if !attr.path().is_ident("health") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("path") {
                let lit: LitStr = meta.value()?.parse()?;
                args.path = lit.value();
                Ok(())
            } else if meta.path.is_ident("status") {
                let lit: LitStr = meta.value()?.parse()?;
                args.status = lit.value();
                Ok(())
            } else {
                Err(meta.error(
                    "unknown `#[health(...)]` option — valid options: path, status\n\
                     \n\
                     Example: #[health(path = \"/healthz\", status = \"alive\")]",
                ))
            }
        })?;
    }
    Ok(args)
}

pub fn expand_health_check(input: DeriveInput) -> syn::Result<TokenStream2> {
    let args = parse_health_args(&input)?;
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let path = &args.path;
    let status = &args.status;

    let doc = format!(
        "Build an axum [`Router`] exposing a health-check endpoint at `GET {}` \
         (responds with `{}`).",
        path, status
    );

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            #[doc = #doc]
            pub fn health_router(&self) -> ::server_less::axum::Router {
                ::server_less::axum::Router::new().route(
                    #path,
                    ::server_less::axum::routing::get(|| async { #status }),
                )
            }
        }
    })
}
