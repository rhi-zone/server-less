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

#[cli(name = "item-cli", version = "1.0.0", description = "Manage items")]
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

// Leaf method with #[cli(name = "...")] override
#[derive(Clone)]
struct RenamedLeafApp;

#[cli(name = "leaf-app")]
impl RenamedLeafApp {
    /// Check health
    #[cli(name = "status")]
    pub fn health_check(&self) -> String {
        "ok".to_string()
    }
}

#[test]
fn test_leaf_name_override() {
    let cmd = RenamedLeafApp::cli_command();
    let names: Vec<_> = cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    assert!(names.contains(&"status".to_string()), "names: {:?}", names);
    assert!(!names.contains(&"health-check".to_string()), "names: {:?}", names);
}

#[test]
fn test_leaf_name_override_dispatch() {
    let app = RenamedLeafApp;
    assert!(app.cli_run_with(["leaf-app", "status"]).is_ok());
}

// Leaf method with hidden `#[cli(alias = "...")]` aliases (migration scaffolding).
#[derive(Clone)]
struct AliasedLeafApp;

#[cli(name = "aliased-app")]
impl AliasedLeafApp {
    /// Analyze architecture
    #[cli(name = "architecture", alias = "arch", alias = "analyze-architecture")]
    pub fn architecture(&self) -> String {
        "ok".to_string()
    }
}

#[test]
fn test_leaf_alias_not_shown_in_help() {
    let cmd = AliasedLeafApp::cli_command();
    // The canonical name is a real subcommand...
    let names: Vec<_> = cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    assert!(names.contains(&"architecture".to_string()), "names: {names:?}");
    // ...and the aliases are NOT separate subcommands.
    assert!(!names.contains(&"arch".to_string()), "names: {names:?}");
    assert!(!names.contains(&"analyze-architecture".to_string()), "names: {names:?}");

    let sub = cmd.find_subcommand("architecture").expect("architecture subcommand");
    // Aliases are registered but HIDDEN: present in all-aliases, absent from visible.
    let all: Vec<_> = sub.get_all_aliases().collect();
    assert!(all.contains(&"arch"), "all aliases: {all:?}");
    assert!(all.contains(&"analyze-architecture"), "all aliases: {all:?}");
    let visible: Vec<_> = sub.get_visible_aliases().collect();
    assert!(visible.is_empty(), "aliases must be hidden, got visible: {visible:?}");

    // clap resolves the alias to the canonical subcommand.
    assert!(cmd.clone().find_subcommand("arch").is_some());
}

#[test]
fn test_leaf_alias_dispatch_both_names() {
    let app = AliasedLeafApp;
    // Invocable under the real name and under each hidden alias.
    assert!(app.cli_run_with(["aliased-app", "architecture"]).is_ok());
    assert!(app.cli_run_with(["aliased-app", "arch"]).is_ok());
    assert!(app.cli_run_with(["aliased-app", "analyze-architecture"]).is_ok());
}

// Static mount with #[cli(name = "...")] override
#[derive(Clone)]
struct RenamedMountApp {
    user_svc: UserService,
}

impl RenamedMountApp {
    fn new() -> Self {
        Self {
            user_svc: UserService::new(),
        }
    }
}

#[cli(name = "renamed-app")]
impl RenamedMountApp {
    /// Manage team members (renamed from field)
    #[cli(name = "members")]
    pub fn user_svc(&self) -> &UserService {
        &self.user_svc
    }
}

#[test]
fn test_static_mount_name_override() {
    let cmd = RenamedMountApp::cli_command();
    let names: Vec<_> = cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();
    // Should use the #[cli(name = "members")] override, not "user-svc"
    assert!(names.contains(&"members".to_string()), "names: {:?}", names);
    assert!(!names.contains(&"user-svc".to_string()), "names: {:?}", names);
}

