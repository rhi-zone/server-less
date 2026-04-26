//! AsyncAPI specification generation macro.
//!
//! Generates AsyncAPI 2.6 specifications for event-driven services.
//! AsyncAPI is to WebSockets/messaging what OpenAPI is to REST.
//!
//! # AsyncAPI
//!
//! Specification for event-driven and message-based APIs:
//! - Describes channels (topics/queues)
//! - Documents message schemas
//! - Supports WebSocket, Kafka, AMQP, MQTT, etc.
//!
//! # Generated Specification
//!
//! Creates AsyncAPI document with:
//! - Channel definitions (one per method)
//! - Message schemas for parameters and results
//! - Subscribe/publish operations
//! - Server information
//!
//! # Generated Methods
//!
//! - `asyncapi_spec() -> serde_json::Value` - Complete AsyncAPI specification
//!
//! # Example
//!
//! ```ignore
//! use server_less::asyncapi;
//!
//! struct ChatService;
//!
//! #[asyncapi(title = "Chat API")]
//! impl ChatService {
//!     /// Send a chat message
//!     fn send_message(&self, text: String) -> String {
//!         format!("Sent: {}", text)
//!     }
//! }
//!
//! let spec = ChatService::asyncapi_spec();
//! ```

use crate::app::extract_app_meta;
use crate::server_attrs::{has_server_hidden, has_server_skip, validate_server_attrs};
use heck::ToLowerCamelCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{
    MethodInfo, ParamInfo, extract_methods, get_impl_name, unwrap_option_type, unwrap_result_ok_type,
    unwrap_vec_type,
};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[asyncapi] attribute
#[derive(Default)]
pub(crate) struct AsyncApiArgs {
    /// Service title
    title: Option<String>,
    /// Service version
    version: Option<String>,
    /// Server URL
    server: Option<String>,
}

