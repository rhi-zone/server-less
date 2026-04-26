//! Cap'n Proto schema generation macro.
//!
//! Generates Cap'n Proto schemas from Rust impl blocks for efficient RPC.
//!
//! # Schema Generation
//!
//! Creates `.capnp` schemas:
//! - Methods → Interface methods
//! - Parameters → Struct fields
//! - Return types → Result structs
//! - Generates unique IDs for interfaces
//!
//! # Type Mapping
//!
//! - `String` → Text
//! - `i32`, `i64` → Int32, Int64
//! - `u32`, `u64` → UInt32, UInt64
//! - `f32`, `f64` → Float32, Float64
//! - `bool` → Bool
//! - `Vec<T>` → List(T)
//! - `Option<T>` → Nullable field
//!
//! # Generated Methods
//!
//! - `capnp_schema() -> &'static str` - Generated Cap'n Proto schema
//! - `validate_schema() -> Result<(), SchemaValidationError>` - Validate if schema path provided
//! - `assert_schema_matches()` - Panic if validation fails
//!
//! # Example
//!
//! ```ignore
//! use server_less::capnp;
//!
//! struct Calculator;
//!
//! #[capnp(id = "0xabcd1234")]
//! impl Calculator {
//!     fn add(&self, a: i32, b: i32) -> i32 {
//!         a + b
//!     }
//! }
//!
//! let schema = Calculator::capnp_schema();
//! ```

use crate::app::extract_app_meta;
use crate::context::partition_context_params;
use crate::server_attrs::{has_server_hidden, has_server_skip, validate_server_attrs};
use heck::{ToLowerCamelCase, ToUpperCamelCase};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{
    MethodInfo, ParamInfo, extract_methods, get_impl_name, unwrap_option_type, unwrap_result_ok_type,
    unwrap_vec_type,
};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[capnp] attribute
#[derive(Default)]
pub(crate) struct CapnpArgs {
    /// Schema ID (required for Cap'n Proto)
    id: Option<String>,
    /// Path to expected schema for validation
    schema: Option<String>,
}

impl Parse for CapnpArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = CapnpArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "id" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.id = Some(lit.value());
                }
                "schema" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.schema = Some(lit.value());
                }
                other => {
                    const VALID: &[&str] = &["id", "schema"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}. Valid arguments: id, schema"
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

pub(crate) fn expand_capnp(args: CapnpArgs, mut impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    crate::reject_generic_impl(&impl_block)?;
    let _app_meta = extract_app_meta(&mut impl_block.attrs);
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

    // Require a non-zero unique ID
    let schema_id = match args.id {
        Some(ref id) if id == "0x0000000000000000" || id == "0" => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[capnp] requires a non-zero id: #[capnp(id = \"0xABCD1234ABCD1234\")] — generate one with: capnp id",
            ));
        }
        Some(ref id) => id.clone(),
        None => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[capnp] requires an id: #[capnp(id = \"0xABCD1234ABCD1234\")] — generate one with: capnp id",
            ));
        }
    };

    // Generate Cap'n Proto schema
    let interface_methods: Vec<String> = methods
        .iter()
        .enumerate()
        .map(|(i, m)| generate_capnp_method(m, i))
        .collect();

    let structs: Vec<String> = methods.iter().flat_map(generate_capnp_structs).collect();

    let capnp_schema = format!(
        r#"@{schema_id};

interface {interface_name} {{
{methods}
}}

{structs}
"#,
        schema_id = schema_id,
        interface_name = struct_name_str,
        methods = interface_methods.join("\n"),
        structs = structs.join("\n")
    );

    // Generate validation method if schema path is provided
    let validation_method = if let Some(schema_path) = &args.schema {
        quote! {
            /// Validate that the generated schema matches the expected schema.
            ///
            /// # Limitation: field-presence only, not field order
            ///
            /// This check verifies line-by-line presence in both directions.  It does **not**
            /// verify field ordinal positions.  In Cap'n Proto, field ordinals (`@0`, `@1`, ...)
            /// determine the wire encoding: reordering fields (changing their ordinals) breaks
            /// binary compatibility with existing clients even though this validation passes.
            /// Users are responsible for maintaining ordinal stability across schema versions.
            pub fn validate_schema() -> Result<(), ::server_less::SchemaValidationError> {
                let expected = include_str!(#schema_path);
                let generated = Self::capnp_schema();

                fn normalize(s: &str) -> Vec<String> {
                    s.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect()
                }

                let expected_lines = normalize(expected);
                let generated_lines = normalize(generated);

                let mut error = ::server_less::SchemaValidationError::new("Cap'n Proto");

                for line in &expected_lines {
                    if !generated_lines.contains(line) {
                        error.add_missing(line.clone());
                    }
                }

                for line in &generated_lines {
                    if !expected_lines.contains(line) {
                        error.add_extra(line.clone());
                    }
                }

                if error.has_differences() {
                    Err(error)
                } else {
                    Ok(())
                }
            }

            /// Assert that the schema matches.
            pub fn assert_schema_matches() {
                if let Err(err) = Self::validate_schema() {
                    panic!("{}", err);
                }
            }
        }
    } else {
        quote! {}
    };

    let maybe_impl = if crate::is_protocol_impl_emitter(&impl_block, "capnp") {
        quote! { #impl_block }
    } else {
        quote! {}
    };

    Ok(quote! {
        #maybe_impl

        impl #impl_generics #self_ty #where_clause {
            /// Get the Cap'n Proto schema for this service.
            pub fn capnp_schema() -> &'static str {
                #capnp_schema
            }

            /// Write the Cap'n Proto schema to a file.
            pub fn write_capnp(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::capnp_schema())
            }

            #validation_method
        }
    })
}

