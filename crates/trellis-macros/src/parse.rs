//! Common parsing utilities for extracting method information.

use syn::{
    FnArg, GenericArgument, Ident, ImplItem, ImplItemFn, ItemImpl, Lit, Meta, Pat, PathArguments,
    ReturnType, Type,
};

/// Parsed information about a method
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MethodInfo {
    /// The original method (kept for potential future use)
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
    /// Whether this is Option<T>
    pub is_optional: bool,
    /// Whether this looks like an ID (ends with _id or is named id)
    pub is_id: bool,
}

/// Parsed return type information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ReturnInfo {
    /// The full return type
    pub ty: Option<Type>,
    /// Inner type if Result<T, E>
    pub ok_type: Option<Type>,
    /// Error type if Result<T, E>
    pub err_type: Option<Type>,
    /// Inner type if Option<T>
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
}

impl MethodInfo {
    /// Parse a method from an ImplItemFn
    ///
    /// Returns None for associated functions without `&self` (constructors, etc.)
    pub fn parse(method: &ImplItemFn) -> syn::Result<Option<Self>> {
        let name = method.sig.ident.clone();
        let is_async = method.sig.asyncness.is_some();

        // Skip associated functions without self receiver (constructors, etc.)
        let has_receiver = method.sig.inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_)));
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
fn extract_docs(attrs: &[syn::Attribute]) -> Option<String> {
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

/// Parse function parameters (excluding self)
fn parse_params(inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>) -> syn::Result<Vec<ParamInfo>> {
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
                            "unsupported parameter pattern. Use a simple identifier like `name: String`",
                        ));
                    }
                };

                let ty = (*pat_type.ty).clone();
                let is_optional = is_option_type(&ty);
                let is_id = is_id_param(&name);

                params.push(ParamInfo {
                    name,
                    ty,
                    is_optional,
                    is_id,
                });
            }
        }
    }

    Ok(params)
}

/// Parse return type information
fn parse_return_type(output: &ReturnType) -> ReturnInfo {
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
            }
        }
    }
}

/// Check if a type is Option<T> and extract T
fn extract_option_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Option"
                && let PathArguments::AngleBracketed(args) = &segment.arguments
                    && let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
    None
}

/// Check if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    extract_option_type(ty).is_some()
}

/// Check if a type is Result<T, E> and extract T and E
fn extract_result_types(ty: &Type) -> Option<(Type, Type)> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Result"
                && let PathArguments::AngleBracketed(args) = &segment.arguments {
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
fn extract_stream_item(ty: &Type) -> Option<Type> {
    if let Type::ImplTrait(impl_trait) = ty {
        for bound in &impl_trait.bounds {
            if let syn::TypeParamBound::Trait(trait_bound) = bound
                && let Some(segment) = trait_bound.path.segments.last()
                    && segment.ident == "Stream"
                        && let PathArguments::AngleBracketed(args) = &segment.arguments {
                            for arg in &args.args {
                                if let GenericArgument::AssocType(assoc) = arg
                                    && assoc.ident == "Item" {
                                        return Some(assoc.ty.clone());
                                    }
                            }
                        }
        }
    }
    None
}

/// Check if a type is ()
fn is_unit_type(ty: &Type) -> bool {
    if let Type::Tuple(tuple) = ty {
        return tuple.elems.is_empty();
    }
    false
}

/// Check if a parameter name looks like an ID
fn is_id_param(name: &Ident) -> bool {
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

/// Get the struct name from an impl block
pub fn get_impl_name(impl_block: &ItemImpl) -> syn::Result<Ident> {
    if let Type::Path(type_path) = impl_block.self_ty.as_ref()
        && let Some(segment) = type_path.path.segments.last() {
            return Ok(segment.ident.clone());
        }
    Err(syn::Error::new_spanned(
        &impl_block.self_ty,
        "Expected a simple type name",
    ))
}
