//! CLI generation macro.
//!
//! Generates clap-based CLI from impl blocks.

use heck::ToKebabCase;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rhizome_trellis_parse::{MethodInfo, ParamInfo, extract_methods, get_impl_name};
use syn::{ItemImpl, Token, parse::Parse};

/// Arguments for the #[cli] attribute
#[derive(Default)]
pub(crate) struct CliArgs {
    pub name: Option<String>,
    pub version: Option<String>,
    pub about: Option<String>,
}

impl Parse for CliArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = CliArgs::default();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "name" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.name = Some(lit.value());
                }
                "version" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.version = Some(lit.value());
                }
                "about" => {
                    let lit: syn::LitStr = input.parse()?;
                    args.about = Some(lit.value());
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!(
                            "unknown argument `{other}`. Valid arguments: name, version, about"
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

pub(crate) fn expand_cli(args: CliArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    let app_name = args
        .name
        .unwrap_or_else(|| struct_name.to_string().to_kebab_case());
    let version = args.version.unwrap_or_else(|| "0.1.0".to_string());
    let about = args.about.unwrap_or_default();

    let subcommands: Vec<_> = methods.iter().map(generate_subcommand).collect();

    let match_arms: Vec<_> = methods
        .iter()
        .map(|m| generate_match_arm(&struct_name, m))
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #impl_block

        impl #struct_name {
            /// Create a clap Command for this CLI
            pub fn cli_command() -> ::clap::Command {
                ::clap::Command::new(#app_name)
                    .version(#version)
                    .about(#about)
                    #(.subcommand(#subcommands))*
            }

            /// Run the CLI application
            pub fn cli_run(&self) -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
                let matches = Self::cli_command().get_matches();

                match matches.subcommand() {
                    #(#match_arms)*
                    _ => {
                        Self::cli_command().print_help()?;
                        Ok(())
                    }
                }
            }

            /// Run the CLI with custom arguments
            pub fn cli_run_with<I, T>(&self, args: I) -> ::std::result::Result<(), Box<dyn ::std::error::Error>>
            where
                I: IntoIterator<Item = T>,
                T: Into<::std::ffi::OsString> + Clone,
            {
                let matches = Self::cli_command().get_matches_from(args);

                match matches.subcommand() {
                    #(#match_arms)*
                    _ => {
                        Self::cli_command().print_help()?;
                        Ok(())
                    }
                }
            }
        }
    })
}

fn generate_subcommand(method: &MethodInfo) -> TokenStream2 {
    let name = method.name.to_string().to_kebab_case();
    let about = method.docs.clone().unwrap_or_default();

    let args: Vec<_> = method.params.iter().map(generate_arg).collect();

    quote! {
        ::clap::Command::new(#name)
            .about(#about)
            #(.arg(#args))*
    }
}

fn generate_arg(param: &ParamInfo) -> TokenStream2 {
    let name = param.name.to_string().to_kebab_case();
    let is_optional = param.is_optional;

    if param.is_id {
        let required = !is_optional;
        quote! {
            ::clap::Arg::new(#name)
                .required(#required)
                .index(1)
                .help(concat!("The ", #name))
        }
    } else if is_optional {
        quote! {
            ::clap::Arg::new(#name)
                .long(#name)
                .required(false)
                .help(concat!("Optional: ", #name))
        }
    } else {
        quote! {
            ::clap::Arg::new(#name)
                .long(#name)
                .required(true)
                .help(concat!("Required: ", #name))
        }
    }
}

fn generate_match_arm(_struct_name: &syn::Ident, method: &MethodInfo) -> syn::Result<TokenStream2> {
    let subcommand_name = method.name.to_string().to_kebab_case();
    let method_name = &method.name;

    let arg_extractions: Vec<_> = method
        .params
        .iter()
        .map(|p| {
            let name = &p.name;
            let name_str = p.name.to_string().to_kebab_case();
            let ty = &p.ty;

            if p.is_optional {
                quote! {
                    let #name: #ty = sub_matches
                        .get_one::<String>(#name_str)
                        .and_then(|s| s.parse().ok());
                }
            } else {
                quote! {
                    let #name: #ty = sub_matches
                        .get_one::<String>(#name_str)
                        .map(|s| s.parse())
                        .transpose()?
                        .ok_or_else(|| format!("Missing required argument: {}", #name_str))?;
                }
            }
        })
        .collect();

    let arg_names: Vec<_> = method.params.iter().map(|p| &p.name).collect();

    let call = if method.return_info.is_unit {
        if method.is_async {
            quote! {
                ::tokio::runtime::Runtime::new()
                    .expect("Failed to create Tokio runtime")
                    .block_on(self.#method_name(#(#arg_names),*));
            }
        } else {
            quote! {
                self.#method_name(#(#arg_names),*);
            }
        }
    } else if method.is_async {
        quote! {
            let result = ::tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime")
                .block_on(self.#method_name(#(#arg_names),*));
        }
    } else {
        quote! {
            let result = self.#method_name(#(#arg_names),*);
        }
    };

    let output = if method.return_info.is_unit {
        quote! { println!("Done"); }
    } else if method.return_info.is_result {
        quote! {
            match result {
                Ok(value) => {
                    let json = ::rhizome_trellis::serde_json::to_string_pretty(&value)?;
                    println!("{}", json);
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    ::std::process::exit(1);
                }
            }
        }
    } else if method.return_info.is_option {
        quote! {
            match result {
                Some(value) => {
                    let json = ::rhizome_trellis::serde_json::to_string_pretty(&value)?;
                    println!("{}", json);
                }
                None => {
                    eprintln!("Not found");
                    ::std::process::exit(1);
                }
            }
        }
    } else {
        quote! {
            let json = ::rhizome_trellis::serde_json::to_string_pretty(&result)?;
            println!("{}", json);
        }
    };

    Ok(quote! {
        Some((#subcommand_name, sub_matches)) => {
            #(#arg_extractions)*
            #call
            #output
            Ok(())
        }
    })
}
