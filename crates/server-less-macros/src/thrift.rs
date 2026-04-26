//! Apache Thrift schema generation macro.
//!
//! Generates Apache Thrift IDL schemas from Rust impl blocks.
//!
//! # Schema Generation
//!
//! Creates `.thrift` interface definitions:
//! - Methods → Service methods
//! - Parameters → Struct fields
//! - Return types → Response types
//! - Generates field IDs automatically
//!
//! # Type Mapping
//!
//! - `String` → string
//! - `i32`, `i64` → i32, i64
//! - `bool` → bool
//! - `f64` → double
//! - `Vec<T>` → list<T>
//! - `Option<T>` → optional T
//!
//! # Generated Methods
//!
//! - `thrift_schema() -> &'static str` - Generated Thrift schema
//! - `validate_schema() -> Result<(), SchemaValidationError>` - Validate if schema path provided
//! - `assert_schema_matches()` - Panic if validation fails
//!
//! # Example
//!
//! ```ignore
//! use server_less::thrift;
//!
//! struct UserService;
//!
//! #[thrift(namespace = "com.example")]
//! impl UserService {
//!     fn get_user(&self, user_id: i32) -> String {
//!         format!("User {}", user_id)
//!     }
//! }
//!
//! let schema = UserService::thrift_schema();
//! ```

use crate::app::extract_app_meta;
use crate::context::partition_context_params;
use crate::server_attrs::{has_server_hidden, has_server_skip, validate_server_attrs};
use heck::{ToSnakeCase, ToUpperCamelCase};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{
    MethodInfo, ParamInfo, ReturnInfo, extract_methods, get_impl_name, unwrap_option_type,
    unwrap_result_ok_type, unwrap_vec_type,
};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[thrift] attribute
#[derive(Default)]
pub(crate) struct ThriftArgs {
    /// Namespace for the thrift file
    namespace: Option<String>,
    /// Path to expected schema for validation
    schema: Option<String>,
}

