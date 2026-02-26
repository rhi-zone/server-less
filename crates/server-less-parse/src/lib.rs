//! Shared parsing utilities for server-less proc macros.
//!
//! This crate provides common types and functions for extracting
//! method information from impl blocks.

use syn::{
    FnArg, GenericArgument, Ident, ImplItem, ImplItemFn, ItemImpl, Lit, Meta, Pat, PathArguments,
    ReturnType, Type, TypeReference,
};

/// Parsed method information with full `syn` AST types.
///
/// This is the rich, compile-time representation used by all proc macros
/// during code generation. It retains full `syn::Type` and `syn::Ident`
/// nodes for accurate token generation.
///
/// **Not to be confused with [`server_less_core::MethodInfo`]**, which is
/// a simplified, string-based representation for runtime introspection.
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// The original method
    pub method: ImplItemFn,
    /// Method name
    pub name: Ident,
    /// Documentation string
    pub docs: Option<String>,
    /// Parameters (excluding self)
    pub params: Vec<ParamInfo>,
    /// Return type info
    pub return_info: ReturnInfo,
    /// Whether the method is async
    pub is_async: bool,
}

/// Parsed parameter information
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Parameter name
    pub name: Ident,
    /// Parameter type
    pub ty: Type,
    /// Whether this is `Option<T>`
    pub is_optional: bool,
    /// Whether this looks like an ID (ends with _id or is named id)
    pub is_id: bool,
    /// Custom wire name (from #[param(name = "...")])
    pub wire_name: Option<String>,
    /// Parameter location override (from #[param(query/path/body/header)])
    pub location: Option<ParamLocation>,
    /// Default value as a string (from #[param(default = ...)])
    pub default_value: Option<String>,
}

/// Parameter location for HTTP requests
#[derive(Debug, Clone, PartialEq)]
pub enum ParamLocation {
    Query,
    Path,
    Body,
    Header,
}

/// Parsed return type information
#[derive(Debug, Clone)]
pub struct ReturnInfo {
    /// The full return type
    pub ty: Option<Type>,
    /// Inner type if `Result<T, E>`
    pub ok_type: Option<Type>,
    /// Error type if `Result<T, E>`
    pub err_type: Option<Type>,
    /// Inner type if `Option<T>`
    pub some_type: Option<Type>,
    /// Whether it's a Result
    pub is_result: bool,
    /// Whether it's an Option (and not Result)
    pub is_option: bool,
    /// Whether it returns ()
    pub is_unit: bool,
    /// Whether it's impl Stream<Item=T>
    pub is_stream: bool,
    /// The stream item type if is_stream
    pub stream_item: Option<Type>,
    /// Whether the return type is a reference (&T)
    pub is_reference: bool,
    /// The inner type T if returning &T
    pub reference_inner: Option<Type>,
}

impl MethodInfo {
    /// Parse a method from an ImplItemFn
    ///
    /// Returns None for associated functions without `&self` (constructors, etc.)
    pub fn parse(method: &ImplItemFn) -> syn::Result<Option<Self>> {
        let name = method.sig.ident.clone();
        let is_async = method.sig.asyncness.is_some();

        // Skip associated functions without self receiver (constructors, etc.)
        let has_receiver = method
            .sig
            .inputs
            .iter()
            .any(|arg| matches!(arg, FnArg::Receiver(_)));
        if !has_receiver {
            return Ok(None);
        }

        // Extract doc comments
        let docs = extract_docs(&method.attrs);

        // Parse parameters
        let params = parse_params(&method.sig.inputs)?;

        // Parse return type
        let return_info = parse_return_type(&method.sig.output);

        Ok(Some(Self {
            method: method.clone(),
            name,
            docs,
            params,
            return_info,
            is_async,
        }))
    }
}

/// Extract doc comments from attributes
pub fn extract_docs(attrs: &[syn::Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let Meta::NameValue(meta) = &attr.meta
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
                }) = &meta.value
            {
                return Some(s.value().trim().to_string());
            }
            None
        })
        .collect();

    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

