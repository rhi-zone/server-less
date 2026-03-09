//! Integration tests for the #[program] blessed preset.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::program;
use server_less::Config;

// Basic program preset (zero-config)
struct BasicApp;

#[program]
impl BasicApp {
    /// Create a user
    pub fn create_user(&self, name: String) {
        println!("Created {}", name);
    }

    /// List users
    pub fn list_users(&self) {
        println!("Listing users...");
    }
}

#[test]
fn test_program_basic_cli_command() {
    let cmd = BasicApp::cli_command();
    let subcommands: Vec<_> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    assert!(subcommands.contains(&"create-user".to_string()));
    assert!(subcommands.contains(&"list-users".to_string()));
}

#[test]
fn test_program_basic_markdown_docs() {
    let docs = BasicApp::markdown_docs();
    assert!(
        docs.contains("create_user"),
        "Docs should contain create_user: {}",
        docs
    );
}

// Program with name and version
struct NamedApp;

#[program(name = "myctl", version = "2.0.0", description = "My cool CLI")]
impl NamedApp {
    /// Do something
    pub fn do_thing(&self, input: String) {
        println!("{}", input);
    }
}

#[test]
fn test_program_named_cli_command() {
    let cmd = NamedApp::cli_command();
    assert_eq!(cmd.get_name(), "myctl");
}

// Program with markdown disabled
struct NoDocsApp;

#[program(markdown = false)]
impl NoDocsApp {
    pub fn run(&self) {
        println!("Running...");
    }
}

#[test]
fn test_program_no_markdown() {
    let cmd = NoDocsApp::cli_command();
    let subcommands: Vec<_> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    assert!(subcommands.contains(&"run".to_string()));
    // markdown_docs() should NOT be available — verified by compilation
}

// Program with all options
struct FullApp;

#[program(
    name = "fullctl",
    version = "1.0.0",
    description = "Full app",
    markdown = true
)]
impl FullApp {
    /// Create something
    pub fn create(&self, name: String) {
        println!("Created {}", name);
    }
}

#[test]
fn test_program_full_options() {
    let cmd = FullApp::cli_command();
    assert_eq!(cmd.get_name(), "fullctl");
    let docs = FullApp::markdown_docs();
    assert!(!docs.is_empty());
}

// --- Config subcommand tests ---

#[derive(Config)]
struct AppConfig {
    #[param(default = "localhost", help = "Hostname to bind")]
    host: String,
    #[param(default = 8080, help = "Port to listen on", env = "APP_PORT")]
    port: u16,
    database_url: Option<String>,
}

struct ConfiguredApp;

#[program(config = AppConfig, name = "myapp")]
impl ConfiguredApp {
    /// Say hello
    pub fn greet(&self, name: String) -> String {
        format!("Hello, {name}!")
    }
}

#[test]
fn test_config_subcommand_appears_in_cli() {
    let cmd = ConfiguredApp::cli_command();
    let subcommands: Vec<_> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    assert!(
        subcommands.contains(&"config".to_string()),
        "Expected 'config' subcommand; got: {subcommands:?}"
    );
    assert!(
        subcommands.contains(&"greet".to_string()),
        "Expected 'greet' subcommand; got: {subcommands:?}"
    );
}

#[test]
fn test_config_subcommand_has_children() {
    let cmd = ConfiguredApp::cli_command();
    let config_cmd = cmd
        .get_subcommands()
        .find(|s| s.get_name() == "config")
        .expect("config subcommand missing");

    let children: Vec<_> = config_cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    assert!(children.contains(&"show".to_string()), "Missing 'show'");
    assert!(children.contains(&"schema".to_string()), "Missing 'schema'");
    assert!(children.contains(&"validate".to_string()), "Missing 'validate'");
    assert!(children.contains(&"set".to_string()), "Missing 'set'");
}

#[test]
fn test_config_cmd_custom_name() {
    struct AltApp;

    #[program(config = AppConfig, config_cmd = "settings", name = "altapp")]
    impl AltApp {
        pub fn ping(&self) -> String { "pong".into() }
    }

    let cmd = AltApp::cli_command();
    let names: Vec<_> = cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    assert!(names.contains(&"settings".to_string()), "Expected 'settings'; got: {names:?}");
    assert!(!names.contains(&"config".to_string()), "Unexpected 'config'");
}

#[test]
fn test_derive_config_load_defaults() {
    use server_less::{ConfigSource, ConfigTrait};
    let cfg = AppConfig::load(&[ConfigSource::Defaults]).unwrap();
    assert_eq!(cfg.host, "localhost");
    assert_eq!(cfg.port, 8080);
    assert_eq!(cfg.database_url, None);
}

