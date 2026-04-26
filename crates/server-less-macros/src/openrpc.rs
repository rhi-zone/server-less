//! OpenRPC specification generation macro.
//!
//! Generates OpenRPC 1.0 specifications from impl blocks.
//! OpenRPC is to JSON-RPC what OpenAPI is to REST.
//!
//! # OpenRPC
//!
//! Machine-readable specification for JSON-RPC APIs:
//! - Describes methods, parameters, and return types
//! - Enables automatic client generation
//! - Powers interactive documentation tools
//!
//! # Generated Specification
//!
//! Creates a complete OpenRPC document:
//! - Method definitions with doc comments
//! - Parameter schemas (JSON Schema format)
//! - Result type schemas
//! - Optional/required parameter marking
//!
//! # Generated Methods
//!
//! - `openrpc_spec() -> serde_json::Value` - Complete OpenRPC specification
//!
//! # Example
//!
//! ```ignore
//! use server_less::openrpc;
//!
//! struct Calculator;
//!
//! #[openrpc(title = "Calculator API")]
//! impl Calculator {
//!     /// Add two numbers
//!     fn add(&self, a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//! }
//!
//! let spec = Calculator::openrpc_spec();
//! ```

use crate::app::extract_app_meta;
use crate::server_attrs::{has_server_hidden, has_server_skip, validate_server_attrs};
use heck::ToLowerCamelCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{
    MethodInfo, ParamInfo, extract_methods, get_impl_name, unwrap_option_type, unwrap_result_ok_type,
    unwrap_vec_type,
};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[openrpc] attribute
#[derive(Default)]
pub(crate) struct OpenRpcArgs {
    /// Service title
    title: Option<String>,
    /// Service version
    version: Option<String>,
}

