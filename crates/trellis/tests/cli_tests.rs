//! Integration tests for the CLI macro.

use serde::{Deserialize, Serialize};
use trellis::cli;

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
