//! Error derive macro for generating IntoErrorCode implementations.
//!
//! ```ignore
//! #[derive(TrellisError)]
//! enum MyError {
//!     #[error(code = NotFound, message = "User not found")]
//!     UserNotFound,
//!     #[error(code = InvalidInput)]
//!     ValidationFailed(String),
//!     // Code inferred from variant name
//!     Unauthorized,
//! }
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Token, parse::Parse, punctuated::Punctuated};

/// Arguments for the #[error(...)] attribute on variants
#[derive(Default)]
struct ErrorVariantArgs {
    /// Error code (e.g., NotFound, InvalidInput, or numeric 404)
    code: Option<ErrorCodeSpec>,
    /// Custom message
    message: Option<String>,
}

enum ErrorCodeSpec {
    /// Named error code: NotFound, InvalidInput, etc.
    Named(Ident),
    /// Numeric HTTP status: 404, 500, etc.
    Numeric(u16),
}

impl Parse for ErrorVariantArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ErrorVariantArgs::default();

        let pairs = Punctuated::<syn::Meta, Token![,]>::parse_terminated(input)?;

        for meta in pairs {
            match meta {
                syn::Meta::NameValue(nv) if nv.path.is_ident("code") => {
                    // code = NotFound or code = 404
                    match &nv.value {
                        syn::Expr::Path(path) => {
                            if let Some(ident) = path.path.get_ident() {
                                args.code = Some(ErrorCodeSpec::Named(ident.clone()));
                            }
                        }
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Int(lit),
                            ..
                        }) => {
                            let value: u16 = lit.base10_parse()?;
                            args.code = Some(ErrorCodeSpec::Numeric(value));
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &nv.value,
                                "expected error code name or HTTP status\n\
                                 \n\
                                 Valid names: NotFound, InvalidInput, Unauthorized, Forbidden, InternalError\n\
                                 Or use HTTP status: 400, 404, 500, etc.\n\
                                 \n\
                                 Example: #[error(code = NotFound)]",
                            ));
                        }
                    }
                }
                syn::Meta::NameValue(nv) if nv.path.is_ident("message") => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        args.message = Some(s.value());
                    } else {
                        return Err(syn::Error::new_spanned(
                            &nv.value,
                            "message must be a string literal\n\
                             \n\
                             Example: #[error(code = NotFound, message = \"Resource not found\")]",
                        ));
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "unknown attribute. Valid: code, message",
                    ));
                }
            }
        }

        Ok(args)
    }
}

/// Expand the TrellisError derive macro
pub fn expand_trellis_error(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "TrellisError can only be derived for enums\n\
             \n\
             Hint: Define your errors as an enum:\n\
             \n\
             #[derive(Debug, TrellisError)]\n\
             enum MyError {{\n\
                 #[error(code = NotFound)]\n\
                 NotFound,\n\
             }}",
        ));
    };

    let mut error_code_arms = Vec::new();
    let mut message_arms = Vec::new();
    let mut display_arms = Vec::new();

    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let variant_name_str = variant_name.to_string();

        // Parse #[error(...)] attribute if present
        let args = variant
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("error"))
            .map(|attr| attr.parse_args::<ErrorVariantArgs>())
            .transpose()?
            .unwrap_or_default();

        // Determine error code
        let error_code = match args.code {
            Some(ErrorCodeSpec::Named(ident)) => {
                quote! { ::rhizome_trellis::ErrorCode::#ident }
            }
            Some(ErrorCodeSpec::Numeric(status)) => {
                // Map HTTP status to ErrorCode

                match status {
                    400 => quote! { ::rhizome_trellis::ErrorCode::InvalidInput },
                    401 => quote! { ::rhizome_trellis::ErrorCode::Unauthenticated },
                    403 => quote! { ::rhizome_trellis::ErrorCode::Forbidden },
                    404 => quote! { ::rhizome_trellis::ErrorCode::NotFound },
                    409 => quote! { ::rhizome_trellis::ErrorCode::Conflict },
                    422 => quote! { ::rhizome_trellis::ErrorCode::FailedPrecondition },
                    429 => quote! { ::rhizome_trellis::ErrorCode::RateLimited },
                    500 => quote! { ::rhizome_trellis::ErrorCode::Internal },
                    501 => quote! { ::rhizome_trellis::ErrorCode::NotImplemented },
                    503 => quote! { ::rhizome_trellis::ErrorCode::Unavailable },
                    _ => quote! { ::rhizome_trellis::ErrorCode::Internal },
                }
            }
            None => {
                // Infer from variant name
                quote! { ::rhizome_trellis::ErrorCode::infer_from_name(#variant_name_str) }
            }
        };

        // Determine message
        let message_expr = if let Some(msg) = args.message {
            quote! { #msg.to_string() }
        } else {
            // Use variant name, converting CamelCase to "Camel case"
            let readable = camel_to_sentence(&variant_name_str);
            quote! { #readable.to_string() }
        };

        // Generate match arms based on variant fields
        let (pattern, display_format) = match &variant.fields {
            Fields::Unit => (
                quote! { Self::#variant_name },
                quote! { write!(f, "{}", self.message()) },
            ),
            Fields::Unnamed(fields) => {
                let field_names: Vec<_> = (0..fields.unnamed.len())
                    .map(|i| quote::format_ident!("_{}", i))
                    .collect();
                let pattern = quote! { Self::#variant_name(#(#field_names),*) };

                // If single String field, include it in display
                if fields.unnamed.len() == 1 {
                    (
                        pattern.clone(),
                        quote! { write!(f, "{}: {}", #variant_name_str, _0) },
                    )
                } else {
                    (pattern, quote! { write!(f, "{}", self.message()) })
                }
            }
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let pattern = quote! { Self::#variant_name { #(#field_names),* } };
                (pattern, quote! { write!(f, "{}", self.message()) })
            }
        };

        error_code_arms.push(quote! {
            #pattern => #error_code
        });

        message_arms.push(quote! {
            #pattern => #message_expr
        });

        display_arms.push(quote! {
            #pattern => #display_format
        });
    }

    Ok(quote! {
        impl ::rhizome_trellis::IntoErrorCode for #name {
            fn error_code(&self) -> ::rhizome_trellis::ErrorCode {
                match self {
                    #(#error_code_arms,)*
                }
            }

            fn message(&self) -> String {
                match self {
                    #(#message_arms,)*
                }
            }
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#display_arms,)*
                }
            }
        }

        impl ::std::error::Error for #name {}
    })
}

/// Convert CamelCase to "Camel case" sentence
fn camel_to_sentence(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push(' ');
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}