impl Parse for ThriftArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ThriftArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "namespace" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.namespace = Some(lit.value());
                }
                "schema" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.schema = Some(lit.value());
                }
                other => {
                    const VALID: &[&str] = &["namespace", "schema"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}. Valid arguments: namespace, schema"
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

pub(crate) fn expand_thrift(args: ThriftArgs, mut impl_block: ItemImpl) -> syn::Result<TokenStream2> {
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

    let namespace = args
        .namespace
        .or_else(|| app_meta.name.map(|n| n.to_snake_case()))
        .unwrap_or_else(|| struct_name_str.to_snake_case());

    // Generate Thrift schema
    let service_methods: Vec<String> = methods
        .iter()
        .enumerate()
        .map(|(i, m)| generate_thrift_method(m, i + 1))
        .collect();

    let structs: Vec<String> = methods.iter().flat_map(generate_thrift_structs).collect();

    let thrift_schema = format!(
        r#"namespace rs {namespace}

service {service_name} {{
{methods}
}}

{structs}
"#,
        namespace = namespace,
        service_name = struct_name_str,
        methods = service_methods.join("\n"),
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
            /// verify field ID order.  In Thrift, field IDs determine the wire encoding:
            /// reordering fields (changing their assigned IDs) breaks binary compatibility
            /// with existing clients even though this validation passes.  Users are responsible
            /// for maintaining field-ID stability across schema versions.
            pub fn validate_schema() -> Result<(), ::server_less::SchemaValidationError> {
                let expected = include_str!(#schema_path);
                let generated = Self::thrift_schema();

                fn normalize(s: &str) -> Vec<String> {
                    s.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("//"))
                        .collect()
                }

                let expected_lines = normalize(expected);
                let generated_lines = normalize(generated);

                let mut error = ::server_less::SchemaValidationError::new("Thrift");

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

    let maybe_impl = if crate::is_protocol_impl_emitter(&impl_block, "thrift") {
        quote! { #impl_block }
    } else {
        quote! {}
    };

    Ok(quote! {
        #maybe_impl

        impl #impl_generics #self_ty #where_clause {
            /// Get the Thrift schema for this service.
            pub fn thrift_schema() -> &'static str {
                #thrift_schema
            }

            /// Write the Thrift schema to a file.
            pub fn write_thrift(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::thrift_schema())
            }

            #validation_method
        }
    })
}

/// Generate a Thrift service method
fn generate_thrift_method(method: &MethodInfo, index: usize) -> String {
    let method_name = method.name_str().to_snake_case();
    let args_name = format!("{}Args", method.name_str().to_upper_camel_case());
    let result_type = get_thrift_return_type(&method.return_info);

    let doc = method
        .docs
        .as_ref()
        .map(|d| format!("  // {}\n", d))
        .unwrap_or_default();

    format!(
        "{}  {} {}({} args) = {};",
        doc, result_type, method_name, args_name, index
    )
}

/// Get Thrift return type
fn get_thrift_return_type(ret: &ReturnInfo) -> String {
    if ret.is_unit {
        "void".to_string()
    } else {
        rust_type_to_thrift(&ret.ty)
    }
}

/// Generate Thrift struct definitions for a method
fn generate_thrift_structs(method: &MethodInfo) -> Vec<String> {
    let method_upper = method.name_str().to_upper_camel_case();
    let args_name = format!("{}Args", method_upper);

    // Filter out server_less::Context params — they are runtime-injected, not schema fields.
    let (_, schema_params) = partition_context_params(&method.params).unwrap_or((None, method.params.iter().collect()));
    // Generate args struct
    let arg_fields: Vec<String> = schema_params
        .iter()
        .enumerate()
        .map(|(i, p)| generate_thrift_field(p, i + 1))
        .collect();

    let args_struct = format!("struct {} {{\n{}\n}}", args_name, arg_fields.join("\n"));

    vec![args_struct]
}

/// Generate a Thrift field definition
fn generate_thrift_field(param: &ParamInfo, index: usize) -> String {
    let name = param.name_str().to_snake_case();
    // Unwrap Option<T> — the `optional` keyword is emitted separately below
    let ty = if let Some(inner) = unwrap_option_type(&param.ty) {
        inner.clone()
    } else {
        param.ty.clone()
    };
    let thrift_type = rust_type_to_thrift(&Some(ty));
    let optional = if param.is_optional { "optional " } else { "" };
    format!("  {}: {}{} {};", index, optional, thrift_type, name)
}

/// Convert Rust type to Thrift type
fn rust_type_to_thrift(ty: &Option<syn::Type>) -> String {
    let Some(ty) = ty else {
        return "void".to_string();
    };
    rust_type_to_thrift_ty(ty)
}

/// Convert a `syn::Type` reference to a Thrift type string.
fn rust_type_to_thrift_ty(ty: &syn::Type) -> String {
    // Unwrap Result<T, E> → T
    if let Some(ok) = unwrap_result_ok_type(ty) {
        return rust_type_to_thrift_ty(ok);
    }
    // Unwrap Option<T> → map inner (optional keyword handled by caller)
    if let Some(inner) = unwrap_option_type(ty) {
        return rust_type_to_thrift_ty(inner);
    }
    // Vec<u8> → binary (check inner element before the generic Vec<T> path).
    // Note: quote!(Vec<u8>).to_string() emits "Vec < u8 >" with spaces; use AST inspection instead.
    if let Some(inner) = unwrap_vec_type(ty) {
        if let syn::Type::Path(tp) = inner
            && tp.path.segments.last().map(|s| s.ident == "u8").unwrap_or(false)
        {
            return "binary".to_string();
        }
        return format!("list<{}>", rust_type_to_thrift_ty(inner));
    }
    // [u8] slice → binary
    if let syn::Type::Slice(ts) = ty
        && let syn::Type::Path(tp) = &*ts.elem
        && tp.path.segments.last().map(|s| s.ident == "u8").unwrap_or(false)
    {
        return "binary".to_string();
    }
    // Use exact path-segment matching to avoid false positives on user-defined wrapper types
    // (e.g. `HashMapWrapper` must not match `HashMap`, `MyI32` must not match `i32`).
    let ident = if let syn::Type::Path(tp) = ty {
        tp.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    };
    match ident.as_deref() {
        Some("HashMap") | Some("BTreeMap") => "map<string, string>".to_string(), // simplified
        Some("HashSet") | Some("BTreeSet") => "set<string>".to_string(),          // simplified
        Some("String") | Some("str") => "string".to_string(),
        Some("bool") => "bool".to_string(),
        Some("i8") => "byte".to_string(),
        Some("i16") => "i16".to_string(),
        Some("i32") => "i32".to_string(),
        Some("i64") => "i64".to_string(),
        Some("f64") => "double".to_string(),
        _ => "binary".to_string(), // fallback
    }
}
