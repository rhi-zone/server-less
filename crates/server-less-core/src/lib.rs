//! Core traits and types for server-less.
//!
//! This crate provides the foundational types that server-less macros generate code against.

pub mod error;
pub mod extract;
#[cfg(feature = "config")]
pub mod config;

/// Re-export of `toml` for use by `#[derive(Config)]`-generated code.
///
/// Generated serde-nested deserialization code references `toml::de::Error`
/// via this path so callers don't need to add `toml` to their own dependencies.
#[cfg(feature = "config")]
#[doc(hidden)]
pub use toml as __toml;

pub use error::{
    ErrorCode, ErrorResponse, HttpStatusFallback, HttpStatusHelper, IntoErrorCode,
    SchemaValidationError,
};
pub use extract::Context;

#[cfg(feature = "ws")]
pub use extract::WsSender;

/// Trait for types that can be mounted as CLI subcommand groups.
///
/// Implemented automatically by `#[cli]` on an impl block. Allows nested
/// composition: a parent CLI can mount a child's commands as a subcommand group.
#[cfg(feature = "cli")]
pub trait CliSubcommand {
    /// Build the clap Command tree for this type's subcommands.
    fn cli_command() -> ::clap::Command;

    /// Dispatch a matched subcommand to the appropriate method.
    fn cli_dispatch(&self, matches: &::clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>>;

    /// Dispatch a matched subcommand asynchronously.
    ///
    /// Awaits the dispatched method directly without creating an internal runtime.
    /// Used by `cli_run_async` to support user-provided async runtimes.
    fn cli_dispatch_async<'a>(
        &'a self,
        matches: &'a ::clap::ArgMatches,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a {
        async move { self.cli_dispatch(matches) }
    }
}

/// Trait for types that can be mounted as MCP tool namespaces.
///
/// Implemented automatically by `#[mcp]` on an impl block. Allows nested
/// composition: a parent MCP server can mount a child's tools with a name prefix.
#[cfg(feature = "mcp")]
pub trait McpNamespace {
    /// Get tool definitions for this namespace.
    fn mcp_namespace_tools() -> Vec<serde_json::Value>;

    /// Get tool names for this namespace (without prefix).
    fn mcp_namespace_tool_names() -> Vec<String>;

