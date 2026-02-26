//! Integration tests for the CLI macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use server_less::{CliSubcommand, cli};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Item {
    id: String,
    name: String,
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