#[test]
fn test_static_mount_name_override_dispatch() {
    let app = RenamedMountApp::new();
    // Dispatch using the overridden name
    assert!(app.cli_run_with(["renamed-app", "members", "list"]).is_ok());
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

// #[cli(helper)] test — self-documenting alias for #[cli(skip)]
#[derive(Clone)]
struct HelperService;

#[cli(name = "helper-app")]
impl HelperService {
    /// A visible command
    pub fn list(&self) -> Vec<String> {
        self.seed_data()
    }

    /// Internal helper — should not become a subcommand
    #[cli(helper)]
    pub fn seed_data(&self) -> Vec<String> {
        vec!["a".to_string(), "b".to_string()]
    }
}

#[test]
fn test_cli_helper_excludes_method() {
    let cmd = HelperService::cli_command();
    let names: Vec<_> = cmd
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect();

    assert!(names.contains(&"list".to_string()));
    assert!(!names.contains(&"seed-data".to_string()), "names: {:?}", names);
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

#[derive(Clone, Default)]
struct GlobalApp {
    verbose: std::cell::Cell<bool>,
}

// Declaring `global = [...]` requires a `CliGlobals` sink (no blanket default impl).
// Globals are delivered *solely* through the sink — the macro no longer auto-fills a
// matching method param (declaring one is now a compile error). This service stashes
// `--verbose` here for `list` to read.
impl server_less::CliGlobals for GlobalApp {
    fn set_global_flag(&self, name: &str, value: bool) {
        if name == "verbose" {
            self.verbose.set(value);
        }
    }
}

#[cli(name = "global-app", global = [verbose, dry_run])]
impl GlobalApp {
    /// List things
    pub fn list(&self) -> Vec<String> {
        if self.verbose.get() {
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
    let app = GlobalApp::default();
    let result = app.cli_run_with(["global-app", "--verbose", "list"]);
    assert!(result.is_ok());
}

#[test]
fn test_global_flag_after_subcommand() {
    let app = GlobalApp::default();
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

// ── Type-mismatch parameter handling ─────────────────────────────────
//
// When `"abc"` is passed for a `u32` parameter, clap's value_parser rejects it
// before dispatch — `try_get_matches_from` returns `Err` rather than silently
// defaulting to 0. We verify via the clap Command directly so that the process
// is not terminated by clap's error-and-exit path.

#[test]
fn test_type_mismatch_u32_param_returns_err() {
    // try_get_matches_from returns Err instead of calling process::exit
    let result = ParamsJsonService::cli_command()
        .try_get_matches_from(["pj-app", "create", "--name", "test", "--count", "abc"]);
    assert!(result.is_err(), "expected Err for unparseable u32, got Ok");
}

#[test]
fn test_type_mismatch_error_message_mentions_param() {
    let result = ParamsJsonService::cli_command()
        .try_get_matches_from(["pj-app", "create", "--name", "test", "--count", "abc"]);
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("count") || msg.contains("invalid digit") || msg.contains("parse"),
        "error message should mention the argument or parse failure, got: {msg}"
    );
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

// ── #[param(name = "...")] flag rename ────────────────────────────────
// The CLI projection must honor the wire-name override on a parameter: the clap
// arg id, the `--long` flag, and extraction all use the declared name, not the
// kebab-cased Rust identifier. (Previously `wire_name` was silently dropped here.)

#[derive(Clone)]
struct ParamNameService;

#[cli(name = "param-name-app")]
impl ParamNameService {
    /// Search
    pub fn search(&self, #[param(name = "q")] query: String) -> String {
        format!("Searching for: {query}")
    }
}

#[test]
fn test_param_name_renames_flag() {
    let cmd = ParamNameService::cli_command();
    let search_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "search")
        .unwrap();

    // The renamed flag exists...
    assert!(
        search_cmd
            .get_arguments()
            .any(|a| a.get_id().as_str() == "q"),
        "expected the renamed `q` arg to exist"
    );
    // ...and the original kebab name does NOT.
    assert!(
        !search_cmd
            .get_arguments()
            .any(|a| a.get_id().as_str() == "query"),
        "the un-renamed `query` arg must not exist"
    );
}

#[test]
fn test_param_name_dispatch_uses_renamed_flag() {
    let svc = ParamNameService;
    // The renamed flag is accepted and dispatches through extraction (which also
    // reads the renamed key — advertisement and extraction stay in lockstep).
    assert!(svc.cli_run_with(["param-name-app", "search", "--q", "hi"]).is_ok());
}

// ── #[param(default = ...)] applied in CLI ────────────────────────────
// A required (non-Option) param carrying `#[param(default = ...)]` must become
// satisfiable from the default — clap supplies it, so omitting the flag no longer
// errors "missing argument". (Previously `default_value` was discarded entirely.)

#[derive(Clone)]
struct ParamDefaultService;

#[cli(name = "param-default-app")]
impl ParamDefaultService {
    /// Connect on a port (defaults to 8080)
    pub fn connect(&self, #[param(default = 8080)] port: u16) -> String {
        format!("port={port}")
    }
}

#[test]
fn test_param_default_sets_clap_default() {
    let cmd = ParamDefaultService::cli_command();
    let connect_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "connect")
        .unwrap();
    let port_arg = connect_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "port")
        .unwrap();

    let defaults: Vec<String> = port_arg
        .get_default_values()
        .iter()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();
    assert_eq!(defaults, vec!["8080".to_string()]);
}

#[test]
fn test_param_default_dispatch_without_arg() {
    let svc = ParamDefaultService;
    // Omitting the required-with-default param succeeds (no "missing argument").
    assert!(svc.cli_run_with(["param-default-app", "connect"]).is_ok());
    // Supplying it still works.
    assert!(svc.cli_run_with(["param-default-app", "connect", "--port", "9090"]).is_ok());
}

// ── CliGlobals delivery (the capability-wiring invariant) ─────────────
// A service that declares `global = [...]` must impl `CliGlobals`; the macro
// delivers each global flag's value to the sink before the method runs. The
// compile-fail counterpart (declaring `global` without the impl) lives in
// tests/fixtures/cli_global_without_sink.rs.

#[derive(Clone, Default)]
struct GlobalSinkApp {
    seen: std::rc::Rc<std::cell::RefCell<Vec<(String, bool)>>>,
}

impl server_less::CliGlobals for GlobalSinkApp {
    fn set_global_flag(&self, name: &str, value: bool) {
        self.seen.borrow_mut().push((name.to_string(), value));
    }
}

#[cli(name = "global-sink-app", global = [verbose, dry_run])]
impl GlobalSinkApp {
    /// Run
    pub fn run(&self) -> String {
        "ok".to_string()
    }
}

#[test]
fn test_cli_globals_delivers_flag_values() {
    let app = GlobalSinkApp::default();
    let result = app.cli_run_with(["global-sink-app", "--verbose", "run"]);
    assert!(result.is_ok());

    let seen = app.seen.borrow();
    // Both declared globals are delivered (kebab-cased), once each, with parsed values.
    assert!(seen.contains(&("verbose".to_string(), true)), "got: {seen:?}");
    assert!(seen.contains(&("dry-run".to_string(), false)), "got: {seen:?}");
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

// Regression: #[cli(name = "...", display_with = "...")] in a single attribute must work.
// Previously get_display_with returned None when name= came before display_with= in the
// same attribute, because the early-return logic exited after the first successful parse
// of the #[cli] attribute even though display_with hadn't been found yet.
struct NameWithDisplay;

impl NameWithDisplay {
    fn fmt_item(&self, s: &String) -> String {
        format!("item: {s}")
    }
}

#[cli(name = "name-display-app")]
impl NameWithDisplay {
    /// Command with both name and display_with
    #[cli(name = "renamed", display_with = "fmt_item")]
    fn original_name(&self) -> String {
        "hello".to_string()
    }
}

// Also test with a Vec return type (no Display on the item — display_with must handle it).
struct NameWithDisplayVec;

#[derive(serde::Serialize, schemars::JsonSchema)]
struct NoDisplayItem {
    value: String,
}

impl NameWithDisplayVec {
    fn fmt_items(&self, items: &[NoDisplayItem]) -> String {
        items.iter().map(|i| i.value.clone()).collect::<Vec<_>>().join("\n")
    }
}

#[cli(name = "name-display-vec-app")]
impl NameWithDisplayVec {
    /// Command with name + display_with returning Vec of non-Display items
    #[cli(name = "renamed", display_with = "fmt_items")]
    fn original_name(&self) -> Vec<NoDisplayItem> {
        vec![NoDisplayItem { value: "hello".into() }]
    }
}

#[test]
fn test_cli_name_and_display_with_combined() {
    let app = NameWithDisplay;
    // The command should be accessible via the renamed CLI name
    assert!(app.cli_run_with(["name-display-app", "renamed"]).is_ok());
}

#[test]
fn test_cli_name_and_display_with_vec_no_display() {
    let app = NameWithDisplayVec;
    assert!(app.cli_run_with(["name-display-vec-app", "renamed"]).is_ok());
}

// Regression: #[cli(default, display_with = "...")] in a single attribute must work.
// Previously get_display_with bailed on unknown keys like `default`, returning None
// even when display_with was present in the same attribute.
struct DefaultWithDisplay;

impl DefaultWithDisplay {
    fn fmt_status(&self, s: &String) -> String {
        format!("status: {s}")
    }
}

#[cli(name = "default-display-app")]
impl DefaultWithDisplay {
    /// Default action with custom display
    #[cli(default, display_with = "fmt_status")]
    fn status(&self) -> String {
        "ok".to_string()
    }
}

#[test]
fn test_cli_default_and_display_with_combined() {
    let app = DefaultWithDisplay;
    assert!(app.cli_run_with(["default-display-app"]).is_ok());
}

#[test]
fn test_cli_default_args_visible_at_root_level() {
    // Args from the default method should appear in the root command's --help,
    // not be hidden. `--verbose` belongs to `run` (the default), so it must be
    // a visible arg on the root command as well.
    let cmd = DefaultWithArgs::cli_command();
    let verbose_arg = cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "verbose");
    assert!(
        verbose_arg.is_some(),
        "`--verbose` should be registered on the root command"
    );
    assert!(
        !verbose_arg.unwrap().is_hide_set(),
        "`--verbose` should be visible in root --help"
    );
}

// ============================================================================
// Multiple positional arguments
// ============================================================================

struct MultiPositionalService;

#[cli(name = "multi-pos-app")]
impl MultiPositionalService {
    /// Copy src to dst
    pub fn copy(
        &self,
        #[param(positional)] src: String,
        #[param(positional)] dst: String,
    ) -> String {
        format!("{src} -> {dst}")
    }
}

#[test]
fn test_multiple_positional_args_have_distinct_indices() {
    let cmd = MultiPositionalService::cli_command();
    let copy_cmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == "copy")
        .unwrap();

    let src_arg = copy_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "src")
        .unwrap();
    let dst_arg = copy_cmd
        .get_arguments()
        .find(|a| a.get_id().as_str() == "dst")
        .unwrap();

    assert_eq!(src_arg.get_index(), Some(1), "src should be index 1");
    assert_eq!(dst_arg.get_index(), Some(2), "dst should be index 2");
}

