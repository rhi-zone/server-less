//! Serve coordination macro.
//!
//! Combines multiple protocol handlers into a single server.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, Ident, ItemImpl, Token};

use crate::parse::get_impl_name;

/// Arguments for the #[serve] attribute
#[derive(Default)]
pub struct ServeArgs {
    /// Protocols to serve (http, ws)
    pub protocols: Vec<String>,
    /// Health check path (default: /health)
    pub health_path: Option<String>,
}

impl Parse for ServeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ServeArgs::default();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let ident_str = ident.to_string();

            match ident_str.as_str() {
                "http" | "ws" => {
                    args.protocols.push(ident_str);
                }
                "health" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitStr = input.parse()?;
                    args.health_path = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown protocol `{other}`. Valid: http, ws, health"),
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

/// Expand the #[serve] attribute macro
pub fn expand_serve(args: ServeArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
    let struct_name = get_impl_name(&impl_block)?;

    let health_path = args.health_path.unwrap_or_else(|| "/health".to_string());

    // Build router combination based on protocols
    let router_setup = generate_router_setup(&args.protocols);

    // Generate the serve method
    let serve_impl = quote! {
        impl #struct_name {
            /// Start serving all configured protocols.
            ///
            /// This combines HTTP and WebSocket routers (as configured) and starts
            /// an axum server on the given address.
            pub async fn serve(self, addr: impl ::std::convert::AsRef<str>) -> ::std::io::Result<()>
            where
                Self: Clone + Send + Sync + 'static,
            {
                #router_setup

                // Add health check
                let router = router.route(
                    #health_path,
                    ::axum::routing::get(|| async { "ok" })
                );

                let listener = ::tokio::net::TcpListener::bind(addr.as_ref()).await?;
                ::axum::serve(listener, router).await
            }

            /// Build the combined router without starting the server.
            ///
            /// Useful for testing or custom server setup.
            pub fn router(self) -> ::axum::Router
            where
                Self: Clone + Send + Sync + 'static,
            {
                #router_setup

                router.route(
                    #health_path,
                    ::axum::routing::get(|| async { "ok" })
                )
            }
        }
    };

    Ok(quote! {
        #impl_block

        #serve_impl
    })
}

/// Generate router setup code based on enabled protocols
fn generate_router_setup(protocols: &[String]) -> TokenStream {
    let has_http = protocols.contains(&"http".to_string());
    let has_ws = protocols.contains(&"ws".to_string());

    match (has_http, has_ws) {
        (true, true) => quote! {
            let http_router = self.clone().http_router();
            let ws_router = self.ws_router();
            let router = http_router.merge(ws_router);
        },
        (true, false) => quote! {
            let router = self.http_router();
        },
        (false, true) => quote! {
            let router = self.ws_router();
        },
        (false, false) => quote! {
            let router = ::axum::Router::new();
        },
    }
}
