//! Generate config subcommand methods for `#[program(config = MyConfig)]`.
//!
//! Generates the `config show`, `config schema`, `config validate`, and
//! `config set` subcommands as methods on the server/program struct.
//!
//! # Limitations (MVP)
//!
//! `config set` round-trips through a flat string map (via `load_toml_file`), so
//! comments and nested table structure are not preserved. Use `toml_edit` for
//! comment-preserving edits — tracked in TODO.md.

use heck::ToShoutySnakeCase;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Type;

/// Generate all config subcommand wiring.
///
/// Returns:
/// - `methods`: `impl` block with `config_subcommand()`, `config_run_subcommand()`, helpers
/// - `subcommand_addition`: `.subcommand(Self::config_subcommand())` for `cli_command()`
/// - `dispatch_arm`: the match arm for `cli_dispatch()`
pub fn generate_all(
    self_ty: &Type,
    config_ty: &syn::Path,
    cmd_name: &str,
    app_name: &str,
) -> (TokenStream2, TokenStream2, TokenStream2) {
    let env_prefix = app_name.to_shouty_snake_case();
    let default_config_file = format!("{app_name}.toml");

    let methods = generate_methods(self_ty, config_ty, cmd_name, &env_prefix, &default_config_file);

    let subcommand_addition = quote! {
        .subcommand(Self::config_subcommand())
    };

    let dispatch_arm = quote! {
        ::std::option::Option::Some((#cmd_name, __config_matches)) => {
            Self::config_run_subcommand(__config_matches)
        }
    };

    (methods, subcommand_addition, dispatch_arm)
}

fn generate_methods(
    self_ty: &Type,
    config_ty: &syn::Path,
    cmd_name: &str,
    env_prefix: &str,
    default_config_file: &str,
) -> TokenStream2 {
    quote! {
        impl #self_ty {
            /// Build the `config` management subcommand tree.
            ///
            /// Subcommands: `show`, `schema`, `validate`, `set`.
            pub fn config_subcommand() -> ::server_less::clap::Command {
                ::server_less::clap::Command::new(#cmd_name)
                    .about("Manage configuration")
                    .subcommand(
                        ::server_less::clap::Command::new("show")
                            .about("Show current configuration values and their sources")
                            .arg(
                                ::server_less::clap::Arg::new("section")
                                    .long("section")
                                    .help("Show only fields matching a dotted-path prefix")
                            )
                            .arg(
                                ::server_less::clap::Arg::new("config-file")
                                    .long("config")
                                    .help("Path to config file")
                            )
                    )
                    .subcommand(
                        ::server_less::clap::Command::new("schema")
                            .about("Print JSON Schema for this configuration")
                    )
                    .subcommand(
                        ::server_less::clap::Command::new("validate")
                            .about("Validate current configuration (all sources merged)")
                            .arg(
                                ::server_less::clap::Arg::new("config-file")
                                    .long("config")
                                    .help("Path to config file")
                            )
                    )
                    .subcommand(
                        ::server_less::clap::Command::new("set")
                            .about("Set a configuration value in the config file")
                            .long_about(
                                "Set a configuration value in the config file.\n\n\
                                 Values are auto-typed: `true`/`false` → bool, integers → integer, \
                                 floats → float, anything else → string.\n\n\
                                 Note: comments are not preserved when rewriting the file."
                            )
                            .arg(
                                ::server_less::clap::Arg::new("key")
                                    .required(true)
                                    .help("Config key (field name)")
                            )
                            .arg(
                                ::server_less::clap::Arg::new("value")
                                    .required(true)
                                    .help("New value")
                            )
                            .arg(
                                ::server_less::clap::Arg::new("dry-run")
                                    .long("dry-run")
                                    .action(::server_less::clap::ArgAction::SetTrue)
                                    .help("Preview the change without writing to the file")
                            )
                            .arg(
                                ::server_less::clap::Arg::new("config-file")
                                    .long("config")
                                    .help("Path to config file")
                            )
                    )
            }

            /// Dispatch a matched `config` subcommand to the appropriate handler.
            pub fn config_run_subcommand(
                matches: &::server_less::clap::ArgMatches,
            ) -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                match matches.subcommand() {
                    ::std::option::Option::Some(("show", sub_m)) => {
                        let section = sub_m.get_one::<::std::string::String>("section").map(|s| s.as_str());
                        let path = resolve_config_path(sub_m, #default_config_file);
                        Self::_config_show(section, &path)
                    }
                    ::std::option::Option::Some(("schema", _)) => {
                        Self::_config_schema()
                    }
                    ::std::option::Option::Some(("validate", sub_m)) => {
                        let path = resolve_config_path(sub_m, #default_config_file);
                        Self::_config_validate(&path)
                    }
                    ::std::option::Option::Some(("set", sub_m)) => {
                        let key = sub_m.get_one::<::std::string::String>("key").unwrap();
                        let value = sub_m.get_one::<::std::string::String>("value").unwrap();
                        let dry_run = sub_m.get_flag("dry-run");
                        let path = resolve_config_path(sub_m, #default_config_file);
                        Self::_config_set(key, value, dry_run, &path)
                    }
                    // No subcommand → show all
                    _ => {
                        let path = ::std::path::PathBuf::from(#default_config_file);
                        Self::_config_show(::std::option::Option::None, &path)
                    }
                }
            }

            fn _config_show(
                section: ::std::option::Option<&str>,
                config_file: &::std::path::Path,
            ) -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                use ::server_less_core::config::{Config, ConfigSource};

                // Best-effort load for "current value" display.
                let sources = [
                    ConfigSource::Defaults,
                    ConfigSource::File(config_file.to_path_buf()),
                    ConfigSource::Env { prefix: ::std::option::Option::Some(#env_prefix.to_string()) },
                ];
                let _loaded = <#config_ty as Config>::load(&sources).ok();

                let meta = <#config_ty as Config>::field_meta();
                for field in meta {
                    let key = field.file_key.unwrap_or(field.name);
                    if let ::std::option::Option::Some(s) = section {
                        if !key.starts_with(s) {
                            continue;
                        }
                    }

                    if let ::std::option::Option::Some(help) = field.help {
                        println!("# {help}");
                    }
                    println!("# type: {}", field.type_name);

                    let env_var = field.env_var.map(::std::string::String::from).unwrap_or_else(|| {
                        format!("{}_{}", #env_prefix, field.name.to_uppercase())
                    });

                    if let ::std::option::Option::Some(def) = field.default {
                        println!("{key} = {def}  # default (override: {})", env_var);
                    } else if field.required {
                        println!("# {key} = (required — set via {} or config file)", env_var);
                    } else {
                        println!("# {key} = (optional — set via {} or config file)", env_var);
                    }
                    println!();
                }
                ::std::result::Result::Ok(())
            }

            fn _config_schema() -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                use ::server_less_core::config::Config;

                let meta = <#config_ty as Config>::field_meta();
                let mut properties = ::server_less::serde_json::Map::new();
                let mut required_fields = ::std::vec::Vec::<::server_less::serde_json::Value>::new();

                for field in meta {
                    let json_type = type_name_to_json_type(field.type_name);
                    let mut prop = ::server_less::serde_json::Map::new();
                    prop.insert("type".into(), ::server_less::serde_json::Value::String(json_type.into()));
                    if let ::std::option::Option::Some(h) = field.help {
                        prop.insert("description".into(), ::server_less::serde_json::Value::String(h.into()));
                    }
                    if let ::std::option::Option::Some(d) = field.default {
                        prop.insert("default".into(), ::server_less::serde_json::Value::String(d.into()));
                    }
                    if let ::std::option::Option::Some(var) = field.env_var {
                        prop.insert("x-env-var".into(), ::server_less::serde_json::Value::String(var.into()));
                    }
                    properties.insert(
                        field.name.into(),
                        ::server_less::serde_json::Value::Object(prop),
                    );
                    if field.required {
                        required_fields.push(::server_less::serde_json::Value::String(field.name.into()));
                    }
                }

                let schema = ::server_less::serde_json::json!({
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "type": "object",
                    "properties": properties,
                    "required": required_fields,
                });
                println!("{}", ::server_less::serde_json::to_string_pretty(&schema)?);
                ::std::result::Result::Ok(())
            }

            fn _config_validate(
                config_file: &::std::path::Path,
            ) -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                use ::server_less_core::config::{Config, ConfigSource};

                let sources = [
                    ConfigSource::Defaults,
                    ConfigSource::File(config_file.to_path_buf()),
                    ConfigSource::Env { prefix: ::std::option::Option::Some(#env_prefix.to_string()) },
                ];
                match <#config_ty as Config>::load(&sources) {
                    ::std::result::Result::Ok(_) => {
                        println!("Config valid");
                        ::std::result::Result::Ok(())
                    }
                    ::std::result::Result::Err(e) => {
                        eprintln!("Config invalid: {e}");
                        ::std::result::Result::Err(e.into())
                    }
                }
            }

            fn _config_set(
                key: &str,
                value: &str,
                dry_run: bool,
                config_file: &::std::path::Path,
            ) -> ::std::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error>> {
                // Load existing flat key map (None if file not found — start empty).
                let mut map = ::server_less_core::config::load_toml_file(config_file)?
                    .unwrap_or_default();

                let old_value = map.get(key).cloned().unwrap_or_else(|| "(unset)".into());

                if dry_run {
                    println!("Would set {key}: {} → {value}", old_value);
                    return ::std::result::Result::Ok(());
                }

                map.insert(key.to_string(), value.to_string());

                // Reconstruct as flat TOML (comments not preserved).
                let mut out = ::std::string::String::new();
                let mut keys: ::std::vec::Vec<&::std::string::String> = map.keys().collect();
                keys.sort();
                for k in keys {
                    let v = &map[k];
                    let toml_val = to_toml_value_str(v);
                    out.push_str(&format!("{k} = {toml_val}\n"));
                }
                ::std::fs::write(config_file, out)?;
                println!("Set {key}: {} → {value}", old_value);
                ::std::result::Result::Ok(())
            }
        }

        // Free functions emitted alongside the impl — not public API.

        fn resolve_config_path(
            matches: &::server_less::clap::ArgMatches,
            default: &str,
        ) -> ::std::path::PathBuf {
            matches
                .get_one::<::std::string::String>("config-file")
                .map(::std::path::PathBuf::from)
                .unwrap_or_else(|| ::std::path::PathBuf::from(default))
        }

        /// Map a Rust type name string to a JSON Schema primitive type.
        fn type_name_to_json_type(ty: &str) -> &'static str {
            if ty.contains("bool") {
                "boolean"
            } else if ty.contains("u8") || ty.contains("u16") || ty.contains("u32")
                || ty.contains("u64") || ty.contains("i8") || ty.contains("i16")
                || ty.contains("i32") || ty.contains("i64") || ty.contains("usize")
                || ty.contains("isize")
            {
                "integer"
            } else if ty.contains("f32") || ty.contains("f64") {
                "number"
            } else {
                "string"
            }
        }

        /// Format a string value as a TOML value literal (bool/int/float pass through; strings quoted).
        fn to_toml_value_str(v: &str) -> ::std::string::String {
            if v == "true" || v == "false" {
                return v.to_string();
            }
            if v.parse::<i64>().is_ok() {
                return v.to_string();
            }
            if v.parse::<f64>().is_ok() {
                return v.to_string();
            }
            format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
        }
    }
}
