//! Markdown documentation generation.
//!
//! Generates API documentation in Markdown format from impl blocks.

use heck::ToTitleCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, ItemImpl, Token};

use crate::parse::{extract_methods, get_impl_name, MethodInfo, ParamInfo};

/// Arguments for the #[markdown] attribute
#[derive(Default)]
pub struct MarkdownArgs {
    /// Document title
    pub title: Option<String>,
    /// Include parameter types
    pub types: bool,
}

impl Parse for MarkdownArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = MarkdownArgs::default();
        args.types = true; // default to showing types

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

/// Expand the #[markdown] attribute macro
pub fn expand_markdown(args: MarkdownArgs, impl_block: ItemImpl) -> syn::Result<TokenStream> {
    let struct_name = get_impl_name(&impl_block)?;
    let struct_name_str = struct_name.to_string();
    let methods = extract_methods(&impl_block)?;

    let title = args
        .title
        .unwrap_or_else(|| format!("{} API", struct_name_str));
    let show_types = args.types;

    // Generate method documentation
    let method_docs: Vec<String> = methods
        .iter()
        .map(|m| generate_method_doc(m, show_types))
        .collect();

    let markdown = format!(
        r#"# {}

{}

## Methods

{}
"#,
        title,
        generate_overview(&struct_name_str, &methods),
        method_docs.join("\n---\n\n")
    );

    Ok(quote! {
        #impl_block

        impl #struct_name {
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

/// Generate overview section
fn generate_overview(name: &str, methods: &[MethodInfo]) -> String {
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

    overview
}

/// Generate documentation for a single method
fn generate_method_doc(method: &MethodInfo, show_types: bool) -> String {
    let name = method.name.to_string();
    let title = name.replace('_', " ").to_title_case();

    let mut doc = format!("### {}\n\n", title);

    // Description from doc comment
    if let Some(desc) = &method.docs {
        doc.push_str(desc);
        doc.push_str("\n\n");
    }

    // Async badge
    if method.is_async {
        doc.push_str("*async*\n\n");
    }

    // Method signature
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

    // Return type
    if let Some(ty) = &method.return_info.ty {
        if show_types && !method.return_info.is_unit {
            let type_str = quote::quote!(#ty).to_string();
            doc.push_str(&format!(" -> {}", simplify_type(&type_str)));
        }
    }

    doc.push_str("\n```\n\n");

    // Parameters section
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

    // Returns section
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

/// Format a parameter for display
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

/// Simplify a type string for display
fn simplify_type(ty: &str) -> String {
    ty.replace(" < ", "<")
        .replace(" > ", ">")
        .replace(" , ", ", ")
        .replace("& str", "&str")
        .replace(":: ", "::")
}

/// Describe a return type in prose
fn describe_return_type(ty: &str, info: &crate::parse::ReturnInfo) -> String {
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