    /// Call a tool by name (sync). Returns error for async-only methods.
    fn mcp_namespace_call(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String>;

    /// Call a tool by name (async).
    fn mcp_namespace_call_async(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
}

/// Trait for types that can be mounted as JSON-RPC method namespaces.
///
/// Implemented automatically by `#[jsonrpc]` on an impl block. Allows nested
/// composition: a parent JSON-RPC server can mount a child's methods with a dot-separated prefix.
#[cfg(feature = "jsonrpc")]
pub trait JsonRpcMount {
    /// Get method names for this mount (without prefix).
    fn jsonrpc_mount_methods() -> Vec<String>;

    /// Dispatch a method call (sync). Returns error for async-only methods.
    fn jsonrpc_mount_dispatch(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String>;

    /// Dispatch a method call (async).
    fn jsonrpc_mount_dispatch_async(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
}

/// Trait for types that can be mounted as WebSocket method namespaces.
///
/// Implemented automatically by `#[ws]` on an impl block. Allows nested
/// composition: a parent WebSocket server can mount a child's methods with a dot-separated prefix.
#[cfg(feature = "ws")]
pub trait WsMount {
    /// Get method names for this mount (without prefix).
    fn ws_mount_methods() -> Vec<String>;

    /// Dispatch a method call (sync). Returns error for async-only methods.
    fn ws_mount_dispatch(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String>;

    /// Dispatch a method call (async).
    fn ws_mount_dispatch_async(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
}

/// Trait for types that can be mounted as HTTP route groups.
///
/// Implemented automatically by `#[http]` on an impl block. Allows nested
/// composition: a parent HTTP server can mount a child's routes under a path prefix.
#[cfg(feature = "http")]
pub trait HttpMount: Send + Sync + 'static {
    /// Build an axum Router for this mount's routes.
    fn http_mount_router(self: ::std::sync::Arc<Self>) -> ::axum::Router;

    /// Get full OpenAPI path definitions for this mount (including any nested mounts).
    ///
    /// Paths are relative to the mount point. The parent prefixes them when composing.
    fn http_mount_openapi_paths() -> Vec<server_less_openapi::OpenApiPath>
    where
        Self: Sized;
}

/// Format a `serde_json::Value` according to the active JSON output flag.
///
/// This function is only called when at least one JSON flag is active
/// (`--json`, `--jsonl`, or `--jq`). The overall CLI default output path
/// (human-readable `Display` text) is generated directly by the `#[cli]`
/// macro and does not go through this function.
///
/// Flag precedence (first match wins):
///
/// - `jq`: filter the value using jaq (jq implemented in Rust, no external binary needed)
/// - `jsonl`: one compact JSON object per line (arrays are unwrapped; non-arrays emit a single line)
/// - `json` (`true`): compact JSON — `serde_json::to_string`, no whitespace
/// - `json` (`false`) with no other flag active: pretty-printed JSON — only reachable when called
///   directly, not from `#[cli]`-generated code (which always sets at least one flag)
#[cfg(feature = "cli")]
pub fn cli_format_output(
    value: serde_json::Value,
    jsonl: bool,
    json: bool,
    jq: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(filter) = jq {
        use jaq_core::load::{Arena, File as JaqFile, Loader};
        use jaq_core::{Compiler, Ctx, Vars, data, unwrap_valr};
        use jaq_json::Val;

        let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
        let arena = Arena::default();

        let program = JaqFile {
            code: filter,
            path: (),
        };

        let modules = loader
            .load(&arena, program)
            .map_err(|errs| format!("jq parse error: {:?}", errs))?;

        let filter_compiled = Compiler::default()
            .with_funs(jaq_std::funs().chain(jaq_json::funs()))
            .compile(modules)
            .map_err(|errs| format!("jq compile error: {:?}", errs))?;

        let val: Val = serde_json::from_value(value)?;
        let ctx = Ctx::<data::JustLut<Val>>::new(&filter_compiled.lut, Vars::new([]));
        let out = filter_compiled.id.run((ctx, val)).map(unwrap_valr);

        let mut results = Vec::new();
        for result in out {
            match result {
                Ok(v) => results.push(v.to_string()),
                Err(e) => return Err(format!("jq runtime error: {:?}", e).into()),
            }
        }

        Ok(results.join("\n"))
    } else if jsonl {
        match value {
            serde_json::Value::Array(items) => {
                let lines: Vec<String> = items
                    .iter()
                    .map(serde_json::to_string)
                    .collect::<Result<_, _>>()?;
                Ok(lines.join("\n"))
            }
            other => Ok(serde_json::to_string(&other)?),
        }
    } else if json {
        Ok(serde_json::to_string(&value)?)
    } else {
        Ok(serde_json::to_string_pretty(&value)?)
    }
}

/// Generate a JSON Schema for a type at runtime using schemars.
///
/// Called by `--output-schema` in `#[cli]`-generated code when the `jsonschema`
/// feature is enabled. Users must `#[derive(schemars::JsonSchema)]` on their
/// return types to use this.
#[cfg(feature = "jsonschema")]
pub fn cli_schema_for<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T))
        .unwrap_or_else(|_| serde_json::json!({"type": "object"}))
}

/// A clap [`TypedValueParser`] that uses [`schemars::JsonSchema`] to surface
/// enum variants as possible values, and [`std::str::FromStr`] for actual parsing.
///
/// When `T` is an enum deriving `JsonSchema`, its variants appear in `--help`
/// output and clap's error messages with no extra derives on the user type.
/// For non-enum types (e.g. `String`, `u32`), this is a transparent pass-through
/// to `FromStr`.
///
/// Used automatically by `#[cli]`-generated code when both the `cli` and
/// `jsonschema` features are enabled.
#[cfg(all(feature = "cli", feature = "jsonschema"))]
#[derive(Clone)]
pub struct SchemaValueParser<T: Clone + Send + Sync + 'static> {
    /// Enum variant names as `'static` str. We leak each string once at
    /// parser-construction time (command build, not per-parse), which is
    /// acceptable for a CLI binary: the leak is bounded (a few bytes per
    /// variant) and the memory is reclaimed when the process exits.
    variants: Option<std::sync::Arc<[&'static str]>>,
    _marker: std::marker::PhantomData<T>,
}

#[cfg(all(feature = "cli", feature = "jsonschema"))]
impl<T> Default for SchemaValueParser<T>
where
    T: schemars::JsonSchema + std::str::FromStr + Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(feature = "cli", feature = "jsonschema"))]
