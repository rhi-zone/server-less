//! Connect protocol schema generation macro.
//!
//! Generates Connect/buf-compatible service definitions.
//! Connect is a modern alternative to gRPC that works over HTTP/1.1, HTTP/2, and HTTP/3.
//!
//! # Protocol
//!
//! Connect is a simple protocol built on Protocol Buffers:
//! - Works with standard HTTP and JSON
//! - Compatible with gRPC clients and servers
//! - Simpler than gRPC for web applications
//!
//! # Schema Generation
//!
//! Creates Protocol Buffers schemas for Connect:
//! - Methods → RPC service definitions
//! - Uses proto3 syntax
//! - Compatible with buf.build tooling
//!
//! # Generated Methods
//!
//! - `connect_schema() -> &'static str` - Generated .proto schema for Connect
//!
//! # Example
//!
//! ```ignore
//! use server_less::connect;
//!
//! struct ChatService;
//!
//! #[connect(package = "chat.v1")]
//! impl ChatService {
//!     fn send_message(&self, message: String) -> String {
//!         format!("Received: {}", message)
//!     }
//! }
//!
//! let schema = ChatService::connect_schema();
//! ```

use heck::{ToSnakeCase, ToUpperCamelCase};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[connect] attribute
#[derive(Default)]
pub(crate) struct ConnectArgs {
    /// Package name
    package: Option<String>,
}

impl Parse for ConnectArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = ConnectArgs::default();

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

pub(crate) fn expand_connect(args: ConnectArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let package = args
        .package
        .unwrap_or_else(|| struct_name_str.to_snake_case());
    let service_name = struct_name_str.clone();

    // Generate proto schema (Connect uses protobuf)
    let proto_methods: Vec<String> = methods.iter().map(generate_proto_method).collect();

    let proto_messages: Vec<String> = methods.iter().flat_map(generate_proto_messages).collect();

    let proto_schema = format!(
        r#"syntax = "proto3";

package {package};

// Connect service definition
// Compatible with connect-go, connect-es, connect-swift, etc.
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

    // Generate Connect-specific paths
    let connect_paths: Vec<String> = methods
        .iter()
        .map(|m| {
            let method_name = m.name.to_string().to_upper_camel_case();
            format!("/{}.{}/{}", package, service_name, method_name)
        })
        .collect();

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Get the Protocol Buffers schema for Connect.
            pub fn connect_schema() -> &'static str {
                #proto_schema
            }

            /// Get Connect endpoint paths.
            pub fn connect_paths() -> Vec<&'static str> {
                vec![#(#connect_paths),*]
            }

            /// Write the Connect schema to a file.
            pub fn write_connect(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::connect_schema())
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
        format!("message {} {{}}", response_name)
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

    let type_str = quote!(#ty).to_string();

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
        "repeated string"
    } else {
        "bytes"
    }
}