/// Parse #[param(...)] attributes from a parameter
pub fn parse_param_attrs(
    attrs: &[syn::Attribute],
) -> syn::Result<(Option<String>, Option<ParamLocation>, Option<String>)> {
    let mut wire_name = None;
    let mut location = None;
    let mut default_value = None;

    for attr in attrs {
        if !attr.path().is_ident("param") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            // #[param(name = "...")]
            if meta.path.is_ident("name") {
                let value: syn::LitStr = meta.value()?.parse()?;
                wire_name = Some(value.value());
                Ok(())
            }
            // #[param(default = ...)]
            else if meta.path.is_ident("default") {
                // Accept various literal types
                let value = meta.value()?;
                let lookahead = value.lookahead1();
                if lookahead.peek(syn::LitStr) {
                    let lit: syn::LitStr = value.parse()?;
                    default_value = Some(format!("\"{}\"", lit.value()));
                } else if lookahead.peek(syn::LitInt) {
                    let lit: syn::LitInt = value.parse()?;
                    default_value = Some(lit.to_string());
                } else if lookahead.peek(syn::LitBool) {
                    let lit: syn::LitBool = value.parse()?;
                    default_value = Some(lit.value.to_string());
                } else {
                    return Err(lookahead.error());
                }
                Ok(())
            }
            // #[param(query)] or #[param(path)] etc.
            else if meta.path.is_ident("query") {
                location = Some(ParamLocation::Query);
                Ok(())
            } else if meta.path.is_ident("path") {
                location = Some(ParamLocation::Path);
                Ok(())
            } else if meta.path.is_ident("body") {
                location = Some(ParamLocation::Body);
                Ok(())
            } else if meta.path.is_ident("header") {
                location = Some(ParamLocation::Header);
                Ok(())
            } else {
                Err(meta.error(
                    "unknown attribute\n\
                     \n\
                     Valid attributes: name, default, query, path, body, header\n\
                     \n\
                     Examples:\n\
                     - #[param(name = \"q\")]\n\
                     - #[param(default = 10)]\n\
                     - #[param(query)]\n\
                     - #[param(header, name = \"X-API-Key\")]",
                ))
            }
        })?;
    }

    Ok((wire_name, location, default_value))
}

/// Parse function parameters (excluding self)
pub fn parse_params(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
) -> syn::Result<Vec<ParamInfo>> {
    let mut params = Vec::new();

    for arg in inputs {
        match arg {
            FnArg::Receiver(_) => continue, // skip self
            FnArg::Typed(pat_type) => {
                let name = match pat_type.pat.as_ref() {
                    Pat::Ident(pat_ident) => pat_ident.ident.clone(),
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "unsupported parameter pattern\n\
                             \n\
                             Server-less macros require simple parameter names.\n\
                             Use: name: String\n\
                             Not: (name, _): (String, i32) or &name: &String",
                        ));
                    }
                };

                let ty = (*pat_type.ty).clone();
                let is_optional = is_option_type(&ty);
                let is_id = is_id_param(&name);

                // Parse #[param(...)] attributes
                let (wire_name, location, default_value) = parse_param_attrs(&pat_type.attrs)?;

                params.push(ParamInfo {
                    name,
                    ty,
                    is_optional,
                    is_id,
                    wire_name,
                    location,
                    default_value,
                });
            }
        }
    }

    Ok(params)
}

/// Parse return type information
pub fn parse_return_type(output: &ReturnType) -> ReturnInfo {
    match output {
        ReturnType::Default => ReturnInfo {
            ty: None,
            ok_type: None,
            err_type: None,
            some_type: None,
            is_result: false,
            is_option: false,
            is_unit: true,
            is_stream: false,
            stream_item: None,
            is_reference: false,
            reference_inner: None,
        },
        ReturnType::Type(_, ty) => {
            let ty = ty.as_ref().clone();

            // Check for Result<T, E>
            if let Some((ok, err)) = extract_result_types(&ty) {
                return ReturnInfo {
                    ty: Some(ty),
                    ok_type: Some(ok),
                    err_type: Some(err),
                    some_type: None,
                    is_result: true,
                    is_option: false,
                    is_unit: false,
                    is_stream: false,
                    stream_item: None,
                    is_reference: false,
                    reference_inner: None,
                };
            }

            // Check for Option<T>
            if let Some(inner) = extract_option_type(&ty) {
                return ReturnInfo {
                    ty: Some(ty),
                    ok_type: None,
                    err_type: None,
                    some_type: Some(inner),
                    is_result: false,
                    is_option: true,
                    is_unit: false,
                    is_stream: false,
                    stream_item: None,
                    is_reference: false,
                    reference_inner: None,
                };
            }

            // Check for impl Stream<Item=T>
            if let Some(item) = extract_stream_item(&ty) {
                return ReturnInfo {
                    ty: Some(ty),
                    ok_type: None,
                    err_type: None,
                    some_type: None,
                    is_result: false,
                    is_option: false,
                    is_unit: false,
                    is_stream: true,
                    stream_item: Some(item),
                    is_reference: false,
                    reference_inner: None,
                };
            }

            // Check for ()
            if is_unit_type(&ty) {
                return ReturnInfo {
                    ty: Some(ty),
                    ok_type: None,
                    err_type: None,
                    some_type: None,
                    is_result: false,
                    is_option: false,
                    is_unit: true,
                    is_stream: false,
                    stream_item: None,
                    is_reference: false,
                    reference_inner: None,
                };
            }

            // Check for &T (reference return — mount point)
            if let Type::Reference(TypeReference { elem, .. }) = &ty {
                let inner = elem.as_ref().clone();
                return ReturnInfo {
                    ty: Some(ty),
                    ok_type: None,
                    err_type: None,
                    some_type: None,
                    is_result: false,
                    is_option: false,
                    is_unit: false,
                    is_stream: false,
                    stream_item: None,
                    is_reference: true,
                    reference_inner: Some(inner),
                };
            }

            // Regular type
            ReturnInfo {
                ty: Some(ty),
                ok_type: None,
                err_type: None,
                some_type: None,
                is_result: false,
                is_option: false,
                is_unit: false,
                is_stream: false,
                stream_item: None,
                is_reference: false,
                reference_inner: None,
            }
        }
    }
}

