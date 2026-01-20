//! OpenRPC specification generation.
//!
//! Generates OpenRPC 1.0 specifications from impl blocks.
//! OpenRPC is to JSON-RPC what OpenAPI is to REST.

use heck::ToLowerCamelCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, ItemImpl, Token};

use crate::parse::{extract_methods, get_impl_name, MethodInfo, ParamInfo};

/// Arguments for the #[openrpc] attribute
#[derive(Default)]
pub struct OpenRpcArgs {
    /// Service title
    pub title: Option<String>,
    /// Service version
    pub version: Option<String>,
}

impl Parse for OpenRpcArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = OpenRpcArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "title" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.title = Some(lit.value());
                }
                "version" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: title, version"),
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

/// Expand the #[openrpc] attribute macro
pub fn expand_openrpc(args: OpenRpcArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let title = args.title.unwrap_or_else(|| struct_name_str.clone());
    let version = args.version.unwrap_or_else(|| "1.0.0".to_string());

    // Generate method specs
    let method_specs: Vec<String> = methods
        .iter()
        .map(|m| generate_method_spec(m))
        .collect();

    let methods_json = method_specs.join(",\n");

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get the OpenRPC specification for this service.
            pub fn openrpc_spec() -> ::trellis::serde_json::Value {
                ::trellis::serde_json::json!({
                    "openrpc": "1.0.0",
                    "info": {
                        "title": #title,
                        "version": #version
                    },
                    "methods": Self::openrpc_methods()
                })
            }

            /// Get the OpenRPC methods array.
            fn openrpc_methods() -> Vec<::trellis::serde_json::Value> {
                let methods_str = concat!("[", #methods_json, "]");
                ::trellis::serde_json::from_str(methods_str).unwrap_or_default()
            }

            /// Get the OpenRPC spec as a JSON string.
            pub fn openrpc_json() -> String {
                ::trellis::serde_json::to_string_pretty(&Self::openrpc_spec())
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
    let name = method.name.to_string().to_lower_camel_case();
    let description = method.docs.clone().unwrap_or_default();

    let params: Vec<String> = method
        .params
        .iter()
        .map(|p| generate_param_spec(p))
        .collect();

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
    let name = param.name.to_string().to_lower_camel_case();
    let schema = get_json_schema(&Some(param.ty.clone()));
    let required = !param.is_optional;

    format!(
        r#"{{
            "name": "{}",
            "required": {},
            "schema": {}
        }}"#,
        name, required, schema
    )
}

/// Get JSON Schema for a type
fn get_json_schema(ty: &Option<syn::Type>) -> String {
    let Some(ty) = ty else {
        return r#"{"type": "null"}"#.to_string();
    };

    let type_str = quote::quote!(#ty).to_string();

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
    } else if type_str.contains("Vec") {
        r#"{"type": "array"}"#.to_string()
    } else if type_str.contains("Option") {
        // For Option<T>, we could extract T but keep it simple
        r#"{"type": "string"}"#.to_string()
    } else {
        r#"{"type": "object"}"#.to_string()
    }
}
