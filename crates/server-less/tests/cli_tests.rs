//! Integration tests for the CLI macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use server_less::{CliSubcommand, cli};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct Item {
    id: String,
    name: String,
}

impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

#[derive(Clone)]
struct ItemService {
    items: std::sync::Arc<std::sync::Mutex<Vec<Item>>>,
}

impl ItemService {
    fn new() -> Self {
        Self {
            items: std::sync::Arc::new(std::sync::Mutex::new(vec![Item {
                id: "1".to_string(),
                name: "Test".to_string(),
            }])),
        }
    }
}

#[cli(name = "item-cli", version = "1.0.0", about = "Manage items")]
impl ItemService {
    /// List all items
    pub fn list_items(&self) -> Vec<Item> {
        self.items.lock().unwrap().clone()
    }

    /// Get item by ID
    pub fn get_item(&self, item_id: String) -> Option<Item> {
        self.items
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == item_id)
            .cloned()
    }

    /// Create an item
    pub fn create_item(&self, name: String) -> Item {
        let mut items = self.items.lock().unwrap();
        let item = Item {
            id: (items.len() + 1).to_string(),
            name,
        };
        items.push(item.clone());
        item
    }
}

#[test]
fn test_cli_command_created() {
    let cmd = ItemService::cli_command();
    assert_eq!(cmd.get_name(), "item-cli");
}

#[test]
fn test_cli_has_subcommands() {
    let cmd = ItemService::cli_command();
    let subcommands: Vec<_> = cmd.get_subcommands().collect();

    let names: Vec<_> = subcommands.iter().map(|c| c.get_name()).collect();
    assert!(names.contains(&"list-items"));
    assert!(names.contains(&"get-item"));
    assert!(names.contains(&"create-item"));
}

#[test]
fn test_cli_subcommand_has_args() {
    let cmd = ItemService::cli_command();

    // Find create-item subcommand
    let create_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "create-item")
        .unwrap();

    let args: Vec<_> = create_cmd.get_arguments().collect();
    let arg_names: Vec<_> = args.iter().map(|a| a.get_id().as_str()).collect();

    assert!(arg_names.contains(&"name"));
}

#[test]
fn test_cli_id_param_is_positional() {
    let cmd = ItemService::cli_command();

    // Find get-item subcommand
    let get_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "get-item")
        .unwrap();

    let id_arg = get_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "item-id")
        .unwrap();

    // ID params should be positional (have an index)
    assert!(id_arg.get_index().is_some());
}

#[test]
fn test_cli_run_list() {
    let service = ItemService::new();
    // Run with list-items subcommand
    let result = service.cli_run_with(["item-cli", "list-items"]);
    assert!(result.is_ok());
}

#[test]
fn test_cli_run_help() {
    let service = ItemService::new();
    // Running without subcommand should print help (not error)
    let result = service.cli_run_with(["item-cli"]);
    assert!(result.is_ok());
}

// --- Mount point tests ---

// A child service that will be mounted
#[derive(Clone)]
struct UserService {
    name: String,
}

impl UserService {
    fn new() -> Self {
        Self {
            name: "default".to_string(),
        }
    }
}

#[cli(name = "users")]
impl UserService {
    /// List all users
    pub fn list(&self) -> Vec<String> {
        vec![self.name.clone()]
    }

    /// Edit a user's name
    pub fn edit(&self, name: String) {
        println!("Editing user name to: {}", name);
    }
}

// Another child for multi-mount testing
#[derive(Clone)]
struct PostService;

#[cli(name = "posts")]
impl PostService {
    /// List all posts
    pub fn list(&self) -> Vec<String> {
        vec!["post1".to_string()]
    }

    /// Create a post
    pub fn create(&self, title: String) {
        println!("Creating post: {}", title);
    }
}

// Grandchild for deep nesting
#[derive(Clone)]
struct CommentService;

#[cli(name = "comments")]
impl CommentService {
    /// List comments
    pub fn list(&self) -> Vec<String> {
        vec!["comment1".to_string()]
    }
}

