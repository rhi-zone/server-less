//! Implementation of `#[derive(Config)]`.
//!
//! Generates a [`server_less_core::config::Config`] impl for a struct with named fields.

use heck::ToShoutySnakeCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use server_less_parse::parse_param_attrs;

/// Metadata extracted from a single struct field.
struct FieldMeta {
    name: syn::Ident,
    ty: syn::Type,
    /// Explicit env var from `#[param(env = "VAR")]`.
    env_var: Option<String>,
    /// Dotted file key from `#[param(file_key = "a.b.c")]`.
    file_key: Option<String>,
    /// Default value string from `#[param(default = ...)]`.
    default_value: Option<String>,
    /// Help text from `#[param(help = "...")]`.
    help_text: Option<String>,
    /// Whether the field type is `Option<T>`.
    is_option: bool,
}

fn is_option_type(ty: &syn::Type) -> bool {
    inner_option_type(ty).is_some()
}

/// Extract the `T` from `Option<T>`, or `None` if the type isn't `Option<T>`.
fn inner_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(tp) = ty
        && let Some(seg) = tp.path.segments.last()
        && seg.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
}

pub fn expand_config(input: DeriveInput) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    struct_name,
                    "#[derive(Config)] only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "#[derive(Config)] only supports structs, not enums or unions",
            ));
        }
    };

    let mut field_metas = Vec::new();
    for field in fields {
        let name = field.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(field, "Config fields must be named")
        })?;
        let ty = field.ty.clone();
        let is_option = is_option_type(&ty);

        let param_attrs = parse_param_attrs(&field.attrs)?;

        field_metas.push(FieldMeta {
            name: name.clone(),
            ty,
            env_var: param_attrs.env_var,
            file_key: param_attrs.file_key,
            default_value: param_attrs.default_value,
            help_text: param_attrs.help_text,
            is_option,
        });
    }

    let load_impl = generate_load(&field_metas, struct_name)?;
    let field_meta_impl = generate_field_meta(&field_metas, struct_name);

    Ok(quote! {
        impl ::server_less_core::config::Config for #struct_name {
            fn load(sources: &[::server_less_core::config::ConfigSource]) -> ::std::result::Result<Self, ::server_less_core::config::ConfigError> {
                #load_impl
            }

            fn field_meta() -> &'static [::server_less_core::config::ConfigFieldMeta] {
                #field_meta_impl
            }
        }
    })
}