impl Parse for AsyncApiArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = AsyncApiArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "name" | "title" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.title = Some(lit.value());
                }
                "version" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                "server" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.server = Some(lit.value());
                }
                other => {
                    const VALID: &[&str] = &["title", "version", "server"];
                    let suggestion = crate::did_you_mean(other, VALID)
                        .map(|s| format!(" — did you mean `{s}`?"))
                        .unwrap_or_default();
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`{suggestion}. Valid arguments: name, version, server"
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

pub(crate) fn expand_asyncapi(
    args: AsyncApiArgs,
    mut impl_block: ItemImpl,
) -> syn::Result<TokenStream2> {
    // L3: reject generic impl blocks (consistent with all other protocol macros).
    crate::reject_generic_impl(&impl_block)?;
    let app_meta = extract_app_meta(&mut impl_block.attrs);
    let struct_name = get_impl_name(&impl_block)?;
    let (impl_generics, _ty_generics, where_clause) = impl_block.generics.split_for_impl();
    let self_ty = &impl_block.self_ty;
    let struct_name_str = struct_name.to_string();
    let all_methods = extract_methods(&impl_block)?;
    // M2: validate #[server(...)] attrs on every method before skip/hidden filtering.
    for m in &all_methods {
        validate_server_attrs(m)?;
    }
    let methods: Vec<_> = all_methods
        .into_iter()
        .filter(|m| !has_server_skip(m) && !has_server_hidden(m))
        .collect();

    let title = args
        .title
        .or(app_meta.name)
        .unwrap_or_else(|| struct_name_str.clone());
    let version = args
        .version
        .or_else(|| app_meta.version.into_explicit())
        .unwrap_or_else(|| "1.0.0".to_string());
    let server = args
        .server
        .unwrap_or_else(|| "ws://localhost:8080".to_string());

    // Generate channel specs for each method
    let channel_specs: Vec<String> = methods.iter().map(generate_channel_spec).collect();
    let channels_json = channel_specs.join(",\n");

    // Generate message schemas
    let message_specs: Vec<String> = methods.iter().map(generate_message_spec).collect();
    let messages_json = message_specs.join(",\n");

    let maybe_impl = if crate::is_protocol_impl_emitter(&impl_block, "asyncapi") {
        quote! { #impl_block }
    } else {
        quote! {}
    };

    Ok(quote! {
        #maybe_impl

        impl #impl_generics #self_ty #where_clause {
            /// Get the AsyncAPI specification for this service.
            pub fn asyncapi_spec() -> ::server_less::serde_json::Value {
                let channels_str = concat!("{", #channels_json, "}");
                let messages_str = concat!("{", #messages_json, "}");

                let channels: ::server_less::serde_json::Value =
                    ::server_less::serde_json::from_str(channels_str).unwrap_or_default();
                let messages: ::server_less::serde_json::Value =
                    ::server_less::serde_json::from_str(messages_str).unwrap_or_default();

                ::server_less::serde_json::json!({
                    "asyncapi": "2.6.0",
                    "info": {
                        "title": #title,
                        "version": #version
                    },
                    "servers": {
                        "default": {
                            "url": #server,
                            "protocol": "ws"
                        }
                    },
                    "channels": channels,
                    "components": {
                        "messages": messages
                    }
                })
            }

            /// Get the AsyncAPI spec as a JSON string.
            pub fn asyncapi_json() -> String {
                ::server_less::serde_json::to_string_pretty(&Self::asyncapi_spec())
                    .unwrap_or_else(|_| "{}".to_string())
            }

            /// Get the AsyncAPI spec as a JSON string (JSON is a valid subset of YAML).
            ///
            /// Note: returns JSON-formatted output. To get idiomatic YAML formatting,
            /// add `serde_yaml` to your project and call `serde_yaml::to_string(&Self::asyncapi_spec())`.
            pub fn asyncapi_yaml() -> String {
                Self::asyncapi_json()
            }

            /// Write the AsyncAPI spec to a file.
            pub fn write_asyncapi(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::asyncapi_json())
            }
        }
    })
}

/// Generate channel specification for a method
fn generate_channel_spec(method: &MethodInfo) -> String {
    let name = method.name_str().to_lower_camel_case();
    let cap_name = capitalize(&name);
    let description = method
        .docs
        .clone()
        .unwrap_or_else(|| format!("{} operation", name));

    format!(
        r##"
        "{}": {{
            "description": "{}",
            "publish": {{
                "operationId": "{}",
                "message": {{
                    "$ref": "#/components/messages/{}Request"
                }}
            }},
            "subscribe": {{
                "message": {{
                    "$ref": "#/components/messages/{}Response"
                }}
            }}
        }}"##,
        name,
        description.replace('"', "\\\""),
        name,
        cap_name,
        cap_name
    )
}

/// Generate message specification for a method
fn generate_message_spec(method: &MethodInfo) -> String {
    let name = method.name_str().to_lower_camel_case();
    let cap_name = capitalize(&name);

    let param_props: Vec<String> = method.params.iter().map(generate_param_property).collect();

    let result_schema = get_json_schema(&method.return_info.ty);

    format!(
        r#"
        "{}Request": {{
            "name": "{}Request",
            "payload": {{
                "type": "object",
                "properties": {{
                    {}
                }}
            }}
        }},
        "{}Response": {{
            "name": "{}Response",
            "payload": {}
        }}"#,
        cap_name,
        cap_name,
        param_props.join(",\n"),
        cap_name,
        cap_name,
        result_schema
    )
}

/// Generate parameter property
fn generate_param_property(param: &ParamInfo) -> String {
    let name = param.name_str().to_lower_camel_case();
    let schema = get_json_schema(&Some(param.ty.clone()));
    // L9: include help_text as the "description" property in the AsyncAPI schema.
    if let Some(help) = param.help_text.as_deref() {
        let escaped = help.replace('"', "\\\"");
        // Inject description into the schema object: strip trailing `}` and append field.
        let schema_with_desc = format!(
            r#"{}, "description": "{}"}}"#,
            &schema[..schema.len() - 1],
            escaped
        );
        format!(r#""{}": {}"#, name, schema_with_desc)
    } else {
        format!(r#""{}": {}"#, name, schema)
    }
}

/// Get JSON Schema for a type
fn get_json_schema(ty: &Option<syn::Type>) -> String {
    let Some(ty) = ty else {
        return r#"{"type": "null"}"#.to_string();
    };
    get_json_schema_ty(ty)
}

/// Get JSON Schema for a `syn::Type` reference.
fn get_json_schema_ty(ty: &syn::Type) -> String {
    // Unwrap Result<T, E> → T
    if let Some(ok) = unwrap_result_ok_type(ty) {
        return get_json_schema_ty(ok);
    }
    // M15: Option<T> → {"anyOf": [{"type": "null"}, <inner_schema>]}
    // Bare `null` is not valid JSON Schema; use {"type": "null"} instead.
    if let Some(inner) = unwrap_option_type(ty) {
        let inner_schema = get_json_schema_ty(inner);
        return format!(r#"{{"anyOf": [{{"type": "null"}}, {}]}}"#, inner_schema);
    }
    // Vec<T> → {"type": "array", "items": <inner_schema>}
    if let Some(inner) = unwrap_vec_type(ty) {
        let inner_schema = get_json_schema_ty(inner);
        return format!(r#"{{"type": "array", "items": {}}}"#, inner_schema);
    }
    let type_str = quote!(#ty).to_string();
    if type_str.contains("String") || type_str.contains("str") {
        r#"{"type": "string"}"#.to_string()
    } else if type_str.contains("i8")
        || type_str.contains("i16")
        || type_str.contains("i32")
        || type_str.contains("i64")
        || type_str.contains("u8")
        || type_str.contains("u16")
        || type_str.contains("u32")
        || type_str.contains("u64")
    {
        r#"{"type": "integer"}"#.to_string()
    } else if type_str.contains("f32") || type_str.contains("f64") {
        r#"{"type": "number"}"#.to_string()
    } else if type_str.contains("bool") {
        r#"{"type": "boolean"}"#.to_string()
    } else {
        r#"{"type": "object"}"#.to_string()
    }
}

/// Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}