// A nested service (level 2) that also has a mount
#[derive(Clone)]
struct NestedPostService;

#[cli(name = "nested-posts")]
impl NestedPostService {
    /// List posts
    pub fn list(&self) -> Vec<String> {
        vec!["post1".to_string()]
    }

    /// Mount comments under posts
    pub fn comments(&self) -> &CommentService {
        static SVC: CommentService = CommentService;
        &SVC
    }
}

// Parent with static mount
#[derive(Clone)]
struct ParentApp {
    users: UserService,
    posts: PostService,
}

impl ParentApp {
    fn new() -> Self {
        Self {
            users: UserService::new(),
            posts: PostService,
        }
    }
}

#[cli(name = "app", version = "1.0.0")]
impl ParentApp {
    /// Health check
    pub fn health(&self) -> String {
        "ok".to_string()
    }

    /// Mount user commands
    pub fn users(&self) -> &UserService {
        &self.users
    }

    /// Mount post commands
    pub fn posts(&self) -> &PostService {
        &self.posts
    }
}

#[test]
fn test_static_mount_subcommands_present() {
    let cmd = ParentApp::cli_command();
    let names: Vec<_> = cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();

    assert!(names.contains(&"health".to_string()));
    assert!(names.contains(&"users".to_string()));
    assert!(names.contains(&"posts".to_string()));
}

#[test]
fn test_static_mount_child_subcommands() {
    let cmd = ParentApp::cli_command();
    let users_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "users")
        .unwrap();

    let child_names: Vec<_> = users_cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    assert!(child_names.contains(&"list".to_string()));
    assert!(child_names.contains(&"edit".to_string()));
}

#[test]
fn test_static_mount_dispatch_leaf_on_parent() {
    let app = ParentApp::new();
    let result = app.cli_run_with(["app", "health"]);
    assert!(result.is_ok());
}

#[test]
fn test_static_mount_dispatch_child() {
    let app = ParentApp::new();
    let result = app.cli_run_with(["app", "users", "list"]);
    assert!(result.is_ok());
}

#[test]
fn test_static_mount_dispatch_child_with_args() {
    let app = ParentApp::new();
    let result = app.cli_run_with(["app", "users", "edit", "--name", "Alice"]);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_static_mounts() {
    let app = ParentApp::new();
    // Can dispatch to both children
    assert!(app.cli_run_with(["app", "users", "list"]).is_ok());
    assert!(app.cli_run_with(["app", "posts", "list"]).is_ok());
    assert!(
        app.cli_run_with(["app", "posts", "create", "--title", "Hello"])
            .is_ok()
    );
}

// Slug mount: parent with parameterized child
#[derive(Clone)]
struct SlugApp {
    user_svc: UserService,
}

impl SlugApp {
    fn new() -> Self {
        Self {
            user_svc: UserService::new(),
        }
    }
}

#[cli(name = "slug-app")]
impl SlugApp {
    /// Access a specific user by ID
    pub fn user(&self, id: String) -> &UserService {
        // In real code, id would select a user; for testing we just return the service
        &self.user_svc
    }
}

#[test]
fn test_slug_mount_subcommand_present() {
    let cmd = SlugApp::cli_command();
    let user_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "user")
        .unwrap();

    // Should have the slug param "id"
    let id_arg = user_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "id");
    assert!(id_arg.is_some());
    assert!(id_arg.unwrap().is_required_set());

    // Should also have child subcommands
    let child_names: Vec<_> = user_cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    assert!(child_names.contains(&"list".to_string()));
    assert!(child_names.contains(&"edit".to_string()));
}

#[test]
fn test_slug_mount_dispatch() {
    let app = SlugApp::new();
    let result = app.cli_run_with(["slug-app", "user", "42", "list"]);
    assert!(result.is_ok());
}

#[test]
fn test_slug_mount_dispatch_with_child_args() {
    let app = SlugApp::new();
    let result = app.cli_run_with(["slug-app", "user", "42", "edit", "--name", "Alice"]);
    assert!(result.is_ok());
}

// Deep nesting: 3 levels
#[derive(Clone)]
struct DeepApp;

