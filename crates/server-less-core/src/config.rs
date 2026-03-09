//! Config management types for `#[derive(Config)]`.
//!
//! Provides the [`Config`] trait, [`ConfigSource`] enum, [`ConfigError`], and
//! [`ConfigFieldMeta`] for runtime introspection.  The proc macro generates
//! implementations of `Config` for user-defined structs; this module supplies
//! the interface those implementations depend on.
//!
//! # Source precedence
//!
//! Sources are applied in the order they appear in the slice passed to
//! [`Config::load`].  Later sources win over earlier ones.  The conventional
//! order is: defaults first, then file, then environment, then CLI overrides
//! (highest priority).  `#[derive(Config)]` uses the same order when it
//! calls `load` internally.
//!
//! # Example
//!
//! ```rust,ignore
//! use server_less_core::config::{Config, ConfigSource};
//!
//! #[derive(Config)]
//! struct AppConfig {
//!     #[param(default = "localhost")]
//!     host: String,
//!     #[param(default = 8080)]
//!     port: u16,
//!     #[param(env = "DATABASE_URL")]
//!     database_url: String,
//! }
//!
//! let cfg = AppConfig::load(&[
//!     ConfigSource::Defaults,
//!     ConfigSource::File("app.toml".into()),
//!     ConfigSource::Env { prefix: Some("APP".into()) },
//! ])?;
//! ```

use std::path::PathBuf;

/// Specifies where configuration values should be loaded from.
///
/// Sources are applied in order.  By default, later sources overwrite earlier
/// values ([`File`](ConfigSource::File), [`Env`](ConfigSource::Env)).
/// [`MergeFile`](ConfigSource::MergeFile) is an exception: it only fills in
/// fields that haven't been set by a prior source.
///
/// ## Conventional ordering
///
/// ```rust,ignore
/// &[
///     ConfigSource::Defaults,              // lowest priority
///     ConfigSource::File(global_config),   // e.g. ~/.config/app/config.toml
///     ConfigSource::MergeFile(local_config), // e.g. .app/config.toml
///     ConfigSource::Env { prefix: Some("APP".into()) }, // highest priority
/// ]
/// ```
///
/// With this ordering, the local config file *supplements* the global one
/// (missing local fields fall back to global values) while env vars always win.
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Apply compile-time defaults declared with `#[param(default = ...)]`.
    ///
    /// Should almost always be the first source so that explicit values from
    /// files or environment variables can override them.
    Defaults,

    /// Read a TOML file at the given path.
    ///
    /// Later sources — including other `File` entries — overwrite values set
    /// here.  Use [`MergeFile`](ConfigSource::MergeFile) when you want a file
    /// to supplement rather than replace earlier values.
    ///
    /// Missing files are silently skipped (not an error) so that optional
    /// config files work without extra boilerplate.
    File(PathBuf),

    /// Like [`File`](ConfigSource::File), but only fills in fields that are
    /// still unset after all previous sources.
    ///
    /// Use this for layered config (global → project → env): a project-local
    /// file should add or override specific keys, not erase values inherited
    /// from the global config.
    ///
    /// ```rust,ignore
    /// &[
    ///     ConfigSource::Defaults,
    ///     ConfigSource::File(global),    // global config: sets host, port, …
    ///     ConfigSource::MergeFile(local), // local config: only overrides what it explicitly sets
    ///     ConfigSource::Env { prefix: Some("APP".into()) },
    /// ]
    /// ```
    ///
    /// Missing files are silently skipped, same as [`File`](ConfigSource::File).
    MergeFile(PathBuf),

    /// Read environment variables.
    ///
    /// Each field is looked up under `{PREFIX}_{FIELD_UPPER}` (e.g. prefix
    /// `"APP"`, field `host` → `APP_HOST`).  An explicit `#[param(env = "X")]`
    /// overrides the generated name entirely.
    ///
    /// `prefix` is uppercased automatically; `None` means no prefix.
    Env { prefix: Option<String> },
}

