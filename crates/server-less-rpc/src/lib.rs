//! Shared utilities for RPC-style macros (MCP, WebSocket, JSON-RPC).
//!
//! These macros use JSON-RPC-like dispatch:
//! - Receive `{"method": "name", "params": {...}}`
//! - Extract params from JSON
//! - Call the method
//! - Serialize result back to JSON

use proc_macro2::TokenStream;
use quote::quote;
use server_less_parse::{MethodInfo, ParamInfo};

/// Generate code to extract a parameter from a `serde_json::Value` args object.
pub fn generate_param_extraction(param: &ParamInfo) -> TokenStream {
    let name = &param.name;
    let name_str = param.name.to_string();
    let ty = &param.ty;

    if param.is_optional {
        // For Option<T>, extract inner value, return None if missing/null
        quote! {
            let #name: #ty = args.get(#name_str)
                .and_then(|v| if v.is_null() { None } else {
                    ::server_less::serde_json::from_value(v.clone()).ok()
                });
        }
    } else {
        // Required parameter - error if missing
        quote! {
            let __val = args.get(#name_str)
                .ok_or_else(|| format!("Missing required parameter: {}", #name_str))?
                .clone();
            let #name: #ty = ::server_less::serde_json::from_value::<#ty>(__val)
                .map_err(|e| format!("Invalid parameter {}: {}", #name_str, e))?;
        }
    }
}

/// Generate all param extractions for a method.
pub fn generate_all_param_extractions(method: &MethodInfo) -> Vec<TokenStream> {
    method
        .params
        .iter()
        .map(generate_param_extraction)
        .collect()
}

/// Generate param extractions for specific parameters only.
///
/// This allows filtering out framework-injected params (like Context)
/// that shouldn't be extracted from JSON.
pub fn generate_param_extractions_for(params: &[&ParamInfo]) -> Vec<TokenStream> {
    params
        .iter()
        .map(|p| generate_param_extraction(p))
        .collect()
}

/// Generate the method call expression.
///
/// Returns tokens for calling `self.method_name(arg1, arg2, ...)`.
/// For async methods, returns an error (caller should handle async context).
pub fn generate_method_call(method: &MethodInfo, handle_async: AsyncHandling) -> TokenStream {
    let method_name = &method.name;
    let arg_names: Vec<_> = method.params.iter().map(|p| &p.name).collect();

    match (method.is_async, handle_async) {
        (true, AsyncHandling::Error) => {
            quote! {
                return Err("Async methods not supported in sync context".to_string());
            }
        }
        (true, AsyncHandling::Await) => {
            quote! {
                let result = self.#method_name(#(#arg_names),*).await;
            }
        }
        (true, AsyncHandling::BlockOn) => {
            quote! {
                let result = ::tokio::runtime::Runtime::new()
                    .expect("Failed to create Tokio runtime")
                    .block_on(self.#method_name(#(#arg_names),*));
            }
        }
        (false, _) => {
            quote! {
                let result = self.#method_name(#(#arg_names),*);
            }
        }
    }
}

/// Generate method call with custom argument expressions.
///
/// This allows mixing framework-injected args (like `__ctx`) with
/// params extracted from JSON.
pub fn generate_method_call_with_args(
    method: &MethodInfo,
    arg_exprs: Vec<TokenStream>,
    handle_async: AsyncHandling,
) -> TokenStream {
    let method_name = &method.name;

    match (method.is_async, handle_async) {
        (true, AsyncHandling::Error) => {
            quote! {
                return Err("Async methods not supported in sync context".to_string());
            }
        }
        (true, AsyncHandling::Await) => {
            quote! {
                let result = self.#method_name(#(#arg_exprs),*).await;
            }
        }
        (true, AsyncHandling::BlockOn) => {
            quote! {
                let result = ::tokio::runtime::Runtime::new()
                    .expect("Failed to create Tokio runtime")
                    .block_on(self.#method_name(#(#arg_exprs),*));
            }
        }
        (false, _) => {
            quote! {
                let result = self.#method_name(#(#arg_exprs),*);
            }
        }
    }
}

/// How to handle async methods.
#[derive(Debug, Clone, Copy)]
pub enum AsyncHandling {
    /// Return an error if method is async
    Error,
    /// Await the method (caller must be async)
    Await,
    /// Use tokio::runtime::Runtime::block_on
    BlockOn,
}

/// Generate response handling that converts the method result to JSON.
///
/// Handles:
/// - `()` → `{"success": true}`
/// - `Result<T, E>` → `Ok(T)` or `Err(message)`
/// - `Option<T>` → `T` or `null`
/// - `T` → serialized T
pub fn generate_json_response(method: &MethodInfo) -> TokenStream {
    let ret = &method.return_info;

    if ret.is_unit {
        quote! {
            Ok(::server_less::serde_json::json!({"success": true}))
        }
    } else if ret.is_stream {
        // Automatically collect streams into Vec for JSON serialization
        quote! {
            {
                use ::server_less::futures::StreamExt;
                let collected: Vec<_> = result.collect().await;
                Ok(::server_less::serde_json::to_value(collected)
                    .map_err(|e| format!("Serialization error: {}", e))?)
            }
        }
    } else if ret.is_iterator {
        // Collect iterator into Vec before serializing (Iterator doesn't implement Serialize)
        quote! {
            {
                let __collected: Vec<_> = result.collect();
                Ok(::server_less::serde_json::to_value(&__collected)
                    .map_err(|e| format!("Serialization error: {}", e))?)
            }
        }
    } else if ret.is_result {
        quote! {
            match result {
                Ok(value) => Ok(::server_less::serde_json::to_value(value)
                    .map_err(|e| format!("Serialization error: {}", e))?),
                Err(err) => Err(format!("{:?}", err)),
            }
        }
    } else if ret.is_option {
        quote! {
            match result {
                Some(value) => Ok(::server_less::serde_json::to_value(value)
                    .map_err(|e| format!("Serialization error: {}", e))?),
                None => Ok(::server_less::serde_json::Value::Null),
            }
        }
    } else {
        // Plain T
        quote! {
            Ok(::server_less::serde_json::to_value(result)
                .map_err(|e| format!("Serialization error: {}", e))?)
        }
    }
}

/// Generate a complete dispatch match arm for an RPC method.
///
/// Combines param extraction, method call, and response handling.
pub fn generate_dispatch_arm(
    method: &MethodInfo,
    method_name_override: Option<&str>,
    async_handling: AsyncHandling,
) -> TokenStream {
    let method_name_str = method_name_override
        .map(String::from)
        .unwrap_or_else(|| method.name.to_string());

    // Methods that are async OR return streams require async context
    let requires_async = method.is_async || method.return_info.is_stream;

    // For methods requiring async with Error handling, return early
    if requires_async && matches!(async_handling, AsyncHandling::Error) {
        return quote! {
            #method_name_str => {
                return Err("Async methods and streaming methods not supported in sync context".to_string());
            }
        };
    }

    let param_extractions = generate_all_param_extractions(method);
    let call = generate_method_call(method, async_handling);
    let response = generate_json_response(method);

    quote! {
        #method_name_str => {
            #(#param_extractions)*
            #call
            #response
        }
    }
}

/// Generate a dispatch arm with support for injected parameters.
///
/// Parameters whose index appears in `injected_params` will use the provided
/// TokenStream expression instead of being deserialized from JSON. This is
/// used for mount trait dispatch where Context/WsSender need injection.
pub fn generate_dispatch_arm_with_injections(
    method: &MethodInfo,
    method_name_override: Option<&str>,
    async_handling: AsyncHandling,
    injected_params: &[(usize, TokenStream)],
) -> TokenStream {
    let method_name_str = method_name_override
        .map(String::from)
        .unwrap_or_else(|| method.name.to_string());

    // Methods that are async OR return streams require async context
    let requires_async = method.is_async || method.return_info.is_stream;

    // For methods requiring async with Error handling, return early
    if requires_async && matches!(async_handling, AsyncHandling::Error) {
        return quote! {
            #method_name_str => {
                return Err("Async methods and streaming methods not supported in sync context".to_string());
            }
        };
    }

    // Generate param extractions, substituting injected params
    let param_extractions: Vec<TokenStream> = method
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if let Some((_, injection)) = injected_params.iter().find(|(idx, _)| *idx == i) {
                let name = &p.name;
                quote! { let #name = #injection; }
            } else {
                generate_param_extraction(p)
            }
        })
        .collect();

    let call = generate_method_call(method, async_handling);
    let response = generate_json_response(method);

    quote! {
        #method_name_str => {
            #(#param_extractions)*
            #call
            #response
        }
    }
}

/// Infer JSON schema type from Rust type.
pub fn infer_json_type(ty: &syn::Type) -> &'static str {
    let ty_str = quote!(#ty).to_string();

    if ty_str.contains("String") || ty_str.contains("str") {
        "string"
    } else if ty_str.contains("i8")
        || ty_str.contains("i16")
        || ty_str.contains("i32")
        || ty_str.contains("i64")
        || ty_str.contains("u8")
        || ty_str.contains("u16")
        || ty_str.contains("u32")
        || ty_str.contains("u64")
        || ty_str.contains("isize")
        || ty_str.contains("usize")
    {
        "integer"
    } else if ty_str.contains("f32") || ty_str.contains("f64") {
        "number"
    } else if ty_str.contains("bool") {
        "boolean"
    } else if ty_str.contains("Vec") || ty_str.contains("[]") {
        "array"
    } else {
        "object"
    }
}

/// Generate JSON schema properties for method parameters.
pub fn generate_param_schema(params: &[ParamInfo]) -> (Vec<TokenStream>, Vec<String>) {
    let properties: Vec<_> = params
        .iter()
        .map(|p| {
            let param_name = p.name.to_string();
            let param_type = infer_json_type(&p.ty);
            let description = format!("Parameter: {}", param_name);

            quote! {
                (#param_name, #param_type, #description)
            }
        })
        .collect();

    let required: Vec<_> = params
        .iter()
        .filter(|p| !p.is_optional)
        .map(|p| p.name.to_string())
        .collect();

    (properties, required)
}

/// Generate JSON schema properties for specific parameters (e.g., excluding Context).
pub fn generate_param_schema_for(params: &[&ParamInfo]) -> (Vec<TokenStream>, Vec<String>) {
    let properties: Vec<_> = params
        .iter()
        .map(|p| {
            let param_name = p.name.to_string();
            let param_type = infer_json_type(&p.ty);
            let description = format!("Parameter: {}", param_name);

            quote! {
                (#param_name, #param_type, #description)
            }
        })
        .collect();

    let required: Vec<_> = params
        .iter()
        .filter(|p| !p.is_optional)
        .map(|p| p.name.to_string())
        .collect();

    (properties, required)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::ImplItemFn;

    /// Helper: parse a method signature string into a MethodInfo.
    /// The method must have a `&self` receiver.
    fn parse_method(tokens: proc_macro2::TokenStream) -> MethodInfo {
        let method: ImplItemFn = syn::parse2(tokens).expect("failed to parse method");
        MethodInfo::parse(&method)
            .expect("MethodInfo::parse failed")
            .expect("method was skipped (no self receiver?)")
    }

    // ---------------------------------------------------------------
    // infer_json_type
    // ---------------------------------------------------------------

    #[test]
    fn infer_json_type_string() {
        let ty: syn::Type = syn::parse_quote!(String);
        assert_eq!(infer_json_type(&ty), "string");
    }

    #[test]
    fn infer_json_type_str_ref() {
        let ty: syn::Type = syn::parse_quote!(&str);
        assert_eq!(infer_json_type(&ty), "string");
    }

    #[test]
    fn infer_json_type_integers() {
        for type_str in &[
            "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "isize", "usize",
        ] {
            let ty: syn::Type =
                syn::parse_str(type_str).unwrap_or_else(|_| panic!("parse {}", type_str));
            assert_eq!(
                infer_json_type(&ty),
                "integer",
                "expected 'integer' for {}",
                type_str
            );
        }
    }

    #[test]
    fn infer_json_type_floats() {
        let ty_f32: syn::Type = syn::parse_quote!(f32);
        assert_eq!(infer_json_type(&ty_f32), "number");

        let ty_f64: syn::Type = syn::parse_quote!(f64);
        assert_eq!(infer_json_type(&ty_f64), "number");
    }

    #[test]
    fn infer_json_type_bool() {
        let ty: syn::Type = syn::parse_quote!(bool);
        assert_eq!(infer_json_type(&ty), "boolean");
    }

    #[test]
    fn infer_json_type_vec() {
        // Vec<T> where T doesn't match an earlier rule maps to "array"
        let ty: syn::Type = syn::parse_quote!(Vec<MyItem>);
        assert_eq!(infer_json_type(&ty), "array");
    }

    #[test]
    fn infer_json_type_vec_string_matches_string_first() {
        // Note: infer_json_type uses string matching, so Vec<String>
        // matches "String" before "Vec", returning "string".
        // This documents the current behavior.
        let ty: syn::Type = syn::parse_quote!(Vec<String>);
        assert_eq!(infer_json_type(&ty), "string");
    }

    #[test]
    fn infer_json_type_custom_struct() {
        let ty: syn::Type = syn::parse_quote!(MyCustomStruct);
        assert_eq!(infer_json_type(&ty), "object");
    }

    // ---------------------------------------------------------------
    // generate_param_schema
    // ---------------------------------------------------------------

    #[test]
    fn param_schema_required_params() {
        let method = parse_method(quote! {
            fn greet(&self, name: String, age: u32) {}
        });

        let (properties, required) = generate_param_schema(&method.params);

        assert_eq!(properties.len(), 2);
        assert_eq!(required, vec!["name", "age"]);
    }

    #[test]
    fn param_schema_optional_params_excluded_from_required() {
        let method = parse_method(quote! {
            fn search(&self, query: String, limit: Option<u32>) {}
        });

        let (properties, required) = generate_param_schema(&method.params);

        assert_eq!(properties.len(), 2);
        assert_eq!(required, vec!["query"]);
        assert!(!required.contains(&"limit".to_string()));
    }

    #[test]
    fn param_schema_all_optional() {
        let method = parse_method(quote! {
            fn list(&self, offset: Option<u32>, limit: Option<u32>) {}
        });

        let (_properties, required) = generate_param_schema(&method.params);
        assert!(required.is_empty());
    }

    #[test]
    fn param_schema_no_params() {
        let method = parse_method(quote! {
            fn ping(&self) {}
        });

        let (properties, required) = generate_param_schema(&method.params);
        assert!(properties.is_empty());
        assert!(required.is_empty());
    }

    // ---------------------------------------------------------------
    // generate_param_extraction
    // ---------------------------------------------------------------

    #[test]
    fn param_extraction_optional_uses_and_then() {
        let method = parse_method(quote! {
            fn search(&self, limit: Option<u32>) {}
        });

        let tokens = generate_param_extraction(&method.params[0]);
        let code = tokens.to_string();

        assert!(
            code.contains("and_then"),
            "optional param should use and_then pattern, got: {}",
            code
        );
        assert!(
            !code.contains("ok_or_else"),
            "optional param should NOT use ok_or_else, got: {}",
            code
        );
    }

    #[test]
    fn param_extraction_required_uses_ok_or_else() {
        let method = parse_method(quote! {
            fn greet(&self, name: String) {}
        });

        let tokens = generate_param_extraction(&method.params[0]);
        let code = tokens.to_string();

        assert!(
            code.contains("ok_or_else"),
            "required param should use ok_or_else pattern, got: {}",
            code
        );
        assert!(
            !code.contains("and_then"),
            "required param should NOT use and_then, got: {}",
            code
        );
    }

    #[test]
    fn param_extraction_references_correct_name() {
        let method = parse_method(quote! {
            fn greet(&self, user_name: String) {}
        });

        let tokens = generate_param_extraction(&method.params[0]);
        let code = tokens.to_string();

        assert!(
            code.contains("\"user_name\""),
            "extraction should reference param name string, got: {}",
            code
        );
    }

    // ---------------------------------------------------------------
    // generate_method_call
    // ---------------------------------------------------------------

    #[test]
    fn method_call_sync() {
        let method = parse_method(quote! {
            fn ping(&self) {}
        });

        let tokens = generate_method_call(&method, AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("self . ping"),
            "sync call should invoke self.ping, got: {}",
            code
        );
        assert!(
            !code.contains("await"),
            "sync call should not contain await, got: {}",
            code
        );
    }

    #[test]
    fn method_call_sync_with_args() {
        let method = parse_method(quote! {
            fn greet(&self, name: String, count: u32) {}
        });

        let tokens = generate_method_call(&method, AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("self . greet"),
            "should call self.greet, got: {}",
            code
        );
        assert!(code.contains("name"), "should pass name arg, got: {}", code);
        assert!(
            code.contains("count"),
            "should pass count arg, got: {}",
            code
        );
    }

    #[test]
    fn method_call_async_error() {
        let method = parse_method(quote! {
            async fn fetch(&self) -> String { todo!() }
        });

        let tokens = generate_method_call(&method, AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("Err") || code.contains("return"),
            "async + Error should return an error, got: {}",
            code
        );
        assert!(
            code.contains("not supported"),
            "error message should mention not supported, got: {}",
            code
        );
    }

    #[test]
    fn method_call_async_await() {
        let method = parse_method(quote! {
            async fn fetch(&self) -> String { todo!() }
        });

        let tokens = generate_method_call(&method, AsyncHandling::Await);
        let code = tokens.to_string();

        assert!(
            code.contains(". await"),
            "async + Await should contain .await, got: {}",
            code
        );
    }

    #[test]
    fn method_call_async_block_on() {
        let method = parse_method(quote! {
            async fn fetch(&self) -> String { todo!() }
        });

        let tokens = generate_method_call(&method, AsyncHandling::BlockOn);
        let code = tokens.to_string();

        assert!(
            code.contains("block_on"),
            "async + BlockOn should contain block_on, got: {}",
            code
        );
        assert!(
            code.contains("Runtime"),
            "should reference tokio Runtime, got: {}",
            code
        );
    }

    // ---------------------------------------------------------------
    // generate_json_response
    // ---------------------------------------------------------------

    #[test]
    fn json_response_unit() {
        let method = parse_method(quote! {
            fn ping(&self) {}
        });

        let tokens = generate_json_response(&method);
        let code = tokens.to_string();

        assert!(
            code.contains("success"),
            "unit return should produce success: true, got: {}",
            code
        );
    }

    #[test]
    fn json_response_result() {
        let method = parse_method(quote! {
            fn get(&self) -> Result<String, String> { todo!() }
        });

        let tokens = generate_json_response(&method);
        let code = tokens.to_string();

        assert!(
            code.contains("Ok"),
            "Result return should match Ok, got: {}",
            code
        );
        assert!(
            code.contains("Err"),
            "Result return should match Err, got: {}",
            code
        );
    }

    #[test]
    fn json_response_option() {
        let method = parse_method(quote! {
            fn find(&self) -> Option<String> { todo!() }
        });

        let tokens = generate_json_response(&method);
        let code = tokens.to_string();

        assert!(
            code.contains("Some"),
            "Option return should match Some, got: {}",
            code
        );
        assert!(
            code.contains("None"),
            "Option return should match None, got: {}",
            code
        );
        assert!(
            code.contains("Null"),
            "Option None should produce Null, got: {}",
            code
        );
    }

    #[test]
    fn json_response_plain_type() {
        let method = parse_method(quote! {
            fn count(&self) -> u64 { todo!() }
        });

        let tokens = generate_json_response(&method);
        let code = tokens.to_string();

        assert!(
            code.contains("to_value"),
            "plain return should serialize with to_value, got: {}",
            code
        );
        // Should NOT have Ok/Err match arms for Result or Some/None for Option
        assert!(
            !code.contains("match"),
            "plain return should not have match, got: {}",
            code
        );
    }

    // ---------------------------------------------------------------
    // generate_dispatch_arm
    // ---------------------------------------------------------------

    #[test]
    fn dispatch_arm_contains_method_name_string() {
        let method = parse_method(quote! {
            fn greet(&self, name: String) -> String { todo!() }
        });

        let tokens = generate_dispatch_arm(&method, None, AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("\"greet\""),
            "dispatch arm should match on method name string, got: {}",
            code
        );
    }

    #[test]
    fn dispatch_arm_with_name_override() {
        let method = parse_method(quote! {
            fn greet(&self, name: String) -> String { todo!() }
        });

        let tokens = generate_dispatch_arm(&method, Some("say_hello"), AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("\"say_hello\""),
            "dispatch arm should use overridden name, got: {}",
            code
        );
        assert!(
            !code.contains("\"greet\""),
            "dispatch arm should not use original name when overridden, got: {}",
            code
        );
    }

    #[test]
    fn dispatch_arm_includes_param_extraction() {
        let method = parse_method(quote! {
            fn greet(&self, name: String) -> String { todo!() }
        });

        let tokens = generate_dispatch_arm(&method, None, AsyncHandling::Error);
        let code = tokens.to_string();

        // Should include param extraction for "name"
        assert!(
            code.contains("\"name\""),
            "dispatch arm should extract 'name' param, got: {}",
            code
        );
    }

    #[test]
    fn dispatch_arm_includes_method_call_and_response() {
        let method = parse_method(quote! {
            fn ping(&self) {}
        });

        let tokens = generate_dispatch_arm(&method, None, AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("self . ping"),
            "dispatch arm should call self.ping, got: {}",
            code
        );
        assert!(
            code.contains("success"),
            "dispatch arm for unit return should include success response, got: {}",
            code
        );
    }

    #[test]
    fn dispatch_arm_async_error_returns_early() {
        let method = parse_method(quote! {
            async fn fetch(&self) -> String { todo!() }
        });

        let tokens = generate_dispatch_arm(&method, None, AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("not supported"),
            "async dispatch with Error handling should return error, got: {}",
            code
        );
    }

    #[test]
    fn dispatch_arm_async_await() {
        let method = parse_method(quote! {
            async fn fetch(&self, url: String) -> Result<String, String> { todo!() }
        });

        let tokens = generate_dispatch_arm(&method, None, AsyncHandling::Await);
        let code = tokens.to_string();

        assert!(
            code.contains(". await"),
            "async dispatch with Await should contain .await, got: {}",
            code
        );
        assert!(
            code.contains("\"url\""),
            "should extract url param, got: {}",
            code
        );
    }

    // ---------------------------------------------------------------
    // generate_dispatch_arm_with_injections
    // ---------------------------------------------------------------

    #[test]
    fn dispatch_arm_with_injections_replaces_injected_param() {
        let method = parse_method(quote! {
            fn handle(&self, ctx: Context, name: String) -> String { todo!() }
        });

        let injection = quote! { __ctx.clone() };
        let tokens = generate_dispatch_arm_with_injections(
            &method,
            None,
            AsyncHandling::Error,
            &[(0, injection)],
        );
        let code = tokens.to_string();

        // The injected param should use the injection expression
        assert!(
            code.contains("__ctx"),
            "injected param should use provided expression, got: {}",
            code
        );
        // The non-injected param should still be extracted from JSON
        assert!(
            code.contains("\"name\""),
            "non-injected param should be extracted from JSON, got: {}",
            code
        );
    }

    // ---------------------------------------------------------------
    // generate_all_param_extractions
    // ---------------------------------------------------------------

    #[test]
    fn all_param_extractions_generates_one_per_param() {
        let method = parse_method(quote! {
            fn create(&self, name: String, value: i32, label: Option<String>) {}
        });

        let extractions = generate_all_param_extractions(&method);
        assert_eq!(
            extractions.len(),
            3,
            "should generate one extraction per param"
        );
    }

    // ---------------------------------------------------------------
    // generate_param_extractions_for (subset)
    // ---------------------------------------------------------------

    #[test]
    fn param_extractions_for_subset() {
        let method = parse_method(quote! {
            fn handle(&self, ctx: Context, name: String, age: u32) {}
        });

        // Only generate extractions for name and age, not ctx
        let subset: Vec<&ParamInfo> = method.params.iter().skip(1).collect();
        let extractions = generate_param_extractions_for(&subset);
        assert_eq!(extractions.len(), 2);

        let code = extractions
            .iter()
            .map(|t| t.to_string())
            .collect::<String>();
        assert!(
            !code.contains("\"ctx\""),
            "should not extract ctx, got: {}",
            code
        );
        assert!(
            code.contains("\"name\""),
            "should extract name, got: {}",
            code
        );
    }

    // ---------------------------------------------------------------
    // generate_method_call_with_args
    // ---------------------------------------------------------------

    #[test]
    fn method_call_with_custom_args() {
        let method = parse_method(quote! {
            fn handle(&self, ctx: Context, name: String) -> String { todo!() }
        });

        let args = vec![quote! { __ctx }, quote! { name }];
        let tokens = generate_method_call_with_args(&method, args, AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("__ctx"),
            "should pass custom arg expression, got: {}",
            code
        );
        assert!(
            code.contains("self . handle"),
            "should call self.handle, got: {}",
            code
        );
    }

    // ---------------------------------------------------------------
    // generate_param_schema_for (subset)
    // ---------------------------------------------------------------

    #[test]
    fn param_schema_for_subset() {
        let method = parse_method(quote! {
            fn handle(&self, ctx: Context, name: String, limit: Option<u32>) {}
        });

        let subset: Vec<&ParamInfo> = method.params.iter().skip(1).collect();
        let (properties, required) = generate_param_schema_for(&subset);

        assert_eq!(properties.len(), 2);
        assert_eq!(required, vec!["name"]);
        assert!(!required.contains(&"ctx".to_string()));
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn dispatch_arm_no_params_unit_return() {
        let method = parse_method(quote! {
            fn health_check(&self) {}
        });

        let tokens = generate_dispatch_arm(&method, None, AsyncHandling::Error);
        let code = tokens.to_string();

        assert!(
            code.contains("\"health_check\""),
            "should match on method name, got: {}",
            code
        );
        assert!(
            code.contains("success"),
            "unit return should produce success, got: {}",
            code
        );
    }

    #[test]
    fn infer_json_type_option_string_is_string() {
        // Option<String> contains "String" so it maps to "string"
        let ty: syn::Type = syn::parse_quote!(Option<String>);
        assert_eq!(infer_json_type(&ty), "string");
    }

    #[test]
    fn infer_json_type_vec_u8_matches_integer_first() {
        // Vec<u8> matches "u8" (integer) before "Vec" (array) due to
        // string-based matching order. This documents the current behavior.
        let ty: syn::Type = syn::parse_quote!(Vec<u8>);
        assert_eq!(infer_json_type(&ty), "integer");
    }

    #[test]
    fn method_call_sync_ignores_async_handling_variant() {
        // A sync method should generate the same code regardless of AsyncHandling variant
        let method = parse_method(quote! {
            fn ping(&self) {}
        });

        let code_error = generate_method_call(&method, AsyncHandling::Error).to_string();
        let code_await = generate_method_call(&method, AsyncHandling::Await).to_string();
        let code_block = generate_method_call(&method, AsyncHandling::BlockOn).to_string();

        assert_eq!(code_error, code_await);
        assert_eq!(code_await, code_block);
    }
}