#[cli(name = "deep-app")]
impl DeepApp {
    /// Mount nested posts
    pub fn posts(&self) -> &NestedPostService {
        static SVC: NestedPostService = NestedPostService;
        &SVC
    }
}

#[test]
fn test_deep_nesting_3_levels() {
    let cmd = DeepApp::cli_command();

    // Level 1: posts
    let posts_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "posts")
        .unwrap();

    // Level 2: posts > comments
    let comments_cmd = posts_cmd
        .get_subcommands()
        .find(|c| c.get_name() == "comments")
        .unwrap();

    // Level 3: posts > comments > list
    let list_cmd = comments_cmd
        .get_subcommands()
        .find(|c| c.get_name() == "list");
    assert!(list_cmd.is_some());
}

#[test]
fn test_deep_nesting_dispatch() {
    let app = DeepApp;
    assert!(app.cli_run_with(["deep-app", "posts", "list"]).is_ok());
    assert!(
        app.cli_run_with(["deep-app", "posts", "comments", "list"])
            .is_ok()
    );
}

// #[cli(skip)] test
struct SkipService {
    internal: UserService,
}

impl SkipService {
    fn new() -> Self {
        Self {
            internal: UserService::new(),
        }
    }
}

#[cli(name = "skip-app")]
impl SkipService {
    /// A visible command
    pub fn status(&self) -> String {
        "ok".to_string()
    }

    /// This returns &T but should be skipped
    #[cli(skip)]
    pub fn internal(&self) -> &UserService {
        &self.internal
    }
}

#[test]
fn test_cli_skip_excludes_mount() {
    let cmd = SkipService::cli_command();
    let names: Vec<_> = cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();

    assert!(names.contains(&"status".to_string()));
    assert!(!names.contains(&"internal".to_string()));
}

// CliSubcommand trait is implemented
#[test]
fn test_cli_subcommand_trait_implemented() {
    // Verify the trait is implemented by calling the trait methods directly
    let cmd = <UserService as CliSubcommand>::cli_command();
    assert_eq!(cmd.get_name(), "users");

    let svc = UserService::new();
    let matches = <UserService as CliSubcommand>::cli_command().get_matches_from(["users", "list"]);
    let result = <UserService as CliSubcommand>::cli_dispatch(&svc, &matches);
    assert!(result.is_ok());
}

// ── Bool switch tests ─────────────────────────────────────────────────

#[derive(Clone)]
struct BoolService;

#[cli(name = "bool-app")]
impl BoolService {
    /// Run with optional verbose flag
    pub fn run(&self, verbose: bool, name: String) -> String {
        if verbose {
            format!("VERBOSE: {name}")
        } else {
            name
        }
    }
}

#[test]
fn test_bool_arg_has_set_true_action() {
    let cmd = BoolService::cli_command();
    let run_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "run")
        .unwrap();

    let verbose_arg = run_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "verbose")
        .unwrap();

    // SetTrue args don't require a value
    assert!(!verbose_arg.is_required_set());
}

#[test]
fn test_bool_dispatch_with_flag() {
    let svc = BoolService;
    let result = svc.cli_run_with(["bool-app", "run", "--verbose", "--name", "test"]);
    assert!(result.is_ok());
}

#[test]
fn test_bool_dispatch_without_flag() {
    let svc = BoolService;
    let result = svc.cli_run_with(["bool-app", "run", "--name", "test"]);
    assert!(result.is_ok());
}

// ── Vec param tests ───────────────────────────────────────────────────

#[derive(Clone)]
struct VecService;

#[cli(name = "vec-app")]
impl VecService {
    /// Tag items
    pub fn tag(&self, tags: Vec<String>) -> Vec<String> {
        tags
    }
}

#[test]
fn test_vec_arg_has_append_action() {
    let cmd = VecService::cli_command();
    let tag_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "tag")
        .unwrap();

    let tags_arg = tag_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "tags")
        .unwrap();

    // Append args are not required
    assert!(!tags_arg.is_required_set());
}