#[test]
fn test_multiple_positional_args_dispatch() {
    let svc = MultiPositionalService;
    assert!(
        svc.cli_run_with(["multi-pos-app", "copy", "a.txt", "b.txt"])
            .is_ok()
    );
}

// ============================================================================
// Enum parameter: possible values surfaced from JsonSchema
// ============================================================================

#[cfg(feature = "jsonschema")]
mod enum_param_tests {
    use super::*;

    #[derive(
        Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
    )]
    #[serde(rename_all = "snake_case")]
    enum Status {
        Active,
        Inactive,
        Pending,
    }

    impl std::fmt::Display for Status {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Status::Active => write!(f, "active"),
                Status::Inactive => write!(f, "inactive"),
                Status::Pending => write!(f, "pending"),
            }
        }
    }

    impl std::str::FromStr for Status {
        type Err = String;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "active" => Ok(Status::Active),
                "inactive" => Ok(Status::Inactive),
                "pending" => Ok(Status::Pending),
                other => Err(format!("unknown status: {other}")),
            }
        }
    }

    struct StatusService;

    #[cli(name = "status-app")]
    impl StatusService {
        /// Filter items by status
        pub fn filter(&self, status: Status) -> String {
            format!("filtered by {status}")
        }
    }

    #[test]
    fn test_enum_arg_has_possible_values() {
        let cmd = StatusService::cli_command();
        let filter_cmd = cmd
            .get_subcommands()
            .find(|c| c.get_name() == "filter")
            .unwrap();
        let status_arg = filter_cmd
            .get_arguments()
            .find(|a| a.get_id().as_str() == "status")
            .unwrap();

        let pvs = status_arg.get_possible_values();
        let possible: Vec<&str> = pvs.iter().map(|pv| pv.get_name()).collect();

        assert!(
            possible.contains(&"active"),
            "expected 'active' in possible values, got: {possible:?}"
        );
        assert!(
            possible.contains(&"inactive"),
            "expected 'inactive' in possible values, got: {possible:?}"
        );
        assert!(
            possible.contains(&"pending"),
            "expected 'pending' in possible values, got: {possible:?}"
        );
    }

    #[test]
    fn test_enum_arg_dispatches_correctly() {
        let svc = StatusService;
        assert!(
            svc.cli_run_with(["status-app", "filter", "--status", "active"])
                .is_ok()
        );
    }
}