/// Check if a type is `Option<T>` and extract T
pub fn extract_option_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }
    None
}

/// Check if a type is `Option<T>`
pub fn is_option_type(ty: &Type) -> bool {
    extract_option_type(ty).is_some()
}

/// Check if a type is Result<T, E> and extract T and E
pub fn extract_result_types(ty: &Type) -> Option<(Type, Type)> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Result"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
    {
        let mut iter = args.args.iter();
        if let (Some(GenericArgument::Type(ok)), Some(GenericArgument::Type(err))) =
            (iter.next(), iter.next())
        {
            return Some((ok.clone(), err.clone()));
        }
    }
    None
}

/// Check if a type is impl Stream<Item=T> and extract T
pub fn extract_stream_item(ty: &Type) -> Option<Type> {
    if let Type::ImplTrait(impl_trait) = ty {
        for bound in &impl_trait.bounds {
            if let syn::TypeParamBound::Trait(trait_bound) = bound
                && let Some(segment) = trait_bound.path.segments.last()
                && segment.ident == "Stream"
                && let PathArguments::AngleBracketed(args) = &segment.arguments
            {
                for arg in &args.args {
                    if let GenericArgument::AssocType(assoc) = arg
                        && assoc.ident == "Item"
                    {
                        return Some(assoc.ty.clone());
                    }
                }
            }
        }
    }
    None
}

/// Check if a type is ()
pub fn is_unit_type(ty: &Type) -> bool {
    if let Type::Tuple(tuple) = ty {
        return tuple.elems.is_empty();
    }
    false
}

/// Check if a parameter name looks like an ID
pub fn is_id_param(name: &Ident) -> bool {
    let name_str = name.to_string();
    name_str == "id" || name_str.ends_with("_id")
}

/// Extract all methods from an impl block
///
/// Skips:
/// - Private methods (starting with `_`)
/// - Associated functions without `&self` receiver (constructors, etc.)
pub fn extract_methods(impl_block: &ItemImpl) -> syn::Result<Vec<MethodInfo>> {
    let mut methods = Vec::new();

    for item in &impl_block.items {
        if let ImplItem::Fn(method) = item {
            // Skip private methods (those starting with _)
            if method.sig.ident.to_string().starts_with('_') {
                continue;
            }
            // Parse method - returns None for associated functions without self
            if let Some(info) = MethodInfo::parse(method)? {
                methods.push(info);
            }
        }
    }

    Ok(methods)
}

/// Categorized methods for code generation.
///
/// Methods returning `&T` (non-async) are mount points; everything else is a leaf.
/// Mount points are further split by whether they take parameters (slug) or not (static).
pub struct PartitionedMethods<'a> {
    /// Regular leaf methods (no reference return).
    pub leaf: Vec<&'a MethodInfo>,
    /// Static mounts: `fn foo(&self) -> &T` (no params).
    pub static_mounts: Vec<&'a MethodInfo>,
    /// Slug mounts: `fn foo(&self, id: Id) -> &T` (has params).
    pub slug_mounts: Vec<&'a MethodInfo>,
}