#[test]
fn test_vec_dispatch_repeated() {
    let svc = VecService;
    let result = svc.cli_run_with(["vec-app", "tag", "--tags", "a", "--tags", "b"]);
    assert!(result.is_ok());
}

#[test]
fn test_vec_dispatch_comma_delimited() {
    let svc = VecService;
    let result = svc.cli_run_with(["vec-app", "tag", "--tags", "a,b"]);
    assert!(result.is_ok());
}

#[test]
fn test_vec_dispatch_empty() {
    let svc = VecService;
    let result = svc.cli_run_with(["vec-app", "tag"]);
    assert!(result.is_ok());
}

// ── Global flag tests ─────────────────────────────────────────────────

#[derive(Clone)]
struct GlobalApp;

#[cli(name = "global-app", global = [verbose, dry_run])]
impl GlobalApp {
    /// List things
    pub fn list(&self, verbose: bool, dry_run: bool) -> Vec<String> {
        if verbose {
            vec!["verbose-item".to_string()]
        } else {
            vec!["item".to_string()]
        }
    }
}

#[test]
fn test_global_flags_on_root_command() {
    let cmd = GlobalApp::cli_command();

    let verbose_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "verbose");
    assert!(verbose_arg.is_some());

    let dry_run_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "dry-run");
    assert!(dry_run_arg.is_some());
}

#[test]
fn test_global_flag_before_subcommand() {
    let app = GlobalApp;
    let result = app.cli_run_with(["global-app", "--verbose", "list"]);
    assert!(result.is_ok());
}

#[test]
fn test_global_flag_after_subcommand() {
    let app = GlobalApp;
    let result = app.cli_run_with(["global-app", "list", "--verbose"]);
    assert!(result.is_ok());
}

// ── Output formatting tests ──────────────────────────────────────────

#[test]
fn test_format_flags_present() {
    let cmd = ItemService::cli_command();

    assert!(cmd.get_arguments().any(|a| a.get_id().as_str() == "jsonl"));
    assert!(cmd.get_arguments().any(|a| a.get_id().as_str() == "json"));
    assert!(cmd.get_arguments().any(|a| a.get_id().as_str() == "jq"));
}

#[test]
fn test_jsonl_flag_dispatch() {
    let svc = ItemService::new();
    let result = svc.cli_run_with(["item-cli", "--jsonl", "list-items"]);
    assert!(result.is_ok());
}

#[test]
fn test_json_flag_dispatch() {
    let svc = ItemService::new();
    let result = svc.cli_run_with(["item-cli", "--json", "list-items"]);
    assert!(result.is_ok());
}

#[test]
fn test_jq_flag_dispatch() {
    let svc = ItemService::new();
    let result = svc.cli_run_with(["item-cli", "--jq", ".[0].name", "list-items"]);
    assert!(result.is_ok());
}

// ── Defaults hook tests ──────────────────────────────────────────────

#[derive(Clone)]
struct DefaultsService;

impl DefaultsService {
    fn my_defaults(&self, param_name: &str) -> Option<String> {
        match param_name {
            "greeting" => Some("hello".to_string()),
            _ => None,
        }
    }
}

#[cli(name = "defaults-app", defaults = "my_defaults")]
impl DefaultsService {
    /// Greet someone
    pub fn greet(&self, greeting: String, target: String) -> String {
        format!("{greeting}, {target}!")
    }
}

#[test]
fn test_defaults_missing_arg_uses_default() {
    let svc = DefaultsService;
    // greeting not provided — should fall back to my_defaults
    let result = svc.cli_run_with(["defaults-app", "greet", "--target", "world"]);
    assert!(result.is_ok());
}

#[test]
fn test_defaults_explicit_arg_overrides() {
    let svc = DefaultsService;
    let result = svc.cli_run_with([
        "defaults-app",
        "greet",
        "--greeting",
        "hi",
        "--target",
        "world",
    ]);
    assert!(result.is_ok());
}

#[test]
fn test_defaults_missing_non_defaulted_errors() {
    let svc = DefaultsService;
    // target has no default → should error
    let result = svc.cli_run_with(["defaults-app", "greet"]);
    assert!(result.is_err());
}

