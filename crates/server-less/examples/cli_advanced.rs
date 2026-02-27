//! Advanced CLI features: global flags, defaults, display_with, skip.
//!
//! # Basic usage
//!
//! ```bash
//! cargo run --example cli_advanced -- list-items
//! cargo run --example cli_advanced -- --verbose list-items
//! cargo run --example cli_advanced -- connect --host db.example.com
//! ```
//!
//! # JSON and jq output
//!
//! ```bash
//! cargo run --example cli_advanced -- --json list-items
//! cargo run --example cli_advanced -- --jq '.[0].name' list-items
//! ```
//!
//! # Schema introspection
//!
//! ```bash
//! cargo run --example cli_advanced -- --output-schema list-items
//! cargo run --example cli_advanced -- --input-schema connect
//! ```
//!
//! # Params as JSON
//!
//! ```bash
//! cargo run --example cli_advanced -- connect --params-json '{"host":"db.example.com","port":"3306"}'
//! ```

use serde::{Deserialize, Serialize};
use server_less::cli;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Item {
    pub name: String,
    pub category: String,
}

impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.category)
    }
}

#[derive(Clone)]
pub struct AdvancedApp;

impl AdvancedApp {
    /// Runtime defaults for required parameters not provided on the command line.
    /// Called with the kebab-case parameter name; return Some to supply a fallback.
    fn get_defaults(&self, key: &str) -> Option<String> {
        match key {
            "host" => Some("localhost".to_string()),
            "port" => Some("5432".to_string()),
            _ => None,
        }
    }

    /// Custom display formatter for list_items output.
    /// Receives a reference to the return value; only used for text output
    /// (--json/--jq bypass this and serialize via serde).
    #[allow(clippy::ptr_arg)] // signature must match generated code expectations
    fn format_items(&self, items: &Vec<Item>) -> String {
        items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("  {}. {} [{}]", i + 1, item.name, item.category))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Internal helper — not exposed as a subcommand.
    fn seed_data(&self) -> Vec<Item> {
        vec![
            Item {
                name: "Widget".to_string(),
                category: "hardware".to_string(),
            },
            Item {
                name: "Gadget".to_string(),
                category: "hardware".to_string(),
            },
            Item {
                name: "Script".to_string(),
                category: "software".to_string(),
            },
        ]
    }
}

#[cli(
    name = "advanced-cli",
    version = "0.1.0",
    about = "Demo of advanced CLI features",
    global = [verbose, debug],
    defaults = "get_defaults"
)]
impl AdvancedApp {
    /// List all items with custom formatting
    #[cli(display_with = "format_items")]
    pub fn list_items(&self, verbose: bool) -> Vec<Item> {
        let items = self.seed_data();
        if verbose {
            eprintln!("[verbose] returning {} items", items.len());
        }
        items
    }

    /// Connect to a database (host and port have runtime defaults)
    pub fn connect(&self, host: String, port: u16, verbose: bool) -> String {
        if verbose {
            eprintln!("[verbose] connecting to {}:{}", host, port);
        }
        format!("Connected to {}:{}", host, port)
    }

    /// Internal helper — skipped from CLI subcommands
    #[cli(skip)]
    pub fn internal_status(&self) -> String {
        "ok".to_string()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = AdvancedApp;
    app.cli_run()
}
