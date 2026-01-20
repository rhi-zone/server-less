//! Apache Thrift schema generation.
//!
//! Generates .thrift schema definitions from impl blocks.

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, ItemImpl, Token};

use crate::parse::{extract_methods, get_impl_name, MethodInfo, ParamInfo};

/// Arguments for the #[thrift] attribute
#[derive(Default)]
pub struct ThriftArgs {
    /// Namespace for the thrift file
    pub namespace: Option<String>,
    /// Path to expected schema for validation
    pub schema: Option<String>,
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
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: namespace, schema"),
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

/// Expand the #[thrift] attribute macro
pub fn expand_thrift(args: ThriftArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let namespace = args
        .namespace
        .unwrap_or_else(|| struct_name_str.to_snake_case());

    // Generate Thrift schema
    let service_methods: Vec<String> = methods
        .iter()
        .enumerate()
        .map(|(i, m)| generate_thrift_method(m, i + 1))
        .collect();

    let structs: Vec<String> = methods
        .iter()
        .flat_map(|m| generate_thrift_structs(m))
        .collect();

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
            pub fn validate_schema() -> Result<(), String> {
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

                if expected_lines == generated_lines {
                    Ok(())
                } else {
                    let mut diff = String::from("Schema mismatch:\n\n");
                    diff.push_str("Expected not in generated:\n");
                    for line in &expected_lines {
                        if !generated_lines.contains(line) {
                            diff.push_str(&format!("  - {}\n", line));
                        }
                    }
                    diff.push_str("\nGenerated not in expected:\n");
                    for line in &generated_lines {
                        if !expected_lines.contains(line) {
                            diff.push_str(&format!("  + {}\n", line));
                        }
                    }
                    Err(diff)
                }
            }

            /// Assert that the schema matches.
            pub fn assert_schema_matches() {
                if let Err(diff) = Self::validate_schema() {
                    panic!("Thrift schema validation failed:\n{}", diff);
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #impl_block

        impl #struct_name {
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
    let method_name = method.name.to_string().to_snake_case();
    let args_name = format!("{}Args", method.name.to_string().to_upper_camel_case());
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
fn get_thrift_return_type(ret: &crate::parse::ReturnInfo) -> &'static str {
    if ret.is_unit {
        "void"
    } else {
        rust_type_to_thrift(&ret.ty)
    }
}

/// Generate Thrift struct definitions for a method
fn generate_thrift_structs(method: &MethodInfo) -> Vec<String> {
    let method_upper = method.name.to_string().to_upper_camel_case();
    let args_name = format!("{}Args", method_upper);

    // Generate args struct
    let arg_fields: Vec<String> = method
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| generate_thrift_field(p, i + 1))
        .collect();

    let args_struct = format!(
        "struct {} {{\n{}\n}}",
        args_name,
        arg_fields.join("\n")
    );

    vec![args_struct]
}

/// Generate a Thrift field definition
fn generate_thrift_field(param: &ParamInfo, index: usize) -> String {
    let name = param.name.to_string().to_snake_case();
    let thrift_type = rust_type_to_thrift(&Some(param.ty.clone()));
    let optional = if param.is_optional { "optional " } else { "" };
    format!("  {}: {}{} {};", index, optional, thrift_type, name)
}

/// Convert Rust type to Thrift type
fn rust_type_to_thrift(ty: &Option<syn::Type>) -> &'static str {
    let Some(ty) = ty else {
        return "void";
    };

    let type_str = quote!(#ty).to_string();

    // Check compound types first
    if type_str.contains("Vec < u8 >") || type_str.contains("Vec<u8>") || type_str.contains("[u8]")
    {
        "binary"
    } else if type_str.contains("Vec") {
        "list<string>" // simplified
    } else if type_str.contains("HashMap") || type_str.contains("BTreeMap") {
        "map<string, string>" // simplified
    } else if type_str.contains("HashSet") || type_str.contains("BTreeSet") {
        "set<string>" // simplified
    } else if type_str.contains("Option") {
        "string" // simplified
    } else if type_str.contains("String") || type_str.contains("str") {
        "string"
    } else if type_str.contains("bool") {
        "bool"
    } else if type_str.contains("i8") {
        "byte"
    } else if type_str.contains("i16") {
        "i16"
    } else if type_str.contains("i32") {
        "i32"
    } else if type_str.contains("i64") {
        "i64"
    } else if type_str.contains("f64") {
        "double"
    } else {
        "binary" // fallback
    }
}