// ── Schema & --params-json tests ──────────────────────────────────────

#[test]
fn test_schema_flags_present_on_root_command() {
    let cmd = ItemService::cli_command();

    assert!(
        cmd.get_arguments()
            .any(|a| a.get_id().as_str() == "input-schema")
    );
    assert!(
        cmd.get_arguments()
            .any(|a| a.get_id().as_str() == "output-schema")
    );
    assert!(
        cmd.get_arguments()
            .any(|a| a.get_id().as_str() == "params-json")
    );
}

#[test]
fn test_input_schema_dispatch() {
    let svc = ItemService::new();
    // --input-schema should print schema and return Ok (without running the method)
    let result = svc.cli_run_with(["item-cli", "--input-schema", "create-item"]);
    assert!(result.is_ok());
}

#[test]
fn test_output_schema_dispatch() {
    let svc = ItemService::new();
    let result = svc.cli_run_with(["item-cli", "--output-schema", "create-item"]);
    assert!(result.is_ok());
}

#[test]
fn test_input_schema_no_params_method() {
    let svc = ItemService::new();
    // list-items has no params — schema should still work
    let result = svc.cli_run_with(["item-cli", "--input-schema", "list-items"]);
    assert!(result.is_ok());
}

#[test]
fn test_output_schema_option_return() {
    let svc = ItemService::new();
    // get-item returns Option<Item> — output schema should still work
    let result = svc.cli_run_with(["item-cli", "--output-schema", "get-item"]);
    assert!(result.is_ok());
}

// Use a service that records what it received to verify --params-json extraction
#[derive(Clone)]
struct ParamsJsonService {
    received: std::sync::Arc<std::sync::Mutex<Option<(String, String)>>>,
}

impl ParamsJsonService {
    fn new() -> Self {
        Self {
            received: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

#[cli(name = "pj-app")]
impl ParamsJsonService {
    /// Create something
    pub fn create(&self, name: String, count: u32) -> String {
        *self.received.lock().unwrap() = Some((name.clone(), count.to_string()));
        format!("{}:{}", name, count)
    }

    /// Toggle something
    pub fn toggle(&self, flag: bool) -> String {
        format!("{}", flag)
    }
}

#[test]
fn test_params_json_dispatch() {
    let svc = ParamsJsonService::new();
    let result = svc.cli_run_with([
        "pj-app",
        "--params-json",
        r#"{"name":"alice","count":42}"#,
        "create",
    ]);
    assert!(result.is_ok());
    let received = svc.received.lock().unwrap();
    assert_eq!(
        received.as_ref().unwrap(),
        &("alice".to_string(), "42".to_string())
    );
}

#[test]
fn test_params_json_missing_required_field_errors() {
    let svc = ParamsJsonService::new();
    // Missing "count" field
    let result = svc.cli_run_with(["pj-app", "--params-json", r#"{"name":"alice"}"#, "create"]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("count"),
        "error should mention the missing field: {}",
        err_msg
    );
}

#[test]
fn test_params_json_invalid_json_errors() {
    let svc = ParamsJsonService::new();
    let result = svc.cli_run_with(["pj-app", "--params-json", "not json", "create"]);
    assert!(result.is_err());
}

#[test]
fn test_params_json_bool_field() {
    let svc = ParamsJsonService::new();
    let result = svc.cli_run_with(["pj-app", "--params-json", r#"{"flag":true}"#, "toggle"]);
    assert!(result.is_ok());
}

// ── Short flag and help text tests ────────────────────────────────────

#[derive(Clone)]
struct ShortFlagService;

#[cli(name = "short-app")]
impl ShortFlagService {
    /// Greet someone
    pub fn greet(
        &self,
        #[param(short = 'n', help = "Name of the person to greet")] name: String,
        #[param(short = 'v')] verbose: bool,
    ) -> String {
        if verbose {
            format!("Hello, {}! (verbose)", name)
        } else {
            format!("Hello, {}!", name)
        }
    }

    /// Search with custom help
    pub fn search(&self, #[param(help = "The search query to execute")] query: String) -> String {
        format!("Searching for: {}", query)
    }
}

#[test]
fn test_short_flag_on_string_param() {
    let cmd = ShortFlagService::cli_command();
    let greet_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "greet")
        .unwrap();

    let name_arg = greet_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "name")
        .unwrap();

    assert_eq!(name_arg.get_short(), Some('n'));
}

#[test]
fn test_short_flag_on_bool_param() {
    let cmd = ShortFlagService::cli_command();
    let greet_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "greet")
        .unwrap();

    let verbose_arg = greet_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "verbose")
        .unwrap();

    assert_eq!(verbose_arg.get_short(), Some('v'));
}

#[test]
fn test_short_flag_dispatch() {
    let svc = ShortFlagService;
    let result = svc.cli_run_with(["short-app", "greet", "-n", "Alice", "-v"]);
    assert!(result.is_ok());
}

#[test]
fn test_help_text_on_param() {
    let cmd = ShortFlagService::cli_command();
    let greet_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "greet")
        .unwrap();

    let name_arg = greet_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "name")
        .unwrap();

