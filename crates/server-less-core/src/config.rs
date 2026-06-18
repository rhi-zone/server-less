//! Config management types for `#[derive(Config)]`.
//!
//! Provides the [`ConfigLoad`] trait, [`ConfigSource`] enum, [`ConfigError`], and
//! [`ConfigFieldMeta`] for runtime introspection.  The proc macro generates
//! implementations of `ConfigLoad` for user-defined structs; this module supplies
//! the interface those implementations depend on.
//!
//! # Source precedence
//!
//! Sources are applied in the order they appear in the slice passed to
//! [`ConfigLoad::load`].  Later sources win over earlier ones.  The conventional
//! order is: defaults first, then file, then environment, then CLI overrides
//! (highest priority).  `#[derive(Config)]` uses the same order when it
//! calls `load` internally.
//!
//! # Example
//!
//! ```rust,ignore
//! use server_less_core::config::{ConfigLoad, ConfigSource};
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
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`: new source kinds may be added in minor
/// releases.  Match on it with a trailing wildcard arm (`_ => ...`) in
/// downstream code so that adding a variant is not a breaking change.
#[derive(Debug, Clone)]
#[non_exhaustive]
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

    /// An already-parsed TOML table passed to a nested `ConfigLoad::load` call.
    ///
    /// This variant is for internal use by `#[derive(Config)]`-generated code
    /// when loading `#[param(nested)]` sub-structs.  It is **not** part of the
    /// public API and should not be constructed directly by callers.
    ///
    /// The payload is the opaque [`NestedTomlTable`] newtype so that `toml`'s
    /// types do not leak into server-less's public API surface.
    #[doc(hidden)]
    #[cfg(feature = "config")]
    TomlTable(NestedTomlTable),

    /// Like [`TomlTable`](ConfigSource::TomlTable) but with merge semantics
    /// (only fills in fields still unset by prior sources).
    ///
    /// Internal use only — generated by `#[derive(Config)]` for `MergeFile`
    /// sources applied to `#[param(nested)]` fields.
    #[doc(hidden)]
    #[cfg(feature = "config")]
    MergeTomlTable(NestedTomlTable),
}

/// Opaque wrapper around an already-parsed TOML sub-table.
///
/// This newtype exists purely to keep `toml`'s types out of server-less's
/// public API: the [`ConfigSource::TomlTable`] / [`ConfigSource::MergeTomlTable`]
/// variants carry a `NestedTomlTable` rather than a `toml::Value`, so bumping
/// `toml`'s major version is not a breaking change for downstream crates.
///
/// It is constructed and consumed only by `#[derive(Config)]`-generated code
/// via the inherent methods below; the inner `toml::Value` is private.
#[doc(hidden)]
#[cfg(feature = "config")]
#[derive(Debug, Clone)]
pub struct NestedTomlTable(toml::Value);

#[cfg(feature = "config")]
impl NestedTomlTable {
    /// Wrap an already-parsed [`toml::Value`].
    ///
    /// Used by `load_toml_file_raw` and by generated code that extracts a
    /// sub-table from a parent table.
    pub fn from_value(value: toml::Value) -> Self {
        NestedTomlTable(value)
    }

    /// Look up a sub-table by key, returning a new `NestedTomlTable`.
    ///
    /// Returns `None` if `self` is not a table or has no entry for `key`.
    pub fn get(&self, key: &str) -> Option<NestedTomlTable> {
        self.0.get(key).cloned().map(NestedTomlTable)
    }

    /// Deserialize the wrapped table into a concrete type `T`.
    ///
    /// The error type is `toml`'s deserialization error; generated code maps it
    /// into [`ConfigError::ParseError`].
    pub fn deserialize<T: serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T, toml::de::Error> {
        self.0.clone().try_into()
    }

    /// Flatten the wrapped table into a dot-separated `HashMap<String, String>`.
    ///
    /// Mirrors the flattening logic of [`load_toml_file`] while keeping the
    /// inner `toml::Value` encapsulated.
    pub fn flatten_into(
        &self,
        out: &mut std::collections::HashMap<String, String>,
    ) {
        flatten_toml("", &self.0, out);
    }
}

/// Error returned by [`ConfigLoad::load`].
///
/// # Stability
///
/// This enum is `#[non_exhaustive]`: new error variants may be added in minor
/// releases.  Match on it with a trailing wildcard arm (`_ => ...`) in
/// downstream code so that adding a variant is not a breaking change.
#[derive(Debug)]
#[non_exhaustive]
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

/// Metadata about a single field in a [`ConfigLoad`]-implementing struct.
///
/// Returned by [`ConfigLoad::field_meta`] for runtime introspection — e.g. to
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
    /// For `#[param(nested)]` fields: metadata of the child struct's fields.
    ///
    /// `Some` when the field is a nested `ConfigLoad` sub-struct; `None` for
    /// leaf (scalar/`Option`) fields.
    pub nested: Option<&'static [ConfigFieldMeta]>,
    /// Env-var prefix override for nested fields (from `#[param(env_prefix = "SEARCH")]`).
    ///
    /// When `Some`, child env vars use `{env_prefix}_{CHILD_FIELD}` instead of
    /// the auto-generated `{parent_prefix}_{field_name}_{CHILD_FIELD}`.
    /// Only meaningful when `nested` is `Some`.
    pub env_prefix: Option<&'static str>,
}

/// Trait implemented by `#[derive(Config)]` structs.
///
/// Provides config loading from multiple sources with a defined precedence order,
/// and field introspection for tooling.
///
/// Named `ConfigLoad` to avoid clashing with the `Config` derive macro name.
pub trait ConfigLoad: Sized {
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

/// Read a TOML file and return the parsed table wrapped in [`NestedTomlTable`]
/// (without flattening).
///
/// Used internally by `#[derive(Config)]`-generated code to extract sub-tables
/// for `#[param(nested)]` fields.  The result is opaque so `toml`'s types do
/// not leak into the public API.
///
/// Returns `Ok(None)` if the file does not exist (not an error).
#[cfg(feature = "config")]
pub fn load_toml_file_raw(
    path: &std::path::Path,
) -> Result<Option<NestedTomlTable>, ConfigError> {
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

    Ok(Some(NestedTomlTable::from_value(value)))
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
        // toml::Value::String: use the inner string directly — calling .to_string()
        // on a toml::Value::String wraps it in TOML quotes, which breaks FromStr parsing.
        toml::Value::String(s) => {
            out.insert(prefix.to_owned(), s.clone());
        }
        other => {
            out.insert(prefix.to_owned(), other.to_string());
        }
    }
}
