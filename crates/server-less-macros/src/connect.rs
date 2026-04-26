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

use crate::app::extract_app_meta;
use crate::context::partition_context_params;
use crate::server_attrs::{has_server_hidden, has_server_skip, validate_server_attrs};
use heck::{ToSnakeCase, ToUpperCamelCase};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{
    MethodInfo, ParamInfo, extract_methods, get_impl_name, unwrap_option_type, unwrap_result_ok_type,
    unwrap_vec_type,
};
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
                    const VALID: &[&str] = &["package"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}. Valid arguments: package"
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

pub(crate) fn expand_connect(args: ConnectArgs, mut impl_block: ItemImpl) -> syn::Result<TokenStream2> {
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

    let package = args
        .package
        .or(app_meta.name.map(|n| n.to_snake_case()))
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
            let method_name = m.name_str().to_upper_camel_case();
            format!("/{}.{}/{}", package, service_name, method_name)
        })
        .collect();

    let maybe_impl = if crate::is_protocol_impl_emitter(&impl_block, "connect") {
        quote! { #impl_block }
    } else {
        quote! {}
    };

    Ok(quote! {
        #maybe_impl

        impl #impl_generics #self_ty #where_clause {
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
    let method_name = method.name_str().to_upper_camel_case();
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
    let method_name = method.name_str().to_upper_camel_case();
    let request_name = format!("{}Request", method_name);
    let response_name = format!("{}Response", method_name);

    // Filter out server_less::Context params — they are runtime-injected, not schema fields.
    let (_, schema_params) = partition_context_params(&method.params).unwrap_or((None, method.params.iter().collect()));
    // Generate request message
    let request_fields: Vec<String> = schema_params
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
    let name = param.name_str().to_snake_case();
    // Unwrap Option<T> — the `optional` keyword is emitted separately below
    let ty = if let Some(inner) = unwrap_option_type(&param.ty) {
        inner.clone()
    } else {
        param.ty.clone()
    };
    let proto_type = rust_type_to_proto(&Some(ty));
    let optional = if param.is_optional { "optional " } else { "" };
    format!("  {}{} {} = {};", optional, proto_type, name, field_num)
}

/// Convert Rust type to protobuf type
fn rust_type_to_proto(ty: &Option<syn::Type>) -> String {
    let Some(ty) = ty else {
        return "google.protobuf.Empty".to_string();
    };
    // Unwrap Result<T, E> → T
    let ty = if let Some(ok) = unwrap_result_ok_type(ty) {
        std::borrow::Cow::Borrowed(ok)
    } else {
        std::borrow::Cow::Borrowed(ty)
    };
    // Unwrap Vec<T> → "repeated <mapped_T>"
    if let Some(inner) = unwrap_vec_type(&ty) {
        return format!("repeated {}", rust_type_to_proto_scalar(inner));
    }
    // Unwrap Option<T> → map inner (optional keyword handled by caller)
    let ty = if let Some(inner) = unwrap_option_type(&ty) {
        inner
    } else {
        &*ty
    };
    rust_type_to_proto_scalar(ty).to_string()
}

fn rust_type_to_proto_scalar(ty: &syn::Type) -> &'static str {
    // Use exact path-segment matching to avoid false positives on user-defined wrapper types
    // (e.g. `MyI32Wrapper` must not match `i32`, `MyString` must not match `String`).
    let ident = if let syn::Type::Path(tp) = ty {
        tp.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    };
    match ident.as_deref() {
        Some("String") | Some("str") => "string",
        Some("i32") => "int32",
        Some("i64") => "int64",
        Some("u32") => "uint32",
        Some("u64") => "uint64",
        Some("f32") => "float",
        Some("f64") => "double",
        Some("bool") => "bool",
        _ => "bytes",
    }
}