    assert_eq!(
        name_arg.get_help().map(|h| h.to_string()),
        Some("Name of the person to greet".to_string())
    );
}

#[test]
fn test_help_text_without_short_flag() {
    let cmd = ShortFlagService::cli_command();
    let search_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "search")
        .unwrap();

    let query_arg = search_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "query")
        .unwrap();

    assert_eq!(
        query_arg.get_help().map(|h| h.to_string()),
        Some("The search query to execute".to_string())
    );
    // No short flag set
    assert_eq!(query_arg.get_short(), None);
}

// ============================================================================
// #[cli(default)] tests
// ============================================================================

struct DefaultApp;

#[cli(name = "default-app")]
impl DefaultApp {
    /// Run the default action
    #[cli(default)]
    fn status(&self) -> String {
        "ok".to_string()
    }

    /// A peer subcommand
    fn version(&self) -> String {
        "1.0".to_string()
    }
}

struct DefaultWithArgs;

#[cli(name = "default-args-app")]
impl DefaultWithArgs {
    /// Default action with a flag
    #[cli(default)]
    fn run(&self, verbose: bool) -> String {
        if verbose {
            "verbose".to_string()
        } else {
            "quiet".to_string()
        }
    }

    fn other(&self) -> String {
        "other".to_string()
    }
}

struct DefaultHidden;

#[cli(name = "default-hidden-app")]
impl DefaultHidden {
    /// Default action, hidden from help
    #[cli(default, hidden)]
    fn run(&self) -> String {
        "run".to_string()
    }

    fn other(&self) -> String {
        "other".to_string()
    }
}

#[test]
fn test_cli_default_runs_when_no_subcommand() {
    let app = DefaultApp;
    assert!(app.cli_run_with(["default-app"]).is_ok());
}

#[test]
fn test_cli_default_subcommand_still_works_explicitly() {
    let app = DefaultApp;
    assert!(app.cli_run_with(["default-app", "status"]).is_ok());
}

#[test]
fn test_cli_peer_subcommand_still_works() {
    let app = DefaultApp;
    assert!(app.cli_run_with(["default-app", "version"]).is_ok());
}

#[test]
fn test_cli_default_flag_passed_without_subcommand() {
    let app = DefaultWithArgs;
    assert!(app.cli_run_with(["default-args-app", "--verbose"]).is_ok());
}

#[test]
fn test_cli_default_flag_passed_with_explicit_subcommand() {
    let app = DefaultWithArgs;
    assert!(
        app.cli_run_with(["default-args-app", "run", "--verbose"])
            .is_ok()
    );
}

#[test]
fn test_cli_default_hidden_not_in_help() {
    let cmd = DefaultHidden::cli_command();
    let run_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "run")
        .unwrap();
    assert!(run_cmd.is_hide_set());
}

#[test]
fn test_cli_default_hidden_still_dispatches_without_subcommand() {
    let app = DefaultHidden;
    assert!(app.cli_run_with(["default-hidden-app"]).is_ok());
}
