//! JSON Schema generation macro.
//!
//! Generates JSON Schema definitions from Rust impl blocks.
//! Useful for API validation, documentation, and tooling.
//!
//! # JSON Schema
//!
//! Standard for describing JSON data structures:
//! - Validates request/response formats
//! - Enables IDE autocompletion
//! - Powers form generation
//! - Language-agnostic type definitions
//!
//! # Schema Generation
//!
//! Creates schemas for:
//! - Method parameters (request schema)
//! - Return types (response schema)
//! - Required vs optional fields
//! - Type information
//!
//! # Generated Methods
//!
//! - `jsonschema() -> serde_json::Value` - Complete JSON Schema
//!
//! # Example
//!
//! ```ignore
//! use server_less::jsonschema;
//!
//! struct UserService;
//!
//! #[jsonschema(title = "User API")]
//! impl UserService {
//!     /// Create a user
//!     fn create_user(&self, name: String, age: Option<i32>) -> String {
//!         name
//!     }
//! }
//!
//! let schema = UserService::jsonschema();
//! ```

use heck::ToLowerCamelCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[jsonschema] attribute
#[derive(Default)]
pub(crate) struct JsonSchemaArgs {
    /// Schema title
    title: Option<String>,
    /// Draft version (default: draft-07)
    draft: Option<String>,
}

impl Parse for JsonSchemaArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = JsonSchemaArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "title" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.title = Some(lit.value());
                }
                "draft" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.draft = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: title, draft"),
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

pub(crate) fn expand_jsonschema(
    args: JsonSchemaArgs,
    impl_block: ItemImpl,
) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let title = args.title.unwrap_or_else(|| struct_name_str.clone());
    let draft = args
        .draft
        .unwrap_or_else(|| "http://json-schema.org/draft-07/schema#".to_string());

    // Generate schema definitions for each method
    let definitions: Vec<String> = methods
        .iter()
        .flat_map(generate_schema_definitions)
        .collect();

    let definitions_json = definitions.join(",\n");

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get JSON Schema for all request/response types.
            pub fn json_schema() -> ::server_less::serde_json::Value {
                let defs_str = concat!("{", #definitions_json, "}");
                let definitions: ::server_less::serde_json::Value =
                    ::server_less::serde_json::from_str(defs_str).unwrap_or_default();

                ::server_less::serde_json::json!({
                    "$schema": #draft,
                    "title": #title,
                    "definitions": definitions
                })
            }

            /// Get JSON Schema as a pretty-printed string.
            pub fn json_schema_string() -> String {
                ::server_less::serde_json::to_string_pretty(&Self::json_schema())
                    .unwrap_or_else(|_| "{}".to_string())
            }

            /// Write JSON Schema to a file.
            pub fn write_json_schema(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::json_schema_string())
            }
        }
    })
}

/// Generate JSON Schema definitions for a method's request/response
fn generate_schema_definitions(method: &MethodInfo) -> Vec<String> {
    let method_name = method.name.to_string();
    let request_name = format!("{}Request", capitalize(&method_name));
    let response_name = format!("{}Response", capitalize(&method_name));

    // Generate request schema
    let request_props: Vec<String> = method.params.iter().map(generate_property).collect();

    let required_fields: Vec<String> = method
        .params
        .iter()
        .filter(|p| !p.is_optional)
        .map(|p| format!("\"{}\"", p.name.to_string().to_lower_camel_case()))
        .collect();

    let request_schema = if request_props.is_empty() {
        format!(
            r#"
        "{}": {{
            "type": "object",
            "properties": {{}},
            "additionalProperties": false
        }}"#,
            request_name
        )
    } else {
        format!(
            r#"
        "{}": {{
            "type": "object",
            "properties": {{
                {}
            }},
            "required": [{}],
            "additionalProperties": false
        }}"#,
            request_name,
            request_props.join(",\n                "),
            required_fields.join(", ")
        )
    };

    // Generate response schema
    let ret = &method.return_info;
    let response_schema = if ret.is_unit {
        format!(
            r#"
        "{}": {{
            "type": "object",
            "properties": {{}},
            "additionalProperties": false
        }}"#,
            response_name
        )
    } else {
        let result_schema = get_type_schema(&ret.ty);
        format!(
            r#"
        "{}": {{
            "type": "object",
            "properties": {{
                "result": {}
            }},
            "required": ["result"],
            "additionalProperties": false
        }}"#,
            response_name, result_schema
        )
    };

    vec![request_schema, response_schema]
}

/// Generate a JSON Schema property
fn generate_property(param: &ParamInfo) -> String {
    let name = param.name.to_string().to_lower_camel_case();
    let schema = get_type_schema(&Some(param.ty.clone()));
    format!(r#""{}": {}"#, name, schema)
}

/// Get JSON Schema for a type
fn get_type_schema(ty: &Option<syn::Type>) -> String {
    let Some(ty) = ty else {
        return r#"{"type": "null"}"#.to_string();
    };

    let type_str = quote!(#ty).to_string();

    // Check container types first (note: quote! adds spaces)
    if type_str.contains("Vec<") || type_str.contains("Vec <") {
        r#"{"type": "array", "items": {}}"#.to_string()
    } else if type_str.contains("Option<") || type_str.contains("Option <") {
        // For simplicity, just allow null - could be enhanced to parse inner type
        r#"{"type": ["null", "object"]}"#.to_string()
    } else if type_str.contains("HashMap") || type_str.contains("BTreeMap") {
        r#"{"type": "object", "additionalProperties": true}"#.to_string()
    } else if type_str.contains("String") || type_str.contains("str") {
        r#"{"type": "string"}"#.to_string()
    } else if type_str.contains("i8")
        || type_str.contains("i16")
        || type_str.contains("i32")
        || type_str.contains("i64")
        || type_str.contains("u8")
        || type_str.contains("u16")
        || type_str.contains("u32")
        || type_str.contains("u64")
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

/// Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}