/// Partition methods into leaf commands, static mounts, and slug mounts.
///
/// The `skip` predicate allows each protocol to apply its own skip logic
/// (e.g., `#[cli(skip)]`, `#[mcp(skip)]`).
pub fn partition_methods<'a>(
    methods: &'a [MethodInfo],
    skip: impl Fn(&MethodInfo) -> bool,
) -> PartitionedMethods<'a> {
    let mut result = PartitionedMethods {
        leaf: Vec::new(),
        static_mounts: Vec::new(),
        slug_mounts: Vec::new(),
    };

    for method in methods {
        if skip(method) {
            continue;
        }

        if method.return_info.is_reference && !method.is_async {
            if method.params.is_empty() {
                result.static_mounts.push(method);
            } else {
                result.slug_mounts.push(method);
            }
        } else {
            result.leaf.push(method);
        }
    }

    result
}

/// Get the struct name from an impl block
pub fn get_impl_name(impl_block: &ItemImpl) -> syn::Result<Ident> {
    if let Type::Path(type_path) = impl_block.self_ty.as_ref()
        && let Some(segment) = type_path.path.segments.last()
    {
        return Ok(segment.ident.clone());
    }
    Err(syn::Error::new_spanned(
        &impl_block.self_ty,
        "Expected a simple type name",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    // ── extract_docs ────────────────────────────────────────────────

    #[test]
    fn extract_docs_returns_none_when_no_doc_attrs() {
        let method: ImplItemFn = syn::parse_quote! {
            fn hello(&self) {}
        };
        assert!(extract_docs(&method.attrs).is_none());
    }

    #[test]
    fn extract_docs_extracts_single_line() {
        let method: ImplItemFn = syn::parse_quote! {
            /// Hello world
            fn hello(&self) {}
        };
        assert_eq!(extract_docs(&method.attrs).unwrap(), "Hello world");
    }

    #[test]
    fn extract_docs_joins_multiple_lines() {
        let method: ImplItemFn = syn::parse_quote! {
            /// Line one
            /// Line two
            fn hello(&self) {}
        };
        assert_eq!(extract_docs(&method.attrs).unwrap(), "Line one\nLine two");
    }

    #[test]
    fn extract_docs_ignores_non_doc_attrs() {
        let method: ImplItemFn = syn::parse_quote! {
            #[inline]
            /// Documented
            fn hello(&self) {}
        };
        assert_eq!(extract_docs(&method.attrs).unwrap(), "Documented");
    }

    // ── parse_return_type ───────────────────────────────────────────

    #[test]
    fn parse_return_type_default_is_unit() {
        let ret: ReturnType = syn::parse_quote! {};
        let info = parse_return_type(&ret);
        assert!(info.is_unit);
        assert!(info.ty.is_none());
        assert!(!info.is_result);
        assert!(!info.is_option);
        assert!(!info.is_reference);
    }

    #[test]
    fn parse_return_type_regular_type() {
        let ret: ReturnType = syn::parse_quote! { -> String };
        let info = parse_return_type(&ret);
        assert!(!info.is_unit);
        assert!(!info.is_result);
        assert!(!info.is_option);
        assert!(!info.is_reference);
        assert!(info.ty.is_some());
    }

    #[test]
    fn parse_return_type_result() {
        let ret: ReturnType = syn::parse_quote! { -> Result<String, MyError> };
        let info = parse_return_type(&ret);
        assert!(info.is_result);
        assert!(!info.is_option);
        assert!(!info.is_unit);

        let ok = info.ok_type.unwrap();
        assert_eq!(quote!(#ok).to_string(), "String");

        let err = info.err_type.unwrap();
        assert_eq!(quote!(#err).to_string(), "MyError");
    }

    #[test]
    fn parse_return_type_option() {
        let ret: ReturnType = syn::parse_quote! { -> Option<i32> };
        let info = parse_return_type(&ret);
        assert!(info.is_option);
        assert!(!info.is_result);
        assert!(!info.is_unit);

        let some = info.some_type.unwrap();
        assert_eq!(quote!(#some).to_string(), "i32");
    }

    #[test]
    fn parse_return_type_unit_tuple() {
        let ret: ReturnType = syn::parse_quote! { -> () };
        let info = parse_return_type(&ret);
        assert!(info.is_unit);
        assert!(info.ty.is_some());
    }

    #[test]
    fn parse_return_type_reference() {
        let ret: ReturnType = syn::parse_quote! { -> &SubRouter };
        let info = parse_return_type(&ret);
        assert!(info.is_reference);
        assert!(!info.is_unit);

        let inner = info.reference_inner.unwrap();
        assert_eq!(quote!(#inner).to_string(), "SubRouter");
    }

    #[test]
    fn parse_return_type_stream() {
        let ret: ReturnType = syn::parse_quote! { -> impl Stream<Item = u64> };
        let info = parse_return_type(&ret);
        assert!(info.is_stream);
        assert!(!info.is_result);

        let item = info.stream_item.unwrap();
        assert_eq!(quote!(#item).to_string(), "u64");
    }

    // ── is_option_type / extract_option_type ────────────────────────

    #[test]
    fn is_option_type_true() {
        let ty: Type = syn::parse_quote! { Option<String> };
        assert!(is_option_type(&ty));
        let inner = extract_option_type(&ty).unwrap();
        assert_eq!(quote!(#inner).to_string(), "String");
    }

    #[test]
    fn is_option_type_false_for_non_option() {
        let ty: Type = syn::parse_quote! { String };
        assert!(!is_option_type(&ty));
        assert!(extract_option_type(&ty).is_none());
    }

    // ── extract_result_types ────────────────────────────────────────

    #[test]
    fn extract_result_types_works() {
        let ty: Type = syn::parse_quote! { Result<Vec<u8>, std::io::Error> };
        let (ok, err) = extract_result_types(&ty).unwrap();
        assert_eq!(quote!(#ok).to_string(), "Vec < u8 >");
        assert_eq!(quote!(#err).to_string(), "std :: io :: Error");
    }

    #[test]
    fn extract_result_types_none_for_non_result() {
        let ty: Type = syn::parse_quote! { Option<i32> };
        assert!(extract_result_types(&ty).is_none());
    }

    // ── is_unit_type ────────────────────────────────────────────────

    #[test]
    fn is_unit_type_true() {
        let ty: Type = syn::parse_quote! { () };
        assert!(is_unit_type(&ty));
    }

    #[test]
    fn is_unit_type_false_for_non_tuple() {
        let ty: Type = syn::parse_quote! { String };
        assert!(!is_unit_type(&ty));
    }

    #[test]
    fn is_unit_type_false_for_nonempty_tuple() {
        let ty: Type = syn::parse_quote! { (i32, i32) };
        assert!(!is_unit_type(&ty));
    }

    // ── is_id_param ─────────────────────────────────────────────────

    #[test]
    fn is_id_param_exact_id() {
        let ident: Ident = syn::parse_quote! { id };
        assert!(is_id_param(&ident));
    }

    #[test]
    fn is_id_param_suffix_id() {
        let ident: Ident = syn::parse_quote! { user_id };
        assert!(is_id_param(&ident));
    }

    #[test]
    fn is_id_param_false_for_other_names() {
        let ident: Ident = syn::parse_quote! { name };
        assert!(!is_id_param(&ident));
    }

    #[test]
    fn is_id_param_false_for_identity() {
        // "identity" ends with "id" but not "_id"
        let ident: Ident = syn::parse_quote! { identity };
        assert!(!is_id_param(&ident));
    }

    // ── MethodInfo::parse ───────────────────────────────────────────

    #[test]
    fn method_info_parse_basic() {
        let method: ImplItemFn = syn::parse_quote! {
            /// Does a thing
            fn greet(&self, name: String) -> String {
                format!("Hello {name}")
            }
        };
        let info = MethodInfo::parse(&method).unwrap().unwrap();
        assert_eq!(info.name.to_string(), "greet");
        assert!(!info.is_async);
        assert_eq!(info.docs.as_deref(), Some("Does a thing"));
        assert_eq!(info.params.len(), 1);
        assert_eq!(info.params[0].name.to_string(), "name");
        assert!(!info.params[0].is_optional);
        assert!(!info.params[0].is_id);
    }

    #[test]
    fn method_info_parse_async_method() {
        let method: ImplItemFn = syn::parse_quote! {
            async fn fetch(&self) -> Vec<u8> {
                vec![]
            }
        };
        let info = MethodInfo::parse(&method).unwrap().unwrap();
        assert!(info.is_async);
    }

    #[test]
    fn method_info_parse_skips_associated_function() {
        let method: ImplItemFn = syn::parse_quote! {
            fn new() -> Self {
                Self
            }
        };
        assert!(MethodInfo::parse(&method).unwrap().is_none());
    }

    #[test]
    fn method_info_parse_optional_param() {
        let method: ImplItemFn = syn::parse_quote! {
            fn search(&self, query: Option<String>) {}
        };
        let info = MethodInfo::parse(&method).unwrap().unwrap();
        assert!(info.params[0].is_optional);
    }

    #[test]
    fn method_info_parse_id_param() {
        let method: ImplItemFn = syn::parse_quote! {
            fn get_user(&self, user_id: u64) -> String {
                String::new()
            }
        };
        let info = MethodInfo::parse(&method).unwrap().unwrap();
        assert!(info.params[0].is_id);
    }

    #[test]
    fn method_info_parse_no_docs() {
        let method: ImplItemFn = syn::parse_quote! {
            fn bare(&self) {}
        };
        let info = MethodInfo::parse(&method).unwrap().unwrap();
        assert!(info.docs.is_none());
    }

    // ── extract_methods ─────────────────────────────────────────────

    #[test]
    fn extract_methods_basic() {
        let impl_block: ItemImpl = syn::parse_quote! {
            impl MyApi {
                fn hello(&self) -> String { String::new() }
                fn world(&self) -> String { String::new() }
            }
        };
        let methods = extract_methods(&impl_block).unwrap();
        assert_eq!(methods.len(), 2);
        assert_eq!(methods[0].name.to_string(), "hello");
        assert_eq!(methods[1].name.to_string(), "world");
    }

    #[test]
    fn extract_methods_skips_underscore_prefix() {
        let impl_block: ItemImpl = syn::parse_quote! {
            impl MyApi {
                fn public(&self) {}
                fn _private(&self) {}
                fn __also_private(&self) {}
            }
        };
        let methods = extract_methods(&impl_block).unwrap();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name.to_string(), "public");
    }

    #[test]
    fn extract_methods_skips_associated_functions() {
        let impl_block: ItemImpl = syn::parse_quote! {
            impl MyApi {
                fn new() -> Self { Self }
                fn from_config(cfg: Config) -> Self { Self }
                fn greet(&self) -> String { String::new() }
            }
        };
        let methods = extract_methods(&impl_block).unwrap();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name.to_string(), "greet");
    }

    // ── partition_methods ───────────────────────────────────────────

    #[test]
    fn partition_methods_splits_correctly() {
        let impl_block: ItemImpl = syn::parse_quote! {
            impl Router {
                fn leaf_action(&self) -> String { String::new() }
                fn static_mount(&self) -> &SubRouter { &self.sub }
                fn slug_mount(&self, id: u64) -> &SubRouter { &self.sub }
                async fn async_ref(&self) -> &SubRouter { &self.sub }
            }
        };
        let methods = extract_methods(&impl_block).unwrap();
        let partitioned = partition_methods(&methods, |_| false);

        // leaf_action and async_ref (async reference returns are leaf, not mounts)
        assert_eq!(partitioned.leaf.len(), 2);
        assert_eq!(partitioned.leaf[0].name.to_string(), "leaf_action");
        assert_eq!(partitioned.leaf[1].name.to_string(), "async_ref");

        assert_eq!(partitioned.static_mounts.len(), 1);
        assert_eq!(
            partitioned.static_mounts[0].name.to_string(),
            "static_mount"
        );

        assert_eq!(partitioned.slug_mounts.len(), 1);
        assert_eq!(partitioned.slug_mounts[0].name.to_string(), "slug_mount");
    }

    #[test]
    fn partition_methods_respects_skip() {
        let impl_block: ItemImpl = syn::parse_quote! {
            impl Router {
                fn keep(&self) -> String { String::new() }
                fn skip_me(&self) -> String { String::new() }
            }
        };
        let methods = extract_methods(&impl_block).unwrap();
        let partitioned = partition_methods(&methods, |m| m.name == "skip_me");

        assert_eq!(partitioned.leaf.len(), 1);
        assert_eq!(partitioned.leaf[0].name.to_string(), "keep");
    }

    // ── get_impl_name ───────────────────────────────────────────────

    #[test]
    fn get_impl_name_extracts_struct_name() {
        let impl_block: ItemImpl = syn::parse_quote! {
            impl MyService {
                fn hello(&self) {}
            }
        };
        let name = get_impl_name(&impl_block).unwrap();
        assert_eq!(name.to_string(), "MyService");
    }

    #[test]
    fn get_impl_name_with_generics() {
        let impl_block: ItemImpl = syn::parse_quote! {
            impl MyService<T> {
                fn hello(&self) {}
            }
        };
        let name = get_impl_name(&impl_block).unwrap();
        assert_eq!(name.to_string(), "MyService");
    }
}
