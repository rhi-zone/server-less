//! Cap'n Proto schema generation macro.

use heck::{ToLowerCamelCase, ToUpperCamelCase};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemImpl, Token, parse::Parse};
use trellis_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};

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
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: id, schema"),
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

pub(crate) fn expand_capnp(args: CapnpArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    // Use provided ID or generate a placeholder
    let schema_id = args.id.unwrap_or_else(|| "0x0000000000000000".to_string());

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
            pub fn validate_schema() -> Result<(), String> {
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
                    panic!("Cap'n Proto schema validation failed:\n{}", diff);
                }
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #impl_block

        impl #struct_name {
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
    let method_name = method.name.to_string().to_lower_camel_case();
    let request_name = format!("{}Params", method.name.to_string().to_upper_camel_case());
    let response_name = format!("{}Result", method.name.to_string().to_upper_camel_case());

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
    let method_upper = method.name.to_string().to_upper_camel_case();
    let params_name = format!("{}Params", method_upper);
    let result_name = format!("{}Result", method_upper);

    // Generate params struct
    let param_fields: Vec<String> = method
        .params
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
    let name = param.name.to_string().to_lower_camel_case();
    let capnp_type = rust_type_to_capnp(&Some(param.ty.clone()));
    format!("  {} @{} :{};", name, index, capnp_type)
}

/// Convert Rust type to Cap'n Proto type
fn rust_type_to_capnp(ty: &Option<syn::Type>) -> &'static str {
    let Some(ty) = ty else {
        return "Void";
    };

    let type_str = quote!(#ty).to_string();

    // Check compound types first (Vec, Option) before primitives
    if type_str.contains("Vec < u8 >") || type_str.contains("Vec<u8>") || type_str.contains("[u8]")
    {
        "Data"
    } else if type_str.contains("Vec") {
        "List(Text)" // simplified
    } else if type_str.contains("Option") || type_str.contains("String") || type_str.contains("str")
    {
        "Text" // simplified - Cap'n Proto doesn't have optional, uses union
    } else if type_str.contains("i8") {
        "Int8"
    } else if type_str.contains("i16") {
        "Int16"
    } else if type_str.contains("i32") {
        "Int32"
    } else if type_str.contains("i64") {
        "Int64"
    } else if type_str.contains("u8") {
        "UInt8"
    } else if type_str.contains("u16") {
        "UInt16"
    } else if type_str.contains("u32") {
        "UInt32"
    } else if type_str.contains("u64") {
        "UInt64"
    } else if type_str.contains("f32") {
        "Float32"
    } else if type_str.contains("f64") {
        "Float64"
    } else if type_str.contains("bool") {
        "Bool"
    } else {
        "Data" // fallback to bytes
    }
}
