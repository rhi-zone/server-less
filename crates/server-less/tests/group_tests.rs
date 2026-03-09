//! Integration tests for method group support.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::{CliSubcommand, cli, markdown, openapi, server};

// ============================================================================
// Grouped service fixture
// ============================================================================

#[derive(Clone)]
struct AnalyzeService;

#[cli(name = "analyze", description = "Code analysis tools")]
#[server(groups(
    code = "Code quality",
    modules = "Module structure",
    repo = "Repository",
))]
impl AnalyzeService {
    /// Rank functions by cyclomatic complexity
    #[server(group = "code")]
    pub fn complexity(&self, path: String) -> String {
        format!("complexity: {path}")
    }

    /// Rank functions by line count
    #[server(group = "code")]
    pub fn length(&self, path: String) -> String {
        format!("length: {path}")
    }

    /// Information density per module
    #[server(group = "modules")]
    pub fn density(&self, path: String) -> String {
        format!("density: {path}")
    }

    /// File change frequency
    #[server(group = "repo")]
    pub fn churn(&self, path: String) -> String {
        format!("churn: {path}")
    }

    /// Codebase health summary
    pub fn summary(&self) -> String {
        "summary".to_string()
    }
}

// ============================================================================
// CLI tests
// ============================================================================

#[test]
fn test_cli_grouped_command_created() {
    let cmd = AnalyzeService::cli_command();
    assert_eq!(cmd.get_name(), "analyze");
}

#[test]
fn test_cli_grouped_subcommands_exist() {
    let cmd = AnalyzeService::cli_command();
    let names: Vec<_> = cmd.get_subcommands().map(|c| c.get_name()).collect();
    assert!(names.contains(&"complexity"), "missing complexity: {names:?}");
    assert!(names.contains(&"length"), "missing length: {names:?}");
    assert!(names.contains(&"density"), "missing density: {names:?}");
    assert!(names.contains(&"churn"), "missing churn: {names:?}");
    assert!(names.contains(&"summary"), "missing summary: {names:?}");
}

#[test]
fn test_cli_grouped_help_contains_group_headings() {
    let cmd = AnalyzeService::cli_command();
    let after_help = cmd
        .get_after_help()
        .expect("should have after_help when groups are present")
        .to_string();

    assert!(
        after_help.contains("Code quality"),
        "missing 'Code quality' heading in after_help: {after_help}"
    );
    assert!(
        after_help.contains("Module structure"),
        "missing 'Module structure' heading in after_help: {after_help}"
    );
    assert!(
        after_help.contains("Repository"),
        "missing 'Repository' heading in after_help: {after_help}"
    );
    assert!(
        after_help.contains("Commands"),
        "missing 'Commands' heading for ungrouped methods: {after_help}"
    );
}

#[test]
fn test_cli_grouped_help_contains_subcommand_names() {
    let cmd = AnalyzeService::cli_command();
    let after_help = cmd.get_after_help().unwrap().to_string();

    assert!(after_help.contains("complexity"), "missing complexity");
    assert!(after_help.contains("length"), "missing length");
    assert!(after_help.contains("density"), "missing density");
    assert!(after_help.contains("churn"), "missing churn");
    assert!(after_help.contains("summary"), "missing summary");
}

#[test]
fn test_cli_grouped_help_contains_descriptions() {
    let cmd = AnalyzeService::cli_command();
    let after_help = cmd.get_after_help().unwrap().to_string();

    assert!(
        after_help.contains("Rank functions by cyclomatic complexity"),
        "missing complexity description"
    );
    assert!(
        after_help.contains("Information density per module"),
        "missing density description"
    );
}

#[test]
fn test_cli_grouped_subcommands_hidden_from_clap() {
    // When groups exist, leaf subcommands are hidden from clap's built-in
    // help rendering (we render them ourselves via after_help).
    let cmd = AnalyzeService::cli_command();
    for sub in cmd.get_subcommands() {
        assert!(
            sub.is_hide_set(),
            "subcommand '{}' should be hidden when groups are active",
            sub.get_name()
        );
    }
}

#[test]
fn test_cli_grouped_dispatch_works() {
    let service = AnalyzeService;
    let cmd = AnalyzeService::cli_command();
    let matches = cmd.get_matches_from(vec!["analyze", "complexity", "--path", "src/"]);
    assert!(service.cli_dispatch(&matches).is_ok());
}

#[test]
fn test_cli_ungrouped_no_after_help() {
    // Service without groups should NOT have after_help
    #[derive(Clone)]
    struct SimpleService;

    #[cli(name = "simple")]
    impl SimpleService {
        pub fn hello(&self) -> String {
            "hi".to_string()
        }
    }

    let cmd = SimpleService::cli_command();
    assert!(
        cmd.get_after_help().is_none(),
        "ungrouped service should not have after_help"
    );
}