/// Generate a Cap'n Proto interface method
fn generate_capnp_method(method: &MethodInfo, index: usize) -> String {
    let method_name = method.name_str().to_lower_camel_case();
    let request_name = format!("{}Params", method.name_str().to_upper_camel_case());
    let response_name = format!("{}Result", method.name_str().to_upper_camel_case());

    let doc = method
        .docs
        .as_ref()
        .map(|d| format!("  # {}\n", d))
        .unwrap_or_default();

    format!(
        "{}  {} @{} ({}) -> ({});",
        doc, method_name, index, request_name, response_name
    )
}

/// Generate Cap'n Proto struct definitions for a method
fn generate_capnp_structs(method: &MethodInfo) -> Vec<String> {
    let method_upper = method.name_str().to_upper_camel_case();
    let params_name = format!("{}Params", method_upper);
    let result_name = format!("{}Result", method_upper);

    // Filter out server_less::Context params — they are runtime-injected, not schema fields.
    let (_, schema_params) = partition_context_params(&method.params).unwrap_or((None, method.params.iter().collect()));
    // Generate params struct
    let param_fields: Vec<String> = schema_params
        .iter()
        .enumerate()
        .map(|(i, p)| generate_capnp_field(p, i))
        .collect();

    let params_struct = format!("struct {} {{\n{}\n}}", params_name, param_fields.join("\n"));

    // Generate result struct
    let ret = &method.return_info;
    let result_struct = if ret.is_unit {
        format!("struct {} {{\n}}", result_name)
    } else {
        let capnp_type = rust_type_to_capnp(&ret.ty);
        format!("struct {} {{\n  value @0 :{};\n}}", result_name, capnp_type)
    };

    vec![params_struct, result_struct]
}

/// Generate a Cap'n Proto field definition
fn generate_capnp_field(param: &ParamInfo, index: usize) -> String {
    let name = param.name_str().to_lower_camel_case();
    let capnp_type = rust_type_to_capnp(&Some(param.ty.clone()));
    format!("  {} @{} :{};", name, index, capnp_type)
}

/// Convert Rust type to Cap'n Proto type
fn rust_type_to_capnp(ty: &Option<syn::Type>) -> String {
    let Some(ty) = ty else {
        return "Void".to_string();
    };
    rust_type_to_capnp_ty(ty)
}

/// Convert a `syn::Type` reference to a Cap'n Proto type string.
fn rust_type_to_capnp_ty(ty: &syn::Type) -> String {
    // Unwrap Result<T, E> → T
    if let Some(ok) = unwrap_result_ok_type(ty) {
        return rust_type_to_capnp_ty(ok);
    }
    // Unwrap Option<T> → map inner (Cap'n Proto uses union for optional; emit inner type)
    if let Some(inner) = unwrap_option_type(ty) {
        return rust_type_to_capnp_ty(inner);
    }
    // Vec<u8> → Data (check inner element before the generic Vec<T> path)
    if let Some(inner) = unwrap_vec_type(ty) {
        if let syn::Type::Path(tp) = inner
            && tp.path.segments.last().map(|s| s.ident == "u8").unwrap_or(false)
        {
            return "Data".to_string();
        }
        return format!("List({})", rust_type_to_capnp_ty(inner));
    }
    // [u8] slice → Data
    if let syn::Type::Slice(ts) = ty
        && let syn::Type::Path(tp) = &*ts.elem
        && tp.path.segments.last().map(|s| s.ident == "u8").unwrap_or(false)
    {
        return "Data".to_string();
    }
    // Use exact path-segment matching to avoid false positives on user-defined wrapper types
    // (e.g. `MyI32Wrapper` must not match `i32`, `MyString` must not match `String`).
    let ident = if let syn::Type::Path(tp) = ty {
        tp.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    };
    match ident.as_deref() {
        Some("String") | Some("str") => "Text".to_string(),
        Some("i8") => "Int8".to_string(),
        Some("i16") => "Int16".to_string(),
        Some("i32") => "Int32".to_string(),
        Some("i64") => "Int64".to_string(),
        Some("u8") => "UInt8".to_string(),
        Some("u16") => "UInt16".to_string(),
        Some("u32") => "UInt32".to_string(),
        Some("u64") => "UInt64".to_string(),
        Some("f32") => "Float32".to_string(),
        Some("f64") => "Float64".to_string(),
        Some("bool") => "Bool".to_string(),
        _ => "Data".to_string(), // fallback to bytes
    }
}