impl<T> SchemaValueParser<T>
where
    T: schemars::JsonSchema + std::str::FromStr + Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let variants = extract_enum_variants::<T>().map(|strings| {
            let leaked: Vec<&'static str> = strings
                .into_iter()
                .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
                .collect();
            leaked.into()
        });
        Self {
            variants,
            _marker: std::marker::PhantomData,
        }
    }
}

#[cfg(all(feature = "cli", feature = "jsonschema"))]
fn extract_enum_variants<T: schemars::JsonSchema>() -> Option<Vec<String>> {
    let schema_value = serde_json::to_value(schemars::schema_for!(T)).ok()?;
    let enum_values = schema_value.get("enum")?.as_array()?;
    let variants: Vec<String> = enum_values
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    if variants.is_empty() {
        None
    } else {
        Some(variants)
    }
}

#[cfg(all(feature = "cli", feature = "jsonschema"))]
impl<T> ::clap::builder::TypedValueParser for SchemaValueParser<T>
where
    T: schemars::JsonSchema + std::str::FromStr + Clone + Send + Sync + 'static,
{
    type Value = T;

    fn parse_ref(
        &self,
        _cmd: &::clap::Command,
        _arg: Option<&::clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<T, ::clap::Error> {
        let s = value
            .to_str()
            .ok_or_else(|| ::clap::Error::new(::clap::error::ErrorKind::InvalidUtf8))?;
        s.parse::<T>()
            .map_err(|_| ::clap::Error::new(::clap::error::ErrorKind::InvalidValue))
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = ::clap::builder::PossibleValue> + '_>> {
        let variants = self.variants.as_ref()?;
        Some(Box::new(
            variants
                .iter()
                .copied()
                .map(::clap::builder::PossibleValue::new),
        ))
    }
}

/// Runtime method metadata with string-based types.
///
/// This is a simplified, serialization-friendly representation of method
/// information intended for runtime introspection and tooling. Types are
/// stored as strings rather than `syn` AST nodes.
///
/// **Not to be confused with [`server_less_parse::MethodInfo`]**, which is
/// the richer, `syn`-based representation used internally by proc macros
/// during code generation. The parse version retains full type information
/// (`syn::Type`, `syn::Ident`) and supports `#[param(...)]` attributes.
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Method name (e.g., "create_user")
    pub name: String,
    /// Documentation string from /// comments
    pub docs: Option<String>,
    /// Parameter names and their type strings
    pub params: Vec<ParamInfo>,
    /// Return type string
    pub return_type: String,
    /// Whether the method is async
    pub is_async: bool,
    /// Whether the return type is a Stream
    pub is_streaming: bool,
    /// Whether the return type is `Option<T>`
    pub is_optional: bool,
    /// Whether the return type is `Result<T, E>`
    pub is_result: bool,
    /// Group display name for categorization
    pub group: Option<String>,
}

/// Runtime parameter metadata with string-based types.
///
/// See [`MethodInfo`] for the relationship between this type and
/// `server_less_parse::ParamInfo`.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Parameter name
    pub name: String,
    /// Type as string
    pub ty: String,
    /// Whether this is an `Option<T>`
    pub is_optional: bool,
    /// Whether this looks like an ID parameter (ends with _id or is named id)
    pub is_id: bool,
}

/// HTTP method inferred from function name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    /// Infer HTTP method from function name prefix
    pub fn infer_from_name(name: &str) -> Self {
        if name.starts_with("get_")
            || name.starts_with("fetch_")
            || name.starts_with("read_")
            || name.starts_with("list_")
            || name.starts_with("find_")
            || name.starts_with("search_")
        {
            HttpMethod::Get
        } else if name.starts_with("create_")
            || name.starts_with("add_")
            || name.starts_with("new_")
        {
            HttpMethod::Post
        } else if name.starts_with("update_") || name.starts_with("set_") {
            HttpMethod::Put
        } else if name.starts_with("patch_") || name.starts_with("modify_") {
            HttpMethod::Patch
        } else if name.starts_with("delete_") || name.starts_with("remove_") {
            HttpMethod::Delete
        } else {
            // Default to POST for RPC-style methods
            HttpMethod::Post
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }
}

