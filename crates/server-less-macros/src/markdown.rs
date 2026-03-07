//! Markdown documentation generation macro.
//!
//! Generates API documentation in Markdown format from impl blocks.
//!
//! # Documentation Generation
//!
//! Creates human-readable API docs:
//! - Method descriptions from doc comments
//! - Parameter lists with types
//! - Return type information
//! - Async badge for async methods
//! - Code examples
//!
//! # Format
//!
//! Generates standard Markdown:
//! - Headers for each method
//! - Parameter tables
//! - Code blocks for examples
//! - Links and cross-references
//!
//! # Generated Methods
//!
//! - `markdown_docs() -> String` - Complete Markdown documentation
//!
//! # Example
//!
//! ```ignore
//! use server_less::markdown;
//!
//! struct PaymentService;
//!
//! #[markdown(title = "Payment API")]
//! impl PaymentService {
//!     /// Process a payment
//!     async fn process_payment(&self, amount: f64, currency: String) -> String {
//!         format!("Processed {} {}", amount, currency)
//!     }
//! }
//!
//! let docs = PaymentService::markdown_docs();
//! // Returns formatted Markdown documentation
//! ```

use heck::ToTitleCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use server_less_parse::{
    MethodInfo, ParamInfo, extract_groups, extract_methods, get_impl_name, resolve_method_group,
};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[markdown] attribute
#[derive(Default)]
pub(crate) struct MarkdownArgs {
    pub title: Option<String>,
    pub types: bool,
}

impl Parse for MarkdownArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = MarkdownArgs {
            title: None,
            types: true,
        };

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;

            match ident.to_string().as_str() {
                "title" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitStr = input.parse()?;
                    args.title = Some(lit.value());
                }
                "types" => {
                    input.parse::<Token![=]>()?;
                    let lit: syn::LitBool = input.parse()?;
                    args.types = lit.value();
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown argument `{other}`. Valid arguments: title, types"),
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

pub(crate) fn expand_markdown(
    args: MarkdownArgs,
    impl_block: ItemImpl,
) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let generics_clone = impl_block.generics.clone();
    let (impl_generics, _ty_generics, where_clause) = generics_clone.split_for_impl();
    let self_ty = impl_block.self_ty.clone();
    let methods = extract_methods(&impl_block)?;

    let title = args
        .title
        .unwrap_or_else(|| format!("{} API", struct_name_str));
    let show_types = args.types;

    // Partition methods by group
    let group_registry = extract_groups(&impl_block)?;
    let groups = partition_by_group(&methods, &group_registry)?;

    let mut methods_section = String::new();
    for (group_name, group_methods) in &groups {
        if let Some(name) = group_name {
            methods_section.push_str(&format!("## {}\n\n", name));
        } else {
            methods_section.push_str("## Methods\n\n");
        }
        let docs: Vec<String> = group_methods
            .iter()
            .map(|m| generate_method_doc(m, show_types))
            .collect();
        methods_section.push_str(&docs.join("\n---\n\n"));
    }

    let markdown = format!(
        "# {}\n\n{}\n\n{}",
        title,
        generate_overview(&struct_name_str, &methods, &groups),
        methods_section,
    );

    // Strip #[server(...)] from impl-level attrs (e.g. groups(...))
    let mut clean_impl = impl_block;
    clean_impl
        .attrs
        .retain(|attr| !attr.path().is_ident("server"));

    Ok(quote! {
        #clean_impl

        impl #impl_generics #self_ty #where_clause {
            /// Get the API documentation in Markdown format.
            pub fn markdown_docs() -> &'static str {
                #markdown
            }

            /// Write the API documentation to a file.
            pub fn write_markdown(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
                std::fs::write(path, Self::markdown_docs())
            }
        }
    })
}

/// Partition methods by resolved group name, maintaining declaration/first-seen order.
/// Ungrouped methods appear first with `None` as the key.
fn partition_by_group<'a>(
    methods: &'a [MethodInfo],
    registry: &Option<server_less_parse::GroupRegistry>,
) -> syn::Result<Vec<(Option<String>, Vec<&'a MethodInfo>)>> {
    let mut groups: Vec<(Option<String>, Vec<&MethodInfo>)> = Vec::new();

    for method in methods {
        let group = resolve_method_group(method, registry)?;

        if let Some(pos) = groups.iter().position(|(g, _)| *g == group) {
            groups[pos].1.push(method);
        } else {
            groups.push((group, vec![method]));
        }
    }

    // Move ungrouped (None) to the front if it isn't already
    if let Some(pos) = groups.iter().position(|(g, _)| g.is_none())
        && pos != 0
    {
        let ungrouped = groups.remove(pos);
        groups.insert(0, ungrouped);
    }

    Ok(groups)
}

