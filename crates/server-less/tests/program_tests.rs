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