#[test]
fn test_derive_config_field_meta() {
    use server_less::ConfigTrait;
    let meta = AppConfig::field_meta();
    assert_eq!(meta.len(), 3);

    let host_meta = meta.iter().find(|f| f.name == "host").expect("host field");
    assert_eq!(host_meta.default, Some("localhost"));
    assert_eq!(host_meta.help, Some("Hostname to bind"));
    assert!(!host_meta.required);

    let port_meta = meta.iter().find(|f| f.name == "port").expect("port field");
    assert_eq!(port_meta.env_var, Some("APP_PORT"));
    assert_eq!(port_meta.default, Some("8080"));

    let db_meta = meta.iter().find(|f| f.name == "database_url").expect("database_url field");
    assert!(!db_meta.required, "Option<T> should not be required");
}

// --- Nested Config tests ---

#[derive(Config)]
struct DaemonConfig {
    #[param(default = "true", help = "Enable daemon mode")]
    enabled: bool,
    #[param(default = "30", help = "Heartbeat interval in seconds")]
    heartbeat_secs: u64,
}

#[derive(Config)]
struct SearchConfig {
    #[param(default = "100")]
    max_results: u32,
    index_path: Option<String>,
}

#[derive(Config)]
struct FullNestedConfig {
    #[param(default = "myapp", help = "Application name")]
    app_name: String,
    #[param(nested)]
    daemon: DaemonConfig,
    /// Section name overridden with file_key
    #[param(nested, file_key = "text-search")]
    search: SearchConfig,
}

#[test]
fn test_nested_config_load_defaults() {
    use server_less::{ConfigSource, ConfigTrait};
    let cfg = FullNestedConfig::load(&[ConfigSource::Defaults]).unwrap();
    assert_eq!(cfg.app_name, "myapp");
    assert!(cfg.daemon.enabled);
    assert_eq!(cfg.daemon.heartbeat_secs, 30);
    assert_eq!(cfg.search.max_results, 100);
    assert_eq!(cfg.search.index_path, None);
}

#[test]
fn test_nested_config_from_toml_file() {
    use server_less::{ConfigSource, ConfigTrait};
    use std::io::Write;

    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        r#"
app_name = "testapp"

[daemon]
enabled = false
heartbeat_secs = 60

[text-search]
max_results = 50
index_path = "/var/search"
"#
    )
    .unwrap();

    let cfg = FullNestedConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::File(f.path().to_path_buf()),
    ])
    .unwrap();

    assert_eq!(cfg.app_name, "testapp");
    assert!(!cfg.daemon.enabled);
    assert_eq!(cfg.daemon.heartbeat_secs, 60);
    assert_eq!(cfg.search.max_results, 50);
    assert_eq!(cfg.search.index_path, Some("/var/search".to_string()));
}

#[test]
fn test_nested_config_env_prefix_inheritance() {
    use server_less::{ConfigSource, ConfigTrait};

    // APP_DAEMON_ENABLED and APP_DAEMON_HEARTBEAT_SECS should be read
    // SAFETY: single-threaded test, no other threads reading these vars.
    unsafe {
        std::env::set_var("APP_DAEMON_ENABLED", "false");
        std::env::set_var("APP_DAEMON_HEARTBEAT_SECS", "120");
    }

    let cfg = FullNestedConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::Env { prefix: Some("APP".into()) },
    ])
    .unwrap();

    unsafe {
        std::env::remove_var("APP_DAEMON_ENABLED");
        std::env::remove_var("APP_DAEMON_HEARTBEAT_SECS");
    }

    assert!(!cfg.daemon.enabled);
    assert_eq!(cfg.daemon.heartbeat_secs, 120);
    // search defaults unchanged
    assert_eq!(cfg.search.max_results, 100);
}

#[test]
fn test_nested_config_env_prefix_override() {
    use server_less::{ConfigSource, ConfigTrait};

    #[derive(Config)]
    struct OverriddenPrefixConfig {
        #[param(default = "main")]
        app_name: String,
        #[param(nested, env_prefix = "SEARCH")]
        search: SearchConfig,
    }

    // With env_prefix = "SEARCH", the child reads SEARCH_MAX_RESULTS not APP_SEARCH_MAX_RESULTS
    // SAFETY: single-threaded test, no other threads reading these vars.
    unsafe {
        std::env::set_var("SEARCH_MAX_RESULTS", "42");
    }

    let cfg = OverriddenPrefixConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::Env { prefix: Some("APP".into()) },
    ])
    .unwrap();

    unsafe {
        std::env::remove_var("SEARCH_MAX_RESULTS");
    }

    assert_eq!(cfg.search.max_results, 42);
}

