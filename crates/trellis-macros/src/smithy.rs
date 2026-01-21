//! Smithy IDL schema generation macro.
//!
//! Generates Smithy interface definition language schemas.
//! Smithy is AWS's open-source IDL for defining APIs.

use heck::{ToPascalCase, ToSnakeCase};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse::Parse, ItemImpl, Token};
use trellis_parse::{extract_methods, get_impl_name, MethodInfo, ParamInfo};

/// Arguments for the #[smithy] attribute
#[derive(Default)]
pub(crate) struct SmithyArgs {
    /// Namespace for the service
    namespace: Option<String>,
    /// Service version
    version: Option<String>,
}

impl Parse for SmithyArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = SmithyArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "namespace" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.namespace = Some(lit.value());
                }
                "version" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`. Valid arguments: namespace, version"
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


pub(crate) fn expand_smithy(args: SmithyArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let namespace = args
        .namespace
        .unwrap_or_else(|| format!("com.example.{}", struct_name_str.to_snake_case()));
    let version = args.version.unwrap_or_else(|| "2024-01-01".to_string());

    // Generate operation definitions
    let operations: Vec<String> = methods.iter().map(generate_operation).collect();

    // Generate structure definitions
    let structures: Vec<String> = methods
        .iter()
        .flat_map(generate_structures)
        .collect();

    // Generate the Smithy IDL
    let smithy_schema = format!(
        r#"$version: "2"

namespace {namespace}

/// {service_name} service
service {service_name} {{
    version: "{version}"
    operations: [
{operation_list}
    ]
}}

{operations}

{structures}
"#,
        namespace = namespace,
        service_name = struct_name_str,
        version = version,
        operation_list = methods
            .iter()
            .map(|m| format!("        {}", m.name.to_string().to_pascal_case()))
            .collect::<Vec<_>>()
            .join("\n"),
        operations = operations.join("\n\n"),
        structures = structures.join("\n\n")
    );

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get the Smithy IDL schema for this service.
            pub fn smithy_schema() -> &'static str {
                #smithy_schema
            }

            /// Write the Smithy schema to a file.
            pub fn write_smithy(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::smithy_schema())
            }
        }
    })
}

/// Generate a Smithy operation definition
fn generate_operation(method: &MethodInfo) -> String {
    let op_name = method.name.to_string().to_pascal_case();
    let input_name = format!("{}Input", op_name);
    let output_name = format!("{}Output", op_name);

    let doc = method
        .docs
        .as_ref()
        .map(|d| format!("/// {}\n", d))
        .unwrap_or_default();

    format!(
        r#"{doc}operation {op_name} {{
    input: {input_name}
    output: {output_name}
}}"#,
        doc = doc,
        op_name = op_name,
        input_name = input_name,
        output_name = output_name
    )
}

/// Generate Smithy structure definitions for a method
fn generate_structures(method: &MethodInfo) -> Vec<String> {
    let op_name = method.name.to_string().to_pascal_case();
    let input_name = format!("{}Input", op_name);
    let output_name = format!("{}Output", op_name);

    // Generate input structure
    let input_fields: Vec<String> = method
        .params
        .iter()
        .map(generate_field)
        .collect();

    let input_struct = if input_fields.is_empty() {
        format!("structure {} {{}}", input_name)
    } else {
        format!(
            "structure {} {{\n{}\n}}",
            input_name,
            input_fields.join("\n")
        )
    };

    // Generate output structure
    let ret = &method.return_info;
    let output_struct = if ret.is_unit {
        format!("structure {} {{}}", output_name)
    } else {
        let smithy_type = rust_type_to_smithy(&ret.ty);
        format!(
            "structure {} {{\n    @required\n    result: {}\n}}",
            output_name, smithy_type
        )
    };

    vec![input_struct, output_struct]
}

/// Generate a Smithy field definition
fn generate_field(param: &ParamInfo) -> String {
    let name = param.name.to_string().to_snake_case();
    let smithy_type = rust_type_to_smithy(&Some(param.ty.clone()));
    let required = if param.is_optional { "" } else { "@required\n    " };
    format!("    {required}{name}: {smithy_type}")
}

/// Convert Rust type to Smithy type
fn rust_type_to_smithy(ty: &Option<syn::Type>) -> &'static str {
    let Some(ty) = ty else {
        return "Unit";
    };

    let type_str = quote!(#ty).to_string();

    if type_str.contains("String") || type_str.contains("str") {
        "String"
    } else if type_str.contains("i8") {
        "Byte"
    } else if type_str.contains("i16") {
        "Short"
    } else if type_str.contains("i32") {
        "Integer"
    } else if type_str.contains("i64") {
        "Long"
    } else if type_str.contains("u8") {
        "Byte"
    } else if type_str.contains("u16") {
        "Short"
    } else if type_str.contains("u32") {
        "Integer"
    } else if type_str.contains("u64") {
        "Long"
    } else if type_str.contains("f32") {
        "Float"
    } else if type_str.contains("f64") {
        "Double"
    } else if type_str.contains("bool") {
        "Boolean"
    } else if type_str.contains("Vec") {
        "List"
    } else {
        "Document"
    }
}
