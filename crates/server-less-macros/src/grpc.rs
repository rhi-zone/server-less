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
//! - `proto_schema() -> &'static str` - Generated .proto schema
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
//! let schema = UserService::proto_schema();
//! ```

use heck::{ToSnakeCase, ToUpperCamelCase};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};
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
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: package, schema"),
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

pub(crate) fn expand_grpc(args: GrpcArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let package = args
        .package
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
                let generated = Self::proto_schema();
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

    Ok(quote! {
        #impl_block
        impl #struct_name {
            pub fn proto_schema() -> &'static str {
                #proto_schema
            }
            pub fn write_proto(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::proto_schema())
            }
            #validation_method
        }
    })
}

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

fn generate_proto_messages(method: &MethodInfo) -> Vec<String> {
    let method_name = method.name.to_string().to_upper_camel_case();
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
    let name = param.name.to_string().to_snake_case();
    let proto_type = rust_type_to_proto(&Some(param.ty.clone()));
    let optional = if param.is_optional { "optional " } else { "" };
    format!("  {}{} {} = {};", optional, proto_type, name, field_num)
}

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
