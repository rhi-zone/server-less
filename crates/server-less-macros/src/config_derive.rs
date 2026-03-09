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
    /// Default value in code-ready form (strings include surrounding quotes).
    /// Used in `defaults_branches` as a Rust literal: `#default_value.to_string()`.
    default_value: Option<String>,
    /// Default value in display form (strings have quotes stripped).
    /// Used in `ConfigFieldMeta.default` for human-readable output.
    default_display: Option<String>,
    /// Help text from `#[param(help = "...")]`.
    help_text: Option<String>,
    /// Whether the field type is `Option<T>`.
    is_option: bool,
    /// Whether the field is a `#[param(nested)]` sub-struct.
    nested: bool,
    /// Whether the nested field uses serde-passthrough (`#[param(nested, serde)]`).
    ///
    /// When `true`, the TOML sub-table is deserialized via `serde::Deserialize`
    /// instead of `Config::load`. Env-var sources are silently skipped.
    nested_serde: bool,
    /// Env-var prefix override for a nested field (`#[param(env_prefix = "SEARCH")]`).
    env_prefix: Option<String>,
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

        let default_display = param_attrs.default_value.as_ref().map(|d| {
            if d.starts_with('"') && d.ends_with('"') && d.len() >= 2 {
                d[1..d.len() - 1].to_string()
            } else {
                d.clone()
            }
        });

        field_metas.push(FieldMeta {
            name: name.clone(),
            ty,
            env_var: param_attrs.env_var,
            file_key: param_attrs.file_key,
            default_value: param_attrs.default_value,
            default_display,
            help_text: param_attrs.help_text,
            is_option,
            nested: param_attrs.nested,
            nested_serde: param_attrs.nested_serde,
            env_prefix: param_attrs.env_prefix,
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
    // Separate fields into leaf (scalar/Option), nested Config sub-structs, and
    // nested serde-passthrough sub-structs.
    let leaf_fields: Vec<&FieldMeta> = fields.iter().filter(|f| !f.nested).collect();
    let nested_fields: Vec<&FieldMeta> = fields
        .iter()
        .filter(|f| f.nested && !f.nested_serde)
        .collect();
    let nested_serde_fields: Vec<&FieldMeta> =
        fields.iter().filter(|f| f.nested_serde).collect();

    // --- Leaf field handling (same as before) ---

    // Variable declarations: one Option<String> per leaf field.
    let var_decls: Vec<TokenStream2> = leaf_fields
        .iter()
        .map(|f| {
            let name = &f.name;
            quote! { let mut #name: ::std::option::Option<String> = None; }
        })
        .collect();

    // Defaults branch for leaf fields.
    let defaults_branches = leaf_fields
        .iter()
        .map(|f| -> syn::Result<TokenStream2> {
            let name = &f.name;
            if let Some(ref default) = f.default_value {
                let default_expr: TokenStream2 = default.parse().map_err(|_| {
                    syn::Error::new(
                        name.span(),
                        format!("failed to parse default value `{default}` as a Rust expression"),
                    )
                })?;
                Ok(quote! {
                    if #name.is_none() {
                        #name = ::std::option::Option::Some(#default_expr.to_string());
                    }
                })
            } else {
                Ok(quote! {})
            }
        })
        .collect::<syn::Result<Vec<TokenStream2>>>()?;

    // Env branch for leaf fields.
    let env_branches: Vec<TokenStream2> = leaf_fields
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
                            ::std::option::Option::Some(p) if !p.is_empty() => {
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

    // File (last-wins) branch for leaf fields.
    let file_branches: Vec<TokenStream2> = leaf_fields
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

    // MergeFile (supplement, don't replace) branch for leaf fields.
    let merge_file_branches: Vec<TokenStream2> = leaf_fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let key = f
                .file_key
                .clone()
                .unwrap_or_else(|| f.name.to_string());
            quote! {
                if #name.is_none() {
                    if let ::std::option::Option::Some(val) = toml_map.get(#key) {
                        #name = ::std::option::Option::Some(val.clone());
                    }
                }
            }
        })
        .collect();

    // --- Nested field handling ---
    //
    // For each #[param(nested)] field, generate a block that:
    //   1. Builds a scoped Vec<ConfigSource> by transforming each source.
    //   2. Calls ChildType::load(&scoped_sources) to get the child value.
    //
    // The scoping rules:
    //   - Defaults       → pass through unchanged
    //   - File(path)     → load raw TOML, extract sub-table by key, pass as TomlTable
    //   - MergeFile(path)→ same but pass as MergeTomlTable
    //   - Env {prefix}   → narrow prefix to "{prefix}_{FIELD_UPPER}" (or override via env_prefix)
    //   - TomlTable(v)   → extract sub-table from already-loaded value, pass as TomlTable
    //   - MergeTomlTable(v) → same with merge semantics
    let nested_var_decls: Vec<TokenStream2> = nested_fields
        .iter()
        .map(|f| -> syn::Result<TokenStream2> {
            let name = &f.name;
            let ty = &f.ty;
            let name_str = name.to_string();
            let field_upper = to_shouty(&name_str);
            // TOML section key: file_key override or field name.
            let toml_key = f.file_key.clone().unwrap_or_else(|| f.name.to_string());

            // The child env prefix token stream.  When env_prefix is set, use it
            // literally (uppercased); otherwise build from the parent prefix + field name.
            let child_prefix_expr: TokenStream2 = if let Some(ref ep) = f.env_prefix {
                let ep_upper = ep.to_uppercase();
                quote! { ::std::option::Option::Some(#ep_upper.to_string()) }
            } else {
                quote! {
                    match prefix {
                        ::std::option::Option::Some(p) if !p.is_empty() => {
                            ::std::option::Option::Some(format!("{}_{}", p.to_uppercase(), #field_upper))
                        }
                        _ => ::std::option::Option::Some(#field_upper.to_string()),
                    }
                }
            };

            Ok(quote! {
                let #name: #ty = {
                    let mut __nested_sources: ::std::vec::Vec<::server_less_core::config::ConfigSource> = ::std::vec::Vec::new();
                    for source in sources {
                        match source {
                            ::server_less_core::config::ConfigSource::Defaults => {
                                __nested_sources.push(::server_less_core::config::ConfigSource::Defaults);
                            }
                            ::server_less_core::config::ConfigSource::File(path) => {
                                if let ::std::option::Option::Some(root_val) =
                                    ::server_less_core::config::load_toml_file_raw(path)?
                                {
                                    if let ::std::option::Option::Some(sub) = root_val.get(#toml_key).cloned() {
                                        __nested_sources.push(::server_less_core::config::ConfigSource::TomlTable(sub));
                                    }
                                }
                            }
                            ::server_less_core::config::ConfigSource::MergeFile(path) => {
                                if let ::std::option::Option::Some(root_val) =
                                    ::server_less_core::config::load_toml_file_raw(path)?
                                {
                                    if let ::std::option::Option::Some(sub) = root_val.get(#toml_key).cloned() {
                                        __nested_sources.push(::server_less_core::config::ConfigSource::MergeTomlTable(sub));
                                    }
                                }
                            }
                            ::server_less_core::config::ConfigSource::Env { prefix } => {
                                let child_prefix = #child_prefix_expr;
                                __nested_sources.push(::server_less_core::config::ConfigSource::Env { prefix: child_prefix });
                            }
                            ::server_less_core::config::ConfigSource::TomlTable(root_val) => {
                                if let ::std::option::Option::Some(sub) = root_val.get(#toml_key).cloned() {
                                    __nested_sources.push(::server_less_core::config::ConfigSource::TomlTable(sub));
                                }
                            }
                            ::server_less_core::config::ConfigSource::MergeTomlTable(root_val) => {
                                if let ::std::option::Option::Some(sub) = root_val.get(#toml_key).cloned() {
                                    __nested_sources.push(::server_less_core::config::ConfigSource::MergeTomlTable(sub));
                                }
                            }
                        }
                    }
                    <#ty as ::server_less_core::config::Config>::load(&__nested_sources)
                        .map_err(|e| {
                            // Prefix the field name to the error for better diagnostics.
                            match e {
                                ::server_less_core::config::ConfigError::MissingField { field } => {
                                    ::server_less_core::config::ConfigError::MissingField {
                                        field: ::std::boxed::Box::leak(
                                            format!("{}.{}", #name_str, field).into_boxed_str()
                                        )
                                    }
                                }
                                other => other,
                            }
                        })?
                };
            })
        })
        .collect::<syn::Result<Vec<TokenStream2>>>()?;

    // --- Leaf field struct construction ---
    let leaf_constructions: Vec<TokenStream2> = leaf_fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let ty = &f.ty;
            let name_str = name.to_string();

            if let Some(inner_ty) = inner_option_type(ty) {
                quote! {
                    #name: match #name {
                        ::std::option::Option::None => ::std::option::Option::None,
                        ::std::option::Option::Some(s) => {
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

    // Nested fields are already bound to local variables of the correct type.
    let nested_constructions: Vec<TokenStream2> = nested_fields
        .iter()
        .map(|f| {
            let name = &f.name;
            quote! { #name, }
        })
        .collect();

    // --- Serde-nested field generation ---
    //
    // For each #[param(nested, serde)] field, generate a block that:
    //   1. Holds an `Option<T>` accumulator (last File wins; MergeFile only fills if None).
    //   2. Iterates sources, skipping Defaults and Env entirely.
    //   3. For File/MergeFile: loads raw TOML, extracts sub-table by key,
    //      deserializes via `toml::Value::try_into::<T>()`.
    //   4. For TomlTable/MergeTomlTable: extracts sub-table by key, deserializes.
    //   5. After the loop, returns `T` (required; use `Option<T>` in the struct for optional serde sections).
    let nested_serde_var_decls: Vec<TokenStream2> = nested_serde_fields
        .iter()
        .map(|f| -> syn::Result<TokenStream2> {
            let name = &f.name;
            let ty = &f.ty;
            let name_str = name.to_string();
            let toml_key = f.file_key.clone().unwrap_or_else(|| f.name.to_string());

            // Determine if the field is Option<T> to allow missing sections.
            let is_opt = f.is_option;

            let missing_handling = if is_opt {
                quote! {
                    ::std::option::Option::None
                }
            } else {
                quote! {
                    return ::std::result::Result::Err(
                        ::server_less_core::config::ConfigError::MissingField { field: #name_str }
                    );
                }
            };

            Ok(quote! {
                let #name: #ty = {
                    // Internal accumulator — Option<T> regardless of field optionality.
                    let mut __serde_val: ::std::option::Option<#ty> = ::std::option::Option::None;

                    for source in sources {
                        match source {
                            // Defaults: skip — serde types handle defaults via #[serde(default)]
                            ::server_less_core::config::ConfigSource::Defaults => {}
                            // Env: skip — env var per-field overrides are unavailable for serde-nested subtrees
                            ::server_less_core::config::ConfigSource::Env { .. } => {}
                            ::server_less_core::config::ConfigSource::File(path) => {
                                if let ::std::option::Option::Some(root_val) =
                                    ::server_less_core::config::load_toml_file_raw(path)?
                                {
                                    if let ::std::option::Option::Some(sub) = root_val.get(#toml_key).cloned() {
                                        let deserialized: #ty = sub.try_into().map_err(|e: ::server_less_core::__toml::de::Error| {
                                            ::server_less_core::config::ConfigError::ParseError {
                                                field: #name_str,
                                                source: "TOML file".to_string(),
                                                message: e.to_string(),
                                            }
                                        })?;
                                        __serde_val = ::std::option::Option::Some(deserialized);
                                    }
                                }
                            }
                            ::server_less_core::config::ConfigSource::MergeFile(path) => {
                                if __serde_val.is_none() {
                                    if let ::std::option::Option::Some(root_val) =
                                        ::server_less_core::config::load_toml_file_raw(path)?
                                    {
                                        if let ::std::option::Option::Some(sub) = root_val.get(#toml_key).cloned() {
                                            let deserialized: #ty = sub.try_into().map_err(|e: ::server_less_core::__toml::de::Error| {
                                                ::server_less_core::config::ConfigError::ParseError {
                                                    field: #name_str,
                                                    source: "TOML file".to_string(),
                                                    message: e.to_string(),
                                                }
                                            })?;
                                            __serde_val = ::std::option::Option::Some(deserialized);
                                        }
                                    }
                                }
                            }
                            ::server_less_core::config::ConfigSource::TomlTable(root_val) => {
                                if let ::std::option::Option::Some(sub) = root_val.get(#toml_key).cloned() {
                                    let deserialized: #ty = sub.try_into().map_err(|e: ::server_less_core::__toml::de::Error| {
                                        ::server_less_core::config::ConfigError::ParseError {
                                            field: #name_str,
                                            source: "TOML table".to_string(),
                                            message: e.to_string(),
                                        }
                                    })?;
                                    __serde_val = ::std::option::Option::Some(deserialized);
                                }
                            }
                            ::server_less_core::config::ConfigSource::MergeTomlTable(root_val) => {
                                if __serde_val.is_none() {
                                    if let ::std::option::Option::Some(sub) = root_val.get(#toml_key).cloned() {
                                        let deserialized: #ty = sub.try_into().map_err(|e: ::server_less_core::__toml::de::Error| {
                                            ::server_less_core::config::ConfigError::ParseError {
                                                field: #name_str,
                                                source: "TOML table".to_string(),
                                                message: e.to_string(),
                                            }
                                        })?;
                                        __serde_val = ::std::option::Option::Some(deserialized);
                                    }
                                }
                            }
                        }
                    }

                    match __serde_val {
                        ::std::option::Option::Some(v) => v,
                        ::std::option::Option::None => { #missing_handling }
                    }
                };
            })
        })
        .collect::<syn::Result<Vec<TokenStream2>>>()?;

    let nested_serde_constructions: Vec<TokenStream2> = nested_serde_fields
        .iter()
        .map(|f| {
            let name = &f.name;
            quote! { #name, }
        })
        .collect();

    Ok(quote! {
        // Leaf field accumulators.
        #(#var_decls)*

        // Nested Config field values — computed in a single pass over sources.
        #(#nested_var_decls)*

        // Serde-nested field values — computed in a single pass over sources.
        #(#nested_serde_var_decls)*

        // Apply sources to leaf fields.
        for source in sources {
            match source {
                ::server_less_core::config::ConfigSource::Defaults => {
                    #(#defaults_branches)*
                }
                ::server_less_core::config::ConfigSource::Env { prefix } => {
                    #(#env_branches)*
                }
                ::server_less_core::config::ConfigSource::File(path) => {
                    match ::server_less_core::config::load_toml_file(path)? {
                        ::std::option::Option::Some(toml_map) => {
                            #(#file_branches)*
                        }
                        ::std::option::Option::None => {} // file not found, skip silently
                    }
                }
                ::server_less_core::config::ConfigSource::MergeFile(path) => {
                    match ::server_less_core::config::load_toml_file(path)? {
                        ::std::option::Option::Some(toml_map) => {
                            #(#merge_file_branches)*
                        }
                        ::std::option::Option::None => {} // file not found, skip silently
                    }
                }
                // TomlTable / MergeTomlTable: flatten the pre-extracted table for
                // leaf fields (used when this struct is itself a nested child).
                ::server_less_core::config::ConfigSource::TomlTable(table_val) => {
                    let mut toml_map = ::std::collections::HashMap::<String, String>::new();
                    ::server_less_core::config::flatten_toml_value("", table_val, &mut toml_map);
                    #(#file_branches)*
                }
                ::server_less_core::config::ConfigSource::MergeTomlTable(table_val) => {
                    let mut toml_map = ::std::collections::HashMap::<String, String>::new();
                    ::server_less_core::config::flatten_toml_value("", table_val, &mut toml_map);
                    #(#merge_file_branches)*
                }
            }
        }

        ::std::result::Result::Ok(#struct_name {
            #(#leaf_constructions)*
            #(#nested_constructions)*
            #(#nested_serde_constructions)*
        })
    })
}

fn generate_field_meta(fields: &[FieldMeta], _struct_name: &syn::Ident) -> TokenStream2 {
    // has_nested: true when any field needs a non-const (OnceLock) initializer.
    // nested_serde fields do NOT call Config::field_meta() so they don't require OnceLock,
    // but regular nested fields do.
    let has_nested = fields.iter().any(|f| f.nested && !f.nested_serde);

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
            let default = match &f.default_display {
                Some(d) => quote! { ::std::option::Option::Some(#d) },
                None => quote! { ::std::option::Option::None },
            };
            let help = if f.nested_serde && f.help_text.is_none() {
                // Default help note for serde-deserialized sections.
                quote! { ::std::option::Option::Some("serde-deserialized from TOML section") }
            } else {
                match &f.help_text {
                    Some(h) => quote! { ::std::option::Option::Some(#h) },
                    None => quote! { ::std::option::Option::None },
                }
            };
            let required = !f.is_option && f.default_value.is_none() && !f.nested;
            let type_name_str = quote!(#ty).to_string();
            let nested_meta = if f.nested && !f.nested_serde {
                // Regular nested: expose child field_meta() for introspection.
                quote! { ::std::option::Option::Some(<#ty as ::server_less_core::config::Config>::field_meta()) }
            } else {
                // Leaf fields and serde-nested fields: opaque, no child introspection.
                quote! { ::std::option::Option::None }
            };
            let env_prefix = match &f.env_prefix {
                Some(ep) => quote! { ::std::option::Option::Some(#ep) },
                None => quote! { ::std::option::Option::None },
            };
            quote! {
                ::server_less_core::config::ConfigFieldMeta {
                    name: #name_str,
                    type_name: #type_name_str,
                    env_var: #env_var,
                    file_key: #file_key,
                    default: #default,
                    help: #help,
                    required: #required,
                    nested: #nested_meta,
                    env_prefix: #env_prefix,
                }
            }
        })
        .collect();

    let count = entries.len();

    if has_nested {
        // Use OnceLock because the nested entries call Config::field_meta() which
        // is not a const fn, so a `static [T; N]` initializer won't compile.
        quote! {
            static META: ::std::sync::OnceLock<
                ::std::vec::Vec<::server_less_core::config::ConfigFieldMeta>
            > = ::std::sync::OnceLock::new();
            META.get_or_init(|| {
                ::std::vec![#(#entries,)*]
            })
        }
    } else {
        quote! {
            static META: [::server_less_core::config::ConfigFieldMeta; #count] = [
                #(#entries,)*
            ];
            &META
        }
    }
}

fn to_shouty(s: &str) -> String {
    s.to_shouty_snake_case()
}