impl Parse for OpenRpcArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = OpenRpcArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "name" | "title" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.title = Some(lit.value());
                }
                "version" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                other => {
                    const VALID: &[&str] = &["title", "version"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}. Valid arguments: name, version"
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

pub(crate) fn expand_openrpc(args: OpenRpcArgs, mut impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    crate::reject_generic_impl(&impl_block)?;
    let app_meta = extract_app_meta(&mut impl_block.attrs);
    let struct_name = get_impl_name(&impl_block)?;
    let (impl_generics, _ty_generics, where_clause) = impl_block.generics.split_for_impl();
    let self_ty = &impl_block.self_ty;
    let struct_name_str = struct_name.to_string();
    let all_methods = extract_methods(&impl_block)?;
    for m in &all_methods {
        validate_server_attrs(m)?;
    }
    let methods: Vec<_> = all_methods
        .into_iter()
        .filter(|m| !has_server_skip(m) && !has_server_hidden(m))
        .collect();

    let title = args
        .title
        .or(app_meta.name)
        .unwrap_or_else(|| struct_name_str.clone());
    let version = args
        .version
        .or_else(|| app_meta.version.into_explicit())
        .unwrap_or_else(|| "1.0.0".to_string());
    // M12: capture description and homepage for the OpenRPC info block.
    let description = app_meta.description;
    let homepage = app_meta.homepage;

    // Generate method specs
    let method_specs: Vec<String> = methods.iter().map(generate_method_spec).collect();

    let methods_json = method_specs.join(",\n");

    // M12: build optional info fields for description and homepage.
    let description_field = match &description {
        Some(desc) => quote! {
            if let Some(__obj) = __info.as_object_mut() {
                __obj.insert("description".to_string(), ::server_less::serde_json::json!(#desc));
            }
        },
        None => quote! {},
    };
    let homepage_field = match &homepage {
        Some(url) => quote! {
            if let Some(__obj) = __info.as_object_mut() {
                __obj.insert("contact".to_string(), ::server_less::serde_json::json!({"url": #url}));
            }
        },
        None => quote! {},
    };

    // Only emit the impl block if no higher-priority protocol sibling is present.
    let maybe_impl = if crate::is_protocol_impl_emitter(&impl_block, "openrpc") {
        quote! { #impl_block }
    } else {
        quote! {}
    };

    Ok(quote! {
        #maybe_impl

        impl #impl_generics #self_ty #where_clause {
            /// Get the OpenRPC specification for this service.
            pub fn openrpc_spec() -> ::server_less::serde_json::Value {
                let mut __info = ::server_less::serde_json::json!({
                    "title": #title,
                    "version": #version
                });
                #description_field
                #homepage_field
                ::server_less::serde_json::json!({
                    "openrpc": "1.0.0",
                    "info": __info,
                    "methods": Self::openrpc_methods()
                })
            }

            /// Get the OpenRPC methods array.
            fn openrpc_methods() -> Vec<::server_less::serde_json::Value> {
                let methods_str = concat!("[", #methods_json, "]");
                ::server_less::serde_json::from_str(methods_str).unwrap_or_default()
            }

            /// Get the OpenRPC spec as a JSON string.
            pub fn openrpc_json() -> String {
                ::server_less::serde_json::to_string_pretty(&Self::openrpc_spec())
                    .unwrap_or_else(|_| "{}".to_string())
            }

            /// Write the OpenRPC spec to a file.
            pub fn write_openrpc(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::openrpc_json())
            }
        }
    })
}

/// Generate OpenRPC method specification
fn generate_method_spec(method: &MethodInfo) -> String {
    let name = method.name_str().to_lower_camel_case();
    let description = method.docs.clone().unwrap_or_default();

    let params: Vec<String> = method.params.iter().map(generate_param_spec).collect();

    let result_schema = get_json_schema(&method.return_info.ty);

    format!(
        r#"{{
            "name": "{}",
            "description": "{}",
            "params": [{}],
            "result": {{
                "name": "result",
                "schema": {}
            }}
        }}"#,
        name,
        description.replace('"', "\\\""),
        params.join(", "),
        result_schema
    )
}

/// Generate parameter specification
fn generate_param_spec(param: &ParamInfo) -> String {
    let name = param.name_str().to_lower_camel_case();
    // M11: include help_text as the "description" field in the param spec.
    let description = param
        .help_text
        .as_deref()
        .unwrap_or("")
        .replace('"', "\\\"");
    let schema = get_json_schema(&Some(param.ty.clone()));
    let required = !param.is_optional;

    format!(
        r#"{{
            "name": "{}",
            "description": "{}",
            "required": {},
            "schema": {}
        }}"#,
        name, description, required, schema
    )
}

/// Get JSON Schema for a type
fn get_json_schema(ty: &Option<syn::Type>) -> String {
    let Some(ty) = ty else {
        return r#"{"type": "null"}"#.to_string();
    };
    get_json_schema_ty(ty)
}

/// Get JSON Schema for a `syn::Type` reference.
fn get_json_schema_ty(ty: &syn::Type) -> String {
    // Unwrap Result<T, E> → T
    if let Some(ok) = unwrap_result_ok_type(ty) {
        return get_json_schema_ty(ok);
    }
    // M15: Option<T> → {"anyOf": [{"type": "null"}, <inner_schema>]}
    // Bare `null` is not valid JSON Schema; use {"type": "null"} instead.
    if let Some(inner) = unwrap_option_type(ty) {
        let inner_schema = get_json_schema_ty(inner);
        return format!(r#"{{"anyOf": [{{"type": "null"}}, {}]}}"#, inner_schema);
    }
    // Vec<T> → {"type": "array", "items": <inner_schema>}
    if let Some(inner) = unwrap_vec_type(ty) {
        let inner_schema = get_json_schema_ty(inner);
        return format!(r#"{{"type": "array", "items": {}}}"#, inner_schema);
    }
    let type_str = quote!(#ty).to_string();
    if type_str.contains("String") || type_str.contains("str") {
        r#"{"type": "string"}"#.to_string()
    } else if type_str.contains("i8")
        || type_str.contains("i16")
        || type_str.contains("i32")
        || type_str.contains("i64")
        || type_str.contains("u8")
        || type_str.contains("u16")
        || type_str.contains("u32")
        || type_str.contains("u64")
        || type_str.contains("isize")
        || type_str.contains("usize")
    {
        r#"{"type": "integer"}"#.to_string()
    } else if type_str.contains("f32") || type_str.contains("f64") {
        r#"{"type": "number"}"#.to_string()
    } else if type_str.contains("bool") {
        r#"{"type": "boolean"}"#.to_string()
    } else {
        r#"{"type": "object"}"#.to_string()
    }
}