fn generate_load(fields: &[FieldMeta], struct_name: &syn::Ident) -> syn::Result<TokenStream2> {
    // Variable declarations: one Option<String> per field holding the raw string value.
    let var_decls: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            quote! { let mut #name: ::std::option::Option<String> = None; }
        })
        .collect();

    // Defaults branch: apply compile-time defaults.
    let defaults_branches: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            if let Some(ref default) = f.default_value {
                quote! {
                    if #name.is_none() {
                        #name = ::std::option::Option::Some(#default.to_string());
                    }
                }
            } else {
                quote! {}
            }
        })
        .collect();

    // Env branch: read each field from environment.
    let env_branches: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let name_str = name.to_string();
            let field_upper = to_shouty(&name_str);

            if let Some(ref explicit_var) = f.env_var {
                // Explicit var name — no prefix applied.
                quote! {
                    if let ::std::result::Result::Ok(val) = ::std::env::var(#explicit_var) {
                        #name = ::std::option::Option::Some(val);
                    }
                }
            } else {
                // Generate `{PREFIX}_{FIELD}` or just `{FIELD}` when prefix is None.
                quote! {
                    {
                        let var_name = match prefix {
                            ::std::option::Option::Some(ref p) if !p.is_empty() => {
                                format!("{}_{}", p.to_uppercase(), #field_upper)
                            }
                            _ => #field_upper.to_string(),
                        };
                        if let ::std::result::Result::Ok(val) = ::std::env::var(&var_name) {
                            #name = ::std::option::Option::Some(val);
                        }
                    }
                }
            }
        })
        .collect();

    // File branch: look up each field's key in the flat TOML map.
    let file_branches: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let key = f
                .file_key
                .clone()
                .unwrap_or_else(|| f.name.to_string());
            quote! {
                if let ::std::option::Option::Some(val) = toml_map.get(#key) {
                    #name = ::std::option::Option::Some(val.clone());
                }
            }
        })
        .collect();

    // Final struct construction: parse each raw string to the target type.
    let field_constructions: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let ty = &f.ty;
            let name_str = name.to_string();

            if let Some(inner_ty) = inner_option_type(ty) {
                // Option<T>: None if no value, Some(parse(v)) if a value exists.
                quote! {
                    #name: match #name {
                        ::std::option::Option::None => ::std::option::Option::None,
                        ::std::option::Option::Some(ref s) => {
                            let parsed: #inner_ty = s.parse().map_err(|e: <#inner_ty as ::std::str::FromStr>::Err| {
                                ::server_less_core::config::ConfigError::ParseError {
                                    field: #name_str,
                                    source: "string".to_string(),
                                    message: e.to_string(),
                                }
                            })?;
                            ::std::option::Option::Some(parsed)
                        }
                    },
                }
            } else {
                quote! {
                    #name: {
                        let raw = #name.ok_or(::server_less_core::config::ConfigError::MissingField { field: #name_str })?;
                        raw.parse::<#ty>().map_err(|e| ::server_less_core::config::ConfigError::ParseError {
                            field: #name_str,
                            source: "string".to_string(),
                            message: e.to_string(),
                        })?
                    },
                }
            }
        })
        .collect();

    Ok(quote! {
        #(#var_decls)*

        for source in sources {
            match source {
                ::server_less_core::config::ConfigSource::Defaults => {
                    #(#defaults_branches)*
                }
                ::server_less_core::config::ConfigSource::Env { prefix } => {
                    #(#env_branches)*
                }
                ::server_less_core::config::ConfigSource::File(path) => {
                    #[cfg(feature = "server-less-core/config")]
                    {
                        match ::server_less_core::config::load_toml_file(path)? {
                            ::std::option::Option::Some(toml_map) => {
                                #(#file_branches)*
                            }
                            ::std::option::Option::None => {} // file not found, skip
                        }
                    }
                    #[cfg(not(feature = "server-less-core/config"))]
                    {
                        // TOML file loading requires the `config` feature on server-less-core.
                        // Without it, File sources are silently skipped.
                        let _ = path;
                    }
                }
            }
        }

        ::std::result::Result::Ok(#struct_name {
            #(#field_constructions)*
        })
    })
}

fn generate_field_meta(fields: &[FieldMeta], _struct_name: &syn::Ident) -> TokenStream2 {
    let entries: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let name_str = f.name.to_string();
            let ty = &f.ty;
            let env_var = match &f.env_var {
                Some(v) => quote! { ::std::option::Option::Some(#v) },
                None => quote! { ::std::option::Option::None },
            };
            let file_key = match &f.file_key {
                Some(k) => quote! { ::std::option::Option::Some(#k) },
                None => quote! { ::std::option::Option::None },
            };
            let default = match &f.default_value {
                Some(d) => quote! { ::std::option::Option::Some(#d) },
                None => quote! { ::std::option::Option::None },
            };
            let help = match &f.help_text {
                Some(h) => quote! { ::std::option::Option::Some(#h) },
                None => quote! { ::std::option::Option::None },
            };
            let required = !f.is_option && f.default_value.is_none();
            let type_name_str = quote!(#ty).to_string();
            quote! {
                ::server_less_core::config::ConfigFieldMeta {
                    name: #name_str,
                    type_name: #type_name_str,
                    env_var: #env_var,
                    file_key: #file_key,
                    default: #default,
                    help: #help,
                    required: #required,
                }
            }
        })
        .collect();

    let count = entries.len();
    quote! {
        static META: [::server_less_core::config::ConfigFieldMeta; #count] = [
            #(#entries,)*
        ];
        &META
    }
}

fn to_shouty(s: &str) -> String {
    s.to_shouty_snake_case()
}
