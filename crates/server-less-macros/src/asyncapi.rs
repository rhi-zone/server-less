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

use heck::ToLowerCamelCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};
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
                "title" => {
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
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`. Valid arguments: title, version, server"
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
    impl_block: ItemImpl,
) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let title = args.title.unwrap_or_else(|| struct_name_str.clone());
    let version = args.version.unwrap_or_else(|| "1.0.0".to_string());
    let server = args
        .server
        .unwrap_or_else(|| "ws://localhost:8080".to_string());

    // Generate channel specs for each method
    let channel_specs: Vec<String> = methods.iter().map(generate_channel_spec).collect();
    let channels_json = channel_specs.join(",\n");

    // Generate message schemas
    let message_specs: Vec<String> = methods.iter().map(generate_message_spec).collect();
    let messages_json = message_specs.join(",\n");

    Ok(quote! {
        #impl_block

        impl #struct_name {
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

            /// Get the AsyncAPI spec as YAML string.
            pub fn asyncapi_yaml() -> String {
                // Simple JSON to YAML-ish conversion for readability
                Self::asyncapi_json()
                    .replace("{", "")
                    .replace("}", "")
                    .replace("[", "")
                    .replace("]", "")
                    .replace("\":", ":")
                    .replace("\",", "")
                    .replace("\"", "")
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
    let name = method.name.to_string().to_lower_camel_case();
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
    let name = method.name.to_string().to_lower_camel_case();
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
    let name = param.name.to_string().to_lower_camel_case();
    let schema = get_json_schema(&Some(param.ty.clone()));
    format!(r#""{}": {}"#, name, schema)
}

/// Get JSON Schema for a type
fn get_json_schema(ty: &Option<syn::Type>) -> String {
    let Some(ty) = ty else {
        return r#"{"type": "null"}"#.to_string();
    };

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
    } else if type_str.contains("Vec") {
        r#"{"type": "array"}"#.to_string()
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