/// Pluralize an English word using common rules.
///
/// Rules applied in order:
/// - Already ends in `s`, `x`, `z`, `ch`, or `sh` → append `es`
/// - Ends in a consonant followed by `y` → replace `y` with `ies`
/// - Everything else → append `s`
fn pluralize(word: &str) -> String {
    if word.ends_with('s')
        || word.ends_with('x')
        || word.ends_with('z')
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        format!("{word}es")
    } else if word.ends_with('y')
        && word.len() >= 2
        && !matches!(
            word.as_bytes()[word.len() - 2],
            b'a' | b'e' | b'i' | b'o' | b'u'
        )
    {
        // consonant + y → drop y, add ies
        format!("{}ies", &word[..word.len() - 1])
    } else {
        format!("{word}s")
    }
}

/// Infer URL path from method name
pub fn infer_path(method_name: &str, http_method: HttpMethod) -> String {
    // Strip common prefixes to get the resource name
    let resource = method_name
        .strip_prefix("get_")
        .or_else(|| method_name.strip_prefix("fetch_"))
        .or_else(|| method_name.strip_prefix("read_"))
        .or_else(|| method_name.strip_prefix("list_"))
        .or_else(|| method_name.strip_prefix("find_"))
        .or_else(|| method_name.strip_prefix("search_"))
        .or_else(|| method_name.strip_prefix("create_"))
        .or_else(|| method_name.strip_prefix("add_"))
        .or_else(|| method_name.strip_prefix("new_"))
        .or_else(|| method_name.strip_prefix("update_"))
        .or_else(|| method_name.strip_prefix("set_"))
        .or_else(|| method_name.strip_prefix("patch_"))
        .or_else(|| method_name.strip_prefix("modify_"))
        .or_else(|| method_name.strip_prefix("delete_"))
        .or_else(|| method_name.strip_prefix("remove_"))
        .unwrap_or(method_name);

    // Pluralize for collection endpoints.
    // If the resource already ends in 's' it is likely already plural (e.g. the
    // caller wrote `list_users` and we stripped the prefix to get "users").
    // For singular forms we apply English pluralization rules.
    let path_resource = if resource.ends_with('s') {
        resource.to_string()
    } else {
        pluralize(resource)
    };

    match http_method {
        // Collection operations
        HttpMethod::Post => format!("/{path_resource}"),
        HttpMethod::Get
            if method_name.starts_with("list_")
                || method_name.starts_with("search_")
                || method_name.starts_with("find_") =>
        {
            format!("/{path_resource}")
        }
        // Single resource operations
        HttpMethod::Get | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Delete => {
            format!("/{path_resource}/{{id}}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pluralize() {
        // ends in s/x/z/ch/sh → add es
        assert_eq!(pluralize("index"), "indexes");
        assert_eq!(pluralize("status"), "statuses");
        assert_eq!(pluralize("match"), "matches");
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("buzz"), "buzzes");
        assert_eq!(pluralize("brush"), "brushes");
        // consonant + y → ies
        assert_eq!(pluralize("query"), "queries");
        // vowel + y → plain s (key, day, …)
        assert_eq!(pluralize("key"), "keys");
        // default → add s
        assert_eq!(pluralize("item"), "items");
    }

    #[test]
    fn test_http_method_inference() {
        assert_eq!(HttpMethod::infer_from_name("get_user"), HttpMethod::Get);
        assert_eq!(HttpMethod::infer_from_name("list_users"), HttpMethod::Get);
        assert_eq!(HttpMethod::infer_from_name("create_user"), HttpMethod::Post);
        assert_eq!(HttpMethod::infer_from_name("update_user"), HttpMethod::Put);
        assert_eq!(
            HttpMethod::infer_from_name("delete_user"),
            HttpMethod::Delete
        );
        assert_eq!(
            HttpMethod::infer_from_name("do_something"),
            HttpMethod::Post
        ); // RPC fallback
    }

    #[test]
    fn test_path_inference() {
        assert_eq!(infer_path("create_user", HttpMethod::Post), "/users");
        assert_eq!(infer_path("get_user", HttpMethod::Get), "/users/{id}");
        assert_eq!(infer_path("list_users", HttpMethod::Get), "/users");
        assert_eq!(infer_path("delete_user", HttpMethod::Delete), "/users/{id}");
    }
}