#[test]
fn test_nested_config_field_meta_populated() {
    use server_less::ConfigTrait;
    let meta = FullNestedConfig::field_meta();

    let app_name_meta = meta.iter().find(|f| f.name == "app_name").expect("app_name field");
    assert!(app_name_meta.nested.is_none(), "app_name should not be nested");
    assert!(app_name_meta.env_prefix.is_none());

    let daemon_meta = meta.iter().find(|f| f.name == "daemon").expect("daemon field");
    assert!(daemon_meta.nested.is_some(), "daemon should have nested meta");
    let child_meta = daemon_meta.nested.unwrap();
    assert_eq!(child_meta.len(), 2);
    assert!(child_meta.iter().any(|f| f.name == "enabled"));
    assert!(child_meta.iter().any(|f| f.name == "heartbeat_secs"));

    let search_meta = meta.iter().find(|f| f.name == "search").expect("search field");
    assert!(search_meta.nested.is_some(), "search should have nested meta");
    assert_eq!(search_meta.file_key, Some("text-search"));
}

#[test]
fn test_nested_config_merge_file() {
    use server_less::{ConfigSource, ConfigTrait};
    use std::io::Write;

    // Global config: sets everything
    let mut global = tempfile::NamedTempFile::new().unwrap();
    write!(
        global,
        r#"
app_name = "global"

[daemon]
enabled = true
heartbeat_secs = 10

[text-search]
max_results = 200
"#
    )
    .unwrap();

    // Local (merge) config: only overrides daemon.heartbeat_secs
    let mut local = tempfile::NamedTempFile::new().unwrap();
    write!(
        local,
        r#"
[daemon]
heartbeat_secs = 99
"#
    )
    .unwrap();

    let cfg = FullNestedConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::File(global.path().to_path_buf()),
        ConfigSource::MergeFile(local.path().to_path_buf()),
    ])
    .unwrap();

    // global sets app_name; local doesn't touch it — but File replaces, so "global" wins
    assert_eq!(cfg.app_name, "global");
    // daemon.enabled set by global, not overridden by local
    assert!(cfg.daemon.enabled);
    // daemon.heartbeat_secs: global set 10, local MergeFile supplements to 99
    // MergeFile only fills in None fields for leaf vars, so since heartbeat_secs is already
    // set to 10 by File, MergeFile's 99 does NOT overwrite it.
    // This matches the "supplement, don't replace" semantics.
    assert_eq!(cfg.daemon.heartbeat_secs, 10);
    assert_eq!(cfg.search.max_results, 200);
}

// --- Serde-nested Config tests ---

/// A type that uses #[serde(flatten)] — incompatible with #[derive(Config)].
#[derive(serde::Deserialize, Debug, PartialEq)]
struct RulesConfig {
    #[serde(default)]
    strict: bool,
    #[serde(default = "default_max_rules")]
    max_rules: u32,
}

fn default_max_rules() -> u32 {
    50
}

/// A HashMap-backed config section — no named fields, so Config is impossible.
#[derive(serde::Deserialize, Debug, PartialEq)]
struct AliasMap {
    #[serde(flatten)]
    entries: std::collections::HashMap<String, String>,
}

/// An optional serde-nested section.
#[derive(serde::Deserialize, Debug, PartialEq)]
struct ExtrasConfig {
    #[serde(default)]
    notes: String,
}

#[derive(Config)]
struct SerdeNestedConfig {
    #[param(default = "main", help = "App name")]
    app_name: String,
    /// Rules section: serde-deserialized, not Config::load
    #[param(nested, serde)]
    rules: RulesConfig,
    /// Aliases section: serde-deserialized HashMap-backed type
    #[param(nested, serde, file_key = "aliases")]
    alias_map: AliasMap,
    /// Optional extras section
    #[param(nested, serde)]
    extras: Option<ExtrasConfig>,
}