// ============================================================================
// Async dispatch tests
// ============================================================================

#[derive(Clone)]
struct AsyncService {
    log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl AsyncService {
    fn new() -> Self {
        Self {
            log: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
        }
    }
    fn logged(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

#[cli(name = "async-app")]
impl AsyncService {
    /// Async method returning a value
    pub async fn ping(&self) -> String {
        self.log.lock().unwrap().push("ping".to_string());
        "pong".to_string()
    }

    /// Async method with an argument
    pub async fn echo(&self, msg: String) -> String {
        self.log.lock().unwrap().push(format!("echo:{msg}"));
        msg
    }

    /// Sync method on the same service
    pub fn version(&self) -> String {
        "1.0".to_string()
    }
}

#[tokio::test]
async fn test_async_dispatch_via_run_with_async() {
    let svc = AsyncService::new();
    let result = svc.cli_run_with_async(["async-app", "ping"]).await;
    assert!(result.is_ok());
    assert_eq!(svc.logged(), vec!["ping"]);
}

#[tokio::test]
async fn test_async_dispatch_with_arg() {
    let svc = AsyncService::new();
    let result = svc
        .cli_run_with_async(["async-app", "echo", "--msg", "hello"])
        .await;
    assert!(result.is_ok());
    assert_eq!(svc.logged(), vec!["echo:hello"]);
}

#[tokio::test]
async fn test_sync_method_via_async_dispatch() {
    // Sync methods work fine through the async dispatch path
    let svc = AsyncService::new();
    let result = svc.cli_run_with_async(["async-app", "version"]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_async_trait_dispatch_directly() {
    use server_less::CliSubcommand;
    let svc = AsyncService::new();
    let matches = AsyncService::cli_command().get_matches_from(["async-app", "ping"]);
    let result = <AsyncService as CliSubcommand>::cli_dispatch_async(&svc, &matches).await;
    assert!(result.is_ok());
    assert_eq!(svc.logged(), vec!["ping"]);
}

// Async mount point: child with async methods mounted under a parent.

#[derive(Clone)]
struct AsyncChild {
    log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl AsyncChild {
    fn new_shared(log: std::sync::Arc<std::sync::Mutex<Vec<String>>>) -> Self {
        Self { log }
    }
}

#[cli(name = "async-child")]
impl AsyncChild {
    pub async fn work(&self) -> String {
        self.log.lock().unwrap().push("work".to_string());
        "done".to_string()
    }
}

#[derive(Clone)]
struct AsyncParent {
    child: AsyncChild,
}

impl AsyncParent {
    fn new() -> Self {
        let log = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        Self {
            child: AsyncChild::new_shared(log),
        }
    }
    fn logged(&self) -> Vec<String> {
        self.child.log.lock().unwrap().clone()
    }
}

#[cli(name = "async-parent")]
impl AsyncParent {
    pub async fn local(&self) -> String {
        "local".to_string()
    }

    pub fn child(&self) -> &AsyncChild {
        &self.child
    }
}

#[tokio::test]
async fn test_async_mount_local_method() {
    let app = AsyncParent::new();
    assert!(app.cli_run_with_async(["async-parent", "local"]).await.is_ok());
}

#[tokio::test]
async fn test_async_mount_child_dispatch() {
    let app = AsyncParent::new();
    assert!(
        app.cli_run_with_async(["async-parent", "child", "work"])
            .await
            .is_ok()
    );
    assert_eq!(app.logged(), vec!["work"]);
}

// no_sync: only async entrypoints generated.

#[derive(Clone)]
struct NoSyncService;

#[cli(name = "no-sync-app", no_sync)]
impl NoSyncService {
    pub async fn run(&self) -> String {
        "ran".to_string()
    }
}

#[tokio::test]
async fn test_no_sync_async_entrypoint_works() {
    let svc = NoSyncService;
    assert!(svc.cli_run_with_async(["no-sync-app", "run"]).await.is_ok());
}

// no_async: only sync entrypoints generated.

#[derive(Clone)]
struct NoAsyncService;

#[cli(name = "no-async-app", no_async)]
impl NoAsyncService {
    pub fn run(&self) -> String {
        "ran".to_string()
    }
}

#[test]
fn test_no_async_sync_entrypoint_works() {
    let svc = NoAsyncService;
    assert!(svc.cli_run_with(["no-async-app", "run"]).is_ok());
}

// ─── Async return type coverage ─────────────────────────────────────────────
//
// Tests for async methods with Result<T,E>, Option<T>, and () return types
// which each have distinct codegen branches in generate_async_method_call.

#[derive(Clone)]
struct AsyncReturnsService;

#[cli(name = "async-returns")]
impl AsyncReturnsService {
    pub async fn ok_value(&self) -> Result<String, String> {
        Ok("success".to_string())
    }

    pub async fn err_value(&self) -> Result<String, String> {
        Err("boom".to_string())
    }

    pub async fn some_value(&self) -> Option<String> {
        Some("found".to_string())
    }

    pub async fn none_value(&self) -> Option<String> {
        None
    }

    pub async fn unit_method(&self) {}
}

#[tokio::test]
async fn test_async_result_ok() {
    let svc = AsyncReturnsService;
    let result = svc.cli_run_with_async(["async-returns", "ok-value"]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_async_option_some() {
    let svc = AsyncReturnsService;
    let result = svc
        .cli_run_with_async(["async-returns", "some-value"])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_async_option_none_with_json_flag() {
    // Without --json, None calls process::exit(1). With --json, None outputs null.
    let svc = AsyncReturnsService;
    let result = svc
        .cli_run_with_async(["async-returns", "--json", "none-value"])
        .await;
    assert!(result.is_ok());
}

// Note: err_value() calls process::exit(1) which cannot be caught in tests.
// The fact that it compiles verifies async Result<T,E> codegen is correct.

#[tokio::test]
async fn test_async_unit_return() {
    let svc = AsyncReturnsService;
    let result = svc
        .cli_run_with_async(["async-returns", "unit-method"])
        .await;
    assert!(result.is_ok());
}

// ─── Async slug mount dispatch ───────────────────────────────────────────────
//
// Tests generate_slug_mount_arm_async at runtime (previously compile-time only).

#[derive(Clone)]
struct SlugChild {
    prefix: String,
}

impl SlugChild {
    fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

#[cli(name = "slug-child")]
impl SlugChild {
    pub async fn hello(&self) -> String {
        format!("{}_hello", self.prefix)
    }
}

#[derive(Clone)]
struct SlugParent {
    children: std::collections::HashMap<String, SlugChild>,
}

impl SlugParent {
    fn new() -> Self {
        let mut children = std::collections::HashMap::new();
        children.insert("abc".to_string(), SlugChild::new("abc"));
        children.insert("def".to_string(), SlugChild::new("def"));
        Self { children }
    }
}

#[cli(name = "slug-parent")]
impl SlugParent {
    pub fn section(&self, id: String) -> &SlugChild {
        self.children
            .get(&id)
            .expect("BUG: test uses known key in slug-parent dispatch")
    }
}

#[tokio::test]
async fn test_async_slug_mount_dispatch() {
    let app = SlugParent::new();
    let result = app
        .cli_run_with_async(["slug-parent", "section", "abc", "hello"])
        .await;
    assert!(result.is_ok(), "slug dispatch failed: {:?}", result);
}

#[tokio::test]
async fn test_async_slug_mount_dispatch_different_slug() {
    let app = SlugParent::new();
    let result = app
        .cli_run_with_async(["slug-parent", "section", "def", "hello"])
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cli_run_with_inside_tokio_returns_err() {
    let svc = ItemService::new();
    let result = svc.cli_run_with(["item-cli", "list-items"]);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("async"),
        "expected error to mention 'async', got: {msg}"
    );
}

// ============================================================================
// Async dispatch + output formatting flag tests
// ============================================================================
//
// Cover --json, --jq, --output-schema, and --params-json going through the
// async dispatch path (cli_run_with_async).

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct StatusInfo {
    code: u32,
    message: String,
}

impl std::fmt::Display for StatusInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[derive(Clone)]
struct AsyncFormattingService;

#[cli(name = "async-fmt-app")]
impl AsyncFormattingService {
    /// Return a structured status object
    pub async fn status(&self) -> StatusInfo {
        StatusInfo {
            code: 200,
            message: "ok".to_string(),
        }
    }

    /// Echo a message with a count
    pub async fn report(&self, label: String, count: u32) -> StatusInfo {
        StatusInfo {
            code: count,
            message: label,
        }
    }
}

#[tokio::test]
async fn test_async_json_flag() {
    let svc = AsyncFormattingService;
    let result = svc
        .cli_run_with_async(["async-fmt-app", "--json", "status"])
        .await;
    assert!(result.is_ok(), "async --json dispatch failed: {:?}", result);
}

#[tokio::test]
async fn test_async_jq_flag() {
    let svc = AsyncFormattingService;
    let result = svc
        .cli_run_with_async(["async-fmt-app", "--jq", ".message", "status"])
        .await;
    assert!(
        result.is_ok(),
        "async --jq dispatch failed: {:?}",
        result
    );
}

#[tokio::test]
async fn test_async_output_schema_flag() {
    let svc = AsyncFormattingService;
    let result = svc
        .cli_run_with_async(["async-fmt-app", "--output-schema", "status"])
        .await;
    assert!(
        result.is_ok(),
        "async --output-schema dispatch failed: {:?}",
        result
    );
}

// --- description_prefix tests ---

#[derive(Clone)]
struct NamePrefixService;

#[cli(name = "my-tool", description = "Does useful things")]
impl NamePrefixService {
    /// A placeholder subcommand
    fn run(&self) {}
}

#[derive(Clone)]
struct NoPrefixService;

#[cli(name = "my-tool", description = "Does useful things", description_prefix = false)]
impl NoPrefixService {
    /// A placeholder subcommand
    fn run(&self) {}
}

#[test]
fn test_description_prefix_default() {
    let cmd = NamePrefixService::cli_command();
    let about = cmd.get_about().map(|s| s.to_string()).unwrap_or_default();
    assert_eq!(about, "my-tool - Does useful things");
}

#[test]
fn test_description_prefix_false() {
    let cmd = NoPrefixService::cli_command();
    let about = cmd.get_about().map(|s| s.to_string()).unwrap_or_default();
    assert_eq!(about, "Does useful things");
}

#[tokio::test]
async fn test_async_params_json_flag() {
    let svc = AsyncFormattingService;
    let result = svc
        .cli_run_with_async([
            "async-fmt-app",
            "--params-json",
            r#"{"label":"test","count":42}"#,
            "report",
        ])
        .await;
    assert!(
        result.is_ok(),
        "async --params-json dispatch failed: {:?}",
        result
    );
}

// ─── --manual (whole-subtree reference) ──────────────────────────────────────

// App with a leaf, a static mount, and a nested static mount (depth 3) to
// exercise whole-subtree aggregation and prefixing.
#[derive(Clone)]
struct ManualApp {
    nested: NestedPostService,
}

#[cli(name = "manual-app", version = "1.0.0", description = "Manual demo")]
impl ManualApp {
    /// Health check
    pub fn health(&self) -> String {
        "ok".to_string()
    }

    /// Mount post commands (which themselves mount comments)
    pub fn posts(&self) -> &NestedPostService {
        &self.nested
    }
}

#[test]
fn test_manual_flag_present_on_command() {
    let cmd = ManualApp::cli_command();
    let has_manual = cmd.get_arguments().any(|a| a.get_id() == "manual");
    assert!(has_manual, "expected a --manual global flag on the root command");
}

#[test]
fn test_manual_nodes_aggregate_whole_tree() {
    let app = ManualApp {
        nested: NestedPostService,
    };
    let nodes = app.cli_manual_nodes("");
    let paths: Vec<&str> = nodes.iter().map(|n| n.path.as_str()).collect();

    // Root leaf
    assert!(paths.contains(&"health"), "paths: {:?}", paths);
    // Mount child leaf — prefixed with mount name
    assert!(paths.contains(&"posts list"), "paths: {:?}", paths);
    // Nested mount grandchild leaf — depth-3 prefix
    assert!(
        paths.contains(&"posts comments list"),
        "paths: {:?}",
        paths
    );
}

#[test]
fn test_manual_nodes_carry_description_and_schemas() {
    let app = ManualApp {
        nested: NestedPostService,
    };
    let nodes = app.cli_manual_nodes("");
    let health = nodes.iter().find(|n| n.path == "health").unwrap();
    assert_eq!(health.description.as_deref(), Some("Health check"));
    // health() -> String, no params
    assert_eq!(health.input_schema["type"], "object");
    assert!(
        health
            .input_schema
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.is_empty())
            .unwrap_or(false),
        "health has no params: {:?}",
        health.input_schema
    );
}

#[test]
fn test_manual_subtree_scoping_via_prefix() {
    // The mount child's own cli_manual_nodes, as invoked when descending into it,
    // yields the subtree rooted at that node (relative to the supplied prefix).
    let nested = NestedPostService;
    let nodes = nested.cli_manual_nodes("posts");
    let paths: Vec<&str> = nodes.iter().map(|n| n.path.as_str()).collect();
    assert!(paths.contains(&"posts list"), "paths: {:?}", paths);
    assert!(
        paths.contains(&"posts comments list"),
        "paths: {:?}",
        paths
    );
    // The root-level `health` is NOT part of this subtree.
    assert!(!paths.contains(&"health"), "paths: {:?}", paths);
}

#[test]
fn test_manual_to_json_keyed_by_path() {
    let app = ManualApp {
        nested: NestedPostService,
    };
    let nodes = app.cli_manual_nodes("");
    let doc = server_less::cli_manual_to_json(&nodes);
    let obj = doc.as_object().expect("manual json is an object");
    assert!(obj.contains_key("health"));
    assert!(obj.contains_key("posts list"));
    assert!(obj.contains_key("posts comments list"));
    assert_eq!(obj["health"]["description"], "Health check");
    assert!(obj["health"]["input_schema"].is_object());
    assert!(obj["health"]["output_schema"].is_object());
}

#[test]
fn test_manual_dispatch_runs_ok() {
    let app = ManualApp {
        nested: NestedPostService,
    };
    // Whole-tree manual (text default).
    assert!(app.cli_run_with(["manual-app", "--manual"]).is_ok());
    // Whole-tree manual (json).
    assert!(app.cli_run_with(["manual-app", "--manual", "--json"]).is_ok());
    // Subtree scoping: `posts --manual`.
    assert!(app.cli_run_with(["manual-app", "posts", "--manual"]).is_ok());
    // Single leaf entry: `health --manual`.
    assert!(app.cli_run_with(["manual-app", "health", "--manual"]).is_ok());
    // NOTE: `--manual --jq '...'` composes with the existing `--jq` machinery
    // (cli_format_output), so it is exercised by the format flag wiring rather
    // than re-tested here. The `--jq` loader is currently broken for ALL surfaces
    // in this build (a pre-existing jaq beta std-defs compile failure, unrelated
    // to --manual); see test_manual_to_json for the structured shape coverage.
}

// ─── meta-surface toggles ────────────────────────────────────────────────────

#[derive(Clone)]
struct ToggleApp;

#[cli(name = "toggle-app", manual = false, input_schema = false, output_schema = false)]
impl ToggleApp {
    /// A plain leaf
    pub fn ping(&self) -> String {
        "pong".to_string()
    }
}

#[test]
fn test_disabled_meta_flags_absent_from_command() {
    let cmd = ToggleApp::cli_command();
    let ids: Vec<String> = cmd.get_arguments().map(|a| a.get_id().to_string()).collect();
    // The disabled meta-surfaces are gone …
    assert!(!ids.contains(&"manual".to_string()), "ids: {ids:?}");
    assert!(!ids.contains(&"input-schema".to_string()), "ids: {ids:?}");
    assert!(!ids.contains(&"output-schema".to_string()), "ids: {ids:?}");
    // … but the always-on format flags remain.
    assert!(ids.contains(&"json".to_string()), "ids: {ids:?}");
    assert!(ids.contains(&"params-json".to_string()), "ids: {ids:?}");
}

#[test]
fn test_disabled_manual_does_not_panic_at_dispatch() {
    // With `--manual` unregistered, dispatch must not call get_flag("manual")
    // (which would panic). A normal subcommand invocation should just work.
    let app = ToggleApp;
    assert!(app.cli_run_with(["toggle-app", "ping"]).is_ok());
}

// A parameter literally named `manual` is legal once the manual surface is off.
#[derive(Clone)]
struct ManualParamApp;

#[cli(name = "manual-param-app", manual = false)]
impl ManualParamApp {
    pub fn render(&self, manual: String) -> String {
        manual
    }
}

#[test]
fn test_manual_param_legal_when_surface_disabled() {
    let app = ManualParamApp;
    // `--manual` now binds to the user's parameter, not the reference surface.
    assert!(
        app.cli_run_with(["manual-param-app", "render", "--manual", "hi"])
            .is_ok()
    );
}

// Per-command `#[cli(manual = false)]` hides one leaf from the aggregate while
// keeping the command runnable.
#[derive(Clone)]
struct PartialManualApp;

#[cli(name = "partial-manual-app")]
impl PartialManualApp {
    /// Public command
    pub fn visible(&self) -> String {
        "v".to_string()
    }

    /// Internal command — kept out of the manual
    #[cli(manual = false)]
    pub fn internal(&self) -> String {
        "i".to_string()
    }
}

#[test]
fn test_per_command_manual_false_excludes_from_aggregate() {
    let app = PartialManualApp;
    let nodes = app.cli_manual_nodes("");
    let paths: Vec<&str> = nodes.iter().map(|n| n.path.as_str()).collect();
    assert!(paths.contains(&"visible"), "paths: {paths:?}");
    assert!(!paths.contains(&"internal"), "paths: {paths:?}");
    // The command itself is still dispatchable.
    assert!(app.cli_run_with(["partial-manual-app", "internal"]).is_ok());
}
