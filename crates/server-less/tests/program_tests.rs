//! Integration tests for the #[program] blessed preset.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::program;

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

#[program(name = "myctl", version = "2.0.0", about = "My cool CLI")]
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
    about = "Full app",
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