/// Error returned by [`Config::load`].
#[derive(Debug)]
pub enum ConfigError {
    /// A required field has no value from any source.
    MissingField { field: &'static str },

    /// A value was present but could not be parsed to the field's type.
    ParseError {
        field: &'static str,
        source: String,
        message: String,
    },

    /// I/O error reading a config file.
    Io(std::io::Error),

    /// TOML parse error in a config file.
    Format { path: PathBuf, message: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingField { field } => {
                write!(f, "missing required config field `{field}`")
            }
            ConfigError::ParseError {
                field,
                source,
                message,
            } => write!(
                f,
                "failed to parse config field `{field}` from {source}: {message}"
            ),
            ConfigError::Io(e) => write!(f, "config I/O error: {e}"),
            ConfigError::Format { path, message } => {
                write!(f, "config format error in {}: {}", path.display(), message)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

/// Metadata about a single field in a [`Config`]-implementing struct.
///
/// Returned by [`Config::field_meta`] for runtime introspection — e.g. to
/// generate a config file template or validate a user-supplied file.
#[derive(Debug, Clone)]
pub struct ConfigFieldMeta {
    /// Field name as it appears in the struct.
    pub name: &'static str,
    /// Rust type name (best-effort string representation).
    pub type_name: &'static str,
    /// Environment variable override from `#[param(env = "VAR")]`.
    ///
    /// If `None`, the macro generates `{PREFIX}_{FIELD_UPPER}` at load time.
    pub env_var: Option<&'static str>,
    /// Dotted config-file key override from `#[param(file_key = "a.b.c")]`.
    ///
    /// If `None`, the field name is used directly as the TOML key.
    pub file_key: Option<&'static str>,
    /// Default value as a display string (from `#[param(default = ...)]`).
    pub default: Option<&'static str>,
    /// Help text from `#[param(help = "...")]`.
    pub help: Option<&'static str>,
    /// Whether the field is required (no default, not `Option<T>`).
    pub required: bool,
}

/// Trait implemented by `#[derive(Config)]` structs.
///
/// Provides config loading from multiple sources with a defined precedence order,
/// and field introspection for tooling.
pub trait Config: Sized {
    /// Load this config by applying `sources` in order (later sources win).
    ///
    /// The conventional source slice is:
    /// ```rust,ignore
    /// &[
    ///     ConfigSource::Defaults,
    ///     ConfigSource::File(path),
    ///     ConfigSource::Env { prefix: Some("APP".into()) },
    /// ]
    /// ```
    fn load(sources: &[ConfigSource]) -> Result<Self, ConfigError>;

    /// Return metadata for all fields in this config struct.
    ///
    /// Useful for generating config file templates, shell completions,
    /// or documentation.
    fn field_meta() -> &'static [ConfigFieldMeta];
}

/// Read a TOML file and return its top-level keys as a flat string map.
///
/// Nested tables are flattened with dot-separated keys (`database.host`).
/// Used by `#[derive(Config)]`-generated `load` implementations.
///
/// Returns `Ok(None)` if the file does not exist (not an error).
#[cfg(feature = "config")]
pub fn load_toml_file(
    path: &std::path::Path,
) -> Result<Option<std::collections::HashMap<String, String>>, ConfigError> {
    use std::io::ErrorKind;

    let contents = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ConfigError::Io(e)),
    };

    let value: toml::Value = contents.parse().map_err(|e: toml::de::Error| {
        ConfigError::Format {
            path: path.to_owned(),
            message: e.to_string(),
        }
    })?;

    let mut map = std::collections::HashMap::new();
    flatten_toml("", &value, &mut map);
    Ok(Some(map))
}

#[cfg(feature = "config")]
fn flatten_toml(
    prefix: &str,
    value: &toml::Value,
    out: &mut std::collections::HashMap<String, String>,
) {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_toml(&key, v, out);
            }
        }
        other => {
            out.insert(prefix.to_owned(), other.to_string());
        }
    }
}