// ============================================================================
// Group ordering tests
// ============================================================================

#[test]
fn test_cli_group_ordering_matches_declaration() {
    let cmd = AnalyzeService::cli_command();
    let after_help = cmd.get_after_help().unwrap().to_string();

    let code_pos = after_help.find("Code quality").expect("missing Code quality");
    let modules_pos = after_help.find("Module structure").expect("missing Module structure");
    let repo_pos = after_help.find("Repository").expect("missing Repository");

    assert!(
        code_pos < modules_pos,
        "Code quality should appear before Module structure"
    );
    assert!(
        modules_pos < repo_pos,
        "Module structure should appear before Repository"
    );
}

#[test]
fn test_cli_ungrouped_methods_appear_first() {
    let cmd = AnalyzeService::cli_command();
    let after_help = cmd.get_after_help().unwrap().to_string();

    let commands_pos = after_help.find("Commands").expect("missing Commands heading");
    let first_group_pos = after_help.find("Code quality").expect("missing Code quality");

    assert!(
        commands_pos < first_group_pos,
        "Ungrouped 'Commands' section should appear before grouped sections"
    );
}

// ============================================================================
// OpenAPI tests
// ============================================================================

#[derive(Clone)]
struct OpenApiGroupedService;

#[openapi]
#[server(groups(
    users = "Users",
    admin = "Administration",
))]
impl OpenApiGroupedService {
    /// Get a user
    #[server(group = "users")]
    pub fn get_user(&self, id: String) -> String {
        id
    }

    /// Admin reset
    #[server(group = "admin")]
    pub fn reset(&self) -> String {
        "reset".to_string()
    }

    /// Health check
    pub fn health(&self) -> String {
        "ok".to_string()
    }
}

#[test]
fn test_openapi_group_becomes_tag() {
    let spec = OpenApiGroupedService::openapi_spec();
    let paths = &spec["paths"];

    let get_user = &paths["/users/{id}"]["get"];
    let tags = get_user["tags"].as_array().expect("should have tags");
    assert!(
        tags.iter().any(|t| t.as_str() == Some("Users")),
        "group display name should appear as tag. Tags: {tags:?}"
    );
}

#[test]
fn test_openapi_ungrouped_no_group_tag() {
    let spec = OpenApiGroupedService::openapi_spec();
    let paths = &spec["paths"];

    let health = &paths["/healths"]["get"];
    // Ungrouped method — should have no tags or empty tags
    let tags = health.get("tags");
    if let Some(tags) = tags {
        let arr = tags.as_array().unwrap();
        assert!(
            arr.is_empty(),
            "ungrouped method should have no tags: {arr:?}"
        );
    }
}

#[test]
fn test_openapi_group_admin_tag() {
    let spec = OpenApiGroupedService::openapi_spec();
    let paths = &spec["paths"];

    let reset = &paths["/resets"]["post"];
    let tags = reset["tags"].as_array().expect("should have tags");
    assert!(
        tags.iter().any(|t| t.as_str() == Some("Administration")),
        "admin group should appear as 'Administration' tag. Tags: {tags:?}"
    );
}

// ============================================================================
// Markdown tests
// ============================================================================

#[derive(Clone)]
struct MarkdownGroupedService;

#[markdown(title = "Analysis API")]
#[server(groups(
    code = "Code Quality",
    structure = "Structure",
))]
impl MarkdownGroupedService {
    /// Check complexity
    #[server(group = "code")]
    pub fn complexity(&self) -> String {
        "complex".to_string()
    }

    /// Check structure
    #[server(group = "structure")]
    pub fn modules(&self) -> String {
        "modules".to_string()
    }

    /// General info
    pub fn info(&self) -> String {
        "info".to_string()
    }
}

#[test]
fn test_markdown_group_sections() {
    let docs = MarkdownGroupedService::markdown_docs();
    assert!(
        docs.contains("## Code Quality"),
        "missing group section heading: {docs}"
    );
    assert!(
        docs.contains("## Structure"),
        "missing group section heading: {docs}"
    );
}

#[test]
fn test_markdown_ungrouped_section() {
    let docs = MarkdownGroupedService::markdown_docs();
    assert!(
        docs.contains("## Methods"),
        "ungrouped methods should appear under '## Methods': {docs}"
    );
}

#[test]
fn test_markdown_group_summary_in_overview() {
    let docs = MarkdownGroupedService::markdown_docs();
    assert!(
        docs.contains("Code Quality"),
        "overview should mention group names: {docs}"
    );
    assert!(
        docs.contains("Structure"),
        "overview should mention group names: {docs}"
    );
}
