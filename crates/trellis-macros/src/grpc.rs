//! gRPC/Protobuf schema generation.
//!
//! Generates .proto schema definitions from impl blocks.
//! Users can then use tonic-build with the generated schema.

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, ItemImpl, Token};

use crate::parse::{extract_methods, get_impl_name, MethodInfo, ParamInfo};

/// Arguments for the #[grpc] attribute
#[derive(Default)]
pub struct GrpcArgs {
    /// Package name for the proto file
    pub package: Option<String>,
}

impl Parse for GrpcArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = GrpcArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "package" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.package = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: package"),
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

/// Expand the #[grpc] attribute macro
pub fn expand_grpc(args: GrpcArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let package = args
        .package
        .unwrap_or_else(|| struct_name_str.to_snake_case());
    let service_name = struct_name_str.clone();

    // Generate proto schema string
    let proto_methods: Vec<String> = methods
        .iter()
        .map(|m| generate_proto_method(m))
        .collect();

    let proto_messages: Vec<String> = methods
        .iter()
        .flat_map(|m| generate_proto_messages(m))
        .collect();

    let proto_schema = format!(
        r#"syntax = "proto3";

package {package};

service {service_name} {{
{methods}
}}

{messages}
"#,
        package = package,
        service_name = service_name,
        methods = proto_methods.join("\n"),
        messages = proto_messages.join("\n")
    );

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get the Protocol Buffers schema for this service.
            ///
            /// This can be written to a .proto file and used with tonic-build
            /// to generate the gRPC client/server code.
            pub fn proto_schema() -> &'static str {
                #proto_schema
            }

            /// Write the proto schema to a file.
            ///
            /// # Example
            /// ```ignore
            /// MyService::write_proto("proto/my_service.proto")?;
            /// ```
            pub fn write_proto(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::proto_schema())
            }
        }
    })
}

/// Generate a proto rpc method definition
fn generate_proto_method(method: &MethodInfo) -> String {
    let method_name = method.name.to_string().to_upper_camel_case();
    let request_name = format!("{}Request", method_name);
    let response_name = format!("{}Response", method_name);

    let doc = method
        .docs
        .as_ref()
        .map(|d| format!("  // {}\n", d))
        .unwrap_or_default();

    format!(
        "{}  rpc {}({}) returns ({});",
        doc, method_name, request_name, response_name
    )
}

/// Generate proto message definitions for a method
fn generate_proto_messages(method: &MethodInfo) -> Vec<String> {
    let method_name = method.name.to_string().to_upper_camel_case();
    let request_name = format!("{}Request", method_name);
    let response_name = format!("{}Response", method_name);

    // Generate request message
    let request_fields: Vec<String> = method
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| generate_proto_field(p, i + 1))
        .collect();

    let request_msg = format!(
        "message {} {{\n{}\n}}",
        request_name,
        request_fields.join("\n")
    );

    // Generate response message
    let ret = &method.return_info;
    let response_msg = if ret.is_unit {
        format!("message {} {{\n}}", response_name)
    } else {
        let proto_type = rust_type_to_proto(&ret.ty);
        format!(
            "message {} {{\n  {} result = 1;\n}}",
            response_name, proto_type
        )
    };

    vec![request_msg, response_msg]
}

/// Generate a proto field definition
fn generate_proto_field(param: &ParamInfo, field_num: usize) -> String {
    let name = param.name.to_string().to_snake_case();
    let proto_type = rust_type_to_proto(&Some(param.ty.clone()));
    let optional = if param.is_optional { "optional " } else { "" };
    format!("  {}{} {} = {};", optional, proto_type, name, field_num)
}

/// Convert Rust type to protobuf type
fn rust_type_to_proto(ty: &Option<syn::Type>) -> &'static str {
    let Some(ty) = ty else {
        return "google.protobuf.Empty";
    };

    // Get the type as string for simple matching
    let type_str = quote!(#ty).to_string();

    // Handle common types
    if type_str.contains("String") || type_str.contains("str") {
        "string"
    } else if type_str.contains("i32") {
        "int32"
    } else if type_str.contains("i64") {
        "int64"
    } else if type_str.contains("u32") {
        "uint32"
    } else if type_str.contains("u64") {
        "uint64"
    } else if type_str.contains("f32") {
        "float"
    } else if type_str.contains("f64") {
        "double"
    } else if type_str.contains("bool") {
        "bool"
    } else if type_str.contains("Vec") {
        // For Vec<T>, we'd need to recursively determine T
        // Simplified: assume string array
        "repeated string"
    } else if type_str.contains("Option") {
        // Option<T> handled at field level with 'optional'
        "string" // simplified
    } else {
        // Unknown type - treat as bytes
        "bytes"
    }
}