fn generate_overview(
    name: &str,
    methods: &[MethodInfo],
    groups: &[(Option<String>, Vec<&MethodInfo>)],
) -> String {
    let method_count = methods.len();
    let has_async = methods.iter().any(|m| m.is_async);

    let mut overview = format!(
        "**{}** provides {} method{}.",
        name,
        method_count,
        if method_count == 1 { "" } else { "s" }
    );

    if has_async {
        overview.push_str(" Some methods are async.");
    }

    let named_groups: Vec<&str> = groups
        .iter()
        .filter_map(|(g, _)| g.as_deref())
        .collect();
    if !named_groups.is_empty() {
        overview.push_str(&format!(
            " Organized into {} group{}: {}.",
            named_groups.len(),
            if named_groups.len() == 1 { "" } else { "s" },
            named_groups.join(", "),
        ));
    }

    overview
}

fn generate_method_doc(method: &MethodInfo, show_types: bool) -> String {
    let name = method.name.to_string();
    let title = name.replace('_', " ").to_title_case();

    let mut doc = format!("### {}\n\n", title);

    if let Some(desc) = &method.docs {
        doc.push_str(desc);
        doc.push_str("\n\n");
    }

    if method.is_async {
        doc.push_str("*async*\n\n");
    }

    doc.push_str("```\n");
    doc.push_str(&name);
    doc.push('(');

    let params: Vec<String> = method
        .params
        .iter()
        .map(|p| format_param(p, show_types))
        .collect();
    doc.push_str(&params.join(", "));

    doc.push(')');

    if let Some(ty) = &method.return_info.ty
        && show_types
        && !method.return_info.is_unit
    {
        let type_str = quote::quote!(#ty).to_string();
        doc.push_str(&format!(" -> {}", simplify_type(&type_str)));
    }

    doc.push_str("\n```\n\n");

    if !method.params.is_empty() {
        doc.push_str("**Parameters:**\n\n");
        for param in &method.params {
            doc.push_str(&format!(
                "- `{}`: {}{}\n",
                param.name,
                if show_types {
                    let ty = &param.ty;
                    let ty = quote::quote!(#ty).to_string();
                    format!("*{}* ", simplify_type(&ty))
                } else {
                    String::new()
                },
                if param.is_optional { "(optional)" } else { "" }
            ));
        }
        doc.push('\n');
    }

    if !method.return_info.is_unit {
        doc.push_str("**Returns:** ");
        if let Some(ty) = &method.return_info.ty {
            let type_str = quote::quote!(#ty).to_string();
            doc.push_str(&describe_return_type(&type_str, &method.return_info));
        }
        doc.push_str("\n\n");
    }

    doc
}

fn format_param(param: &ParamInfo, show_types: bool) -> String {
    let name = param.name.to_string();
    if show_types {
        let ty = &param.ty;
        let ty_str = quote::quote!(#ty).to_string();
        format!("{}: {}", name, simplify_type(&ty_str))
    } else {
        name
    }
}

fn simplify_type(ty: &str) -> String {
    ty.replace(" < ", "<")
        .replace(" > ", ">")
        .replace(" , ", ", ")
        .replace("& str", "&str")
        .replace(":: ", "::")
}

fn describe_return_type(ty: &str, info: &server_less_parse::ReturnInfo) -> String {
    if info.is_result {
        "Result (success or error)".to_string()
    } else if info.is_option {
        "Optional value (may be null)".to_string()
    } else if ty.contains("Vec") {
        "Array of values".to_string()
    } else if ty.contains("String") || ty.contains("str") {
        "String".to_string()
    } else if ty.contains("bool") {
        "Boolean".to_string()
    } else if ty.contains("i32") || ty.contains("i64") || ty.contains("u32") || ty.contains("u64") {
        "Integer".to_string()
    } else if ty.contains("f32") || ty.contains("f64") {
        "Number".to_string()
    } else {
        simplify_type(ty)
    }
}