#[test]
fn test_serde_nested_load_from_toml_file() {
    use server_less::{ConfigSource, ConfigTrait};
    use std::io::Write;

    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        r#"
app_name = "serde-test"

[rules]
strict = true
max_rules = 100

[aliases]
foo = "bar"
baz = "qux"

[extras]
notes = "hello"
"#
    )
    .unwrap();

    let cfg = SerdeNestedConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::File(f.path().to_path_buf()),
    ])
    .unwrap();

    assert_eq!(cfg.app_name, "serde-test");
    assert!(cfg.rules.strict);
    assert_eq!(cfg.rules.max_rules, 100);
    assert_eq!(cfg.alias_map.entries.get("foo").map(String::as_str), Some("bar"));
    assert_eq!(cfg.alias_map.entries.get("baz").map(String::as_str), Some("qux"));
    assert_eq!(cfg.extras, Some(ExtrasConfig { notes: "hello".to_string() }));
}

#[test]
fn test_serde_nested_serde_defaults_via_serde_default() {
    use server_less::{ConfigSource, ConfigTrait};
    use std::io::Write;

    // Only provide the required sections (rules and aliases), no extras
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        r#"
[rules]
# strict defaults to false via #[serde(default)]
max_rules = 10

[aliases]
"#
    )
    .unwrap();

    let cfg = SerdeNestedConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::File(f.path().to_path_buf()),
    ])
    .unwrap();

    // strict defaults to false (serde default)
    assert!(!cfg.rules.strict);
    assert_eq!(cfg.rules.max_rules, 10);
    // extras section absent → Option::None
    assert_eq!(cfg.extras, None);
}

#[test]
fn test_serde_nested_file_key_override() {
    use server_less::{ConfigSource, ConfigTrait};
    use std::io::Write;

    let mut f = tempfile::NamedTempFile::new().unwrap();
    // "aliases" is the file_key for alias_map
    write!(
        f,
        r#"
[rules]
strict = false

[aliases]
mykey = "myval"
"#
    )
    .unwrap();

    let cfg = SerdeNestedConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::File(f.path().to_path_buf()),
    ])
    .unwrap();

    assert_eq!(cfg.alias_map.entries.get("mykey").map(String::as_str), Some("myval"));
}

#[test]
fn test_serde_nested_env_vars_ignored() {
    use server_less::{ConfigSource, ConfigTrait};
    use std::io::Write;

    // Set env vars that would normally be picked up for a Config-nested type.
    // For serde-nested fields, these should be silently ignored.
    // SAFETY: single-threaded test.
    unsafe {
        std::env::set_var("APP_RULES_STRICT", "true");
        std::env::set_var("APP_ALIAS_MAP_FOO", "fromenv");
    }

    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        r#"
[rules]
strict = false
max_rules = 5

[aliases]
"#
    )
    .unwrap();

    let cfg = SerdeNestedConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::File(f.path().to_path_buf()),
        ConfigSource::Env { prefix: Some("APP".into()) },
    ])
    .unwrap();

    unsafe {
        std::env::remove_var("APP_RULES_STRICT");
        std::env::remove_var("APP_ALIAS_MAP_FOO");
    }

    // strict should remain false (from TOML), not overridden by env var
    assert!(!cfg.rules.strict, "env var should not override serde-nested field");
    // alias_map should not have a 'foo' key from env
    assert!(!cfg.alias_map.entries.contains_key("foo"), "env var should not inject into serde-nested field");
}

#[test]
fn test_serde_nested_merge_file_semantics() {
    use server_less::{ConfigSource, ConfigTrait};
    use std::io::Write;

    // Primary file: sets rules and aliases
    let mut primary = tempfile::NamedTempFile::new().unwrap();
    write!(
        primary,
        r#"
[rules]
strict = true
max_rules = 20

[aliases]
a = "1"
"#
    )
    .unwrap();

    // Merge file: also has rules section — should NOT overwrite since primary already set it
    let mut merge = tempfile::NamedTempFile::new().unwrap();
    write!(
        merge,
        r#"
[rules]
strict = false
max_rules = 99

[aliases]
b = "2"
"#
    )
    .unwrap();

    let cfg = SerdeNestedConfig::load(&[
        ConfigSource::Defaults,
        ConfigSource::File(primary.path().to_path_buf()),
        ConfigSource::MergeFile(merge.path().to_path_buf()),
    ])
    .unwrap();

    // File set rules first; MergeFile should NOT overwrite (supplement semantics)
    assert!(cfg.rules.strict, "MergeFile should not overwrite rules set by File");
    assert_eq!(cfg.rules.max_rules, 20, "MergeFile should not overwrite rules set by File");
    // alias_map: primary set it, so merge should not overwrite
    assert!(cfg.alias_map.entries.contains_key("a"));
    assert!(!cfg.alias_map.entries.contains_key("b"), "MergeFile should not overwrite aliases");
}
