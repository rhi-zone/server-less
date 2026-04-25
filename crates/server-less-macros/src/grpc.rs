//! gRPC/Protobuf schema generation macro.
//!
//! Generates Protocol Buffers (.proto) schemas from Rust impl blocks for gRPC services.
//!
//! # Schema Generation
//!
//! Creates `.proto` files from Rust code:
//! - Methods → RPC service definitions
//! - Parameters → Message fields
//! - Return types → Response messages
//! - Doc comments → Proto comments
//!
//! # Type Mapping
//!
//! - `String` → string
//! - `i32`, `i64` → int32, int64
//! - `u32`, `u64` → uint32, uint64
//! - `f32`, `f64` → float, double
//! - `bool` → bool
//! - `Vec<T>` → repeated T
//! - `Option<T>` → optional T
//!
//! # Generated Methods
//!
//! - `grpc_schema() -> &'static str` - Generated .proto schema
//! - `write_grpc(path)` - Write .proto schema to a file
//! - `validate_schema() -> Result<(), SchemaValidationError>` - Validate if schema path provided
//! - `assert_schema_matches()` - Panic if validation fails (for tests)
//!
//! # Example
//!
//! ```ignore
//! use server_less::grpc;
//!
//! struct UserService;
//!
//! #[grpc(package = "users.v1", service = "UserService")]
//! impl UserService {
//!     /// Get user by ID
//!     fn get_user(&self, user_id: i32) -> String {
//!         format!("User {}", user_id)
//!     }
//! }
//!
//! let schema = UserService::grpc_schema();
//! ```

use crate::app::extract_app_meta;
use crate::server_attrs::{has_server_hidden, has_server_skip};
use heck::{ToSnakeCase, ToUpperCamelCase};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{
    MethodInfo, ParamInfo, extract_methods, get_impl_name, unwrap_option_type, unwrap_result_ok_type,
    unwrap_vec_type,
};
use syn::{ItemImpl, Token, parse::Parse};

#[derive(Default)]
pub(crate) struct GrpcArgs {
    package: Option<String>,
    schema: Option<String>,
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
                "schema" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.schema = Some(lit.value());
                }
                other => {
                    const VALID: &[&str] = &["package", "schema"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}. Valid arguments: package, schema"
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

pub(crate) fn expand_grpc(args: GrpcArgs, mut impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    crate::reject_generic_impl(&impl_block)?;
    let app_meta = extract_app_meta(&mut impl_block.attrs);
    let struct_name = get_impl_name(&impl_block)?;
    let (impl_generics, _ty_generics, where_clause) = impl_block.generics.split_for_impl();
    let self_ty = &impl_block.self_ty;
    let struct_name_str = struct_name.to_string();
    let methods: Vec<_> = extract_methods(&impl_block)?
        .into_iter()
        .filter(|m| !has_server_skip(m) && !has_server_hidden(m))
        .collect();

    let package = args
        .package
        .or(app_meta.name.map(|n| n.to_snake_case()))
        .unwrap_or_else(|| struct_name_str.to_snake_case());
    let service_name = struct_name_str.clone();

    let proto_methods: Vec<String> = methods.iter().map(generate_proto_method).collect();
    let proto_messages: Vec<String> = methods.iter().flat_map(generate_proto_messages).collect();

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

    let validation_method = if let Some(schema_path) = &args.schema {
        quote! {
            pub fn validate_schema() -> Result<(), ::server_less::SchemaValidationError> {
                let expected = include_str!(#schema_path);
                let generated = Self::grpc_schema();
                fn normalize(s: &str) -> Vec<String> {
                    s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
                }
                let expected_lines = normalize(expected);
                let generated_lines = normalize(generated);

                let mut error = ::server_less::SchemaValidationError::new("Proto");

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
            pub fn assert_schema_matches() {
                if let Err(err) = Self::validate_schema() {
                    panic!("{}", err);
                }
            }
        }
    } else {
        quote! {}
    };

    let maybe_impl = if crate::is_protocol_impl_emitter(&impl_block, "grpc") {
        quote! { #impl_block }
    } else {
        quote! {}
    };

    Ok(quote! {
        #maybe_impl
        impl #impl_generics #self_ty #where_clause {
            pub fn grpc_schema() -> &'static str {
                #proto_schema
            }
            pub fn write_grpc(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::grpc_schema())
            }
            #validation_method
        }
    })
}

fn generate_proto_method(method: &MethodInfo) -> String {
    let method_name = method.name_str().to_upper_camel_case();
    let request_name = format!("{}Request", method_name);
    let response_name = format!("{}Response", method_name);
    let doc = method
        .docs
        .as_ref()
        .map(|d| format!("  // {}\n", d))
        .unwrap_or_default();

    // Check if this is a streaming response (returns impl Stream<Item = T>)
    let ret = &method.return_info;
    if ret.is_stream {
        // Server streaming RPC
        format!(
            "{}  rpc {}({}) returns (stream {});",
            doc, method_name, request_name, response_name
        )
    } else {
        // Unary RPC
        format!(
            "{}  rpc {}({}) returns ({});",
            doc, method_name, request_name, response_name
        )
    }
}

fn generate_proto_messages(method: &MethodInfo) -> Vec<String> {
    let method_name = method.name_str().to_upper_camel_case();
    let request_name = format!("{}Request", method_name);
    let response_name = format!("{}Response", method_name);
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
    let ret = &method.return_info;
    let response_msg = if ret.is_unit {
        format!("message {} {{\n}}", response_name)
    } else if ret.is_stream {
        // For streaming responses, use the stream item type
        let proto_type = rust_type_to_proto(&ret.stream_item);
        format!(
            "message {} {{\n  {} result = 1;\n}}",
            response_name, proto_type
        )
    } else {
        let proto_type = rust_type_to_proto(&ret.ty);
        format!(
            "message {} {{\n  {} result = 1;\n}}",
            response_name, proto_type
        )
    };
    vec![request_msg, response_msg]
}

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
    } else {
        // NOTE: unknown type mapping — if this type should map to a specific proto type,
        // add it to rust_type_to_proto_scalar(). Defaulting to bytes.
        "bytes"
    }
}
