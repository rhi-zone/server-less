//! Example demonstrating the #[program] blessed preset.
//!
//! `#[program]` = `#[cli]` + `#[markdown]` in one attribute.
//!
//! ```bash
//! cargo run --example program_preset -- create-user --name "Alice"
//! cargo run --example program_preset -- list-users
//! ```

use server_less::program;

pub struct MyApp;

#[program(name = "myctl", version = "1.0.0", about = "Example CLI application")]
impl MyApp {
    /// Create a new user
    pub fn create_user(&self, name: String) {
        println!("Created user: {}", name);
    }

    /// List all users
    pub fn list_users(&self) {
        println!("Users:");
        println!("  1. Alice");
        println!("  2. Bob");
    }

    /// Delete a user by ID
    pub fn delete_user(&self, user_id: u32) -> Result<(), String> {
        if user_id == 0 {
            Err("user ID 0 is reserved".into())
        } else {
            println!("Deleted user {}", user_id);
            Ok(())
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = MyApp;

    // Print generated markdown docs
    println!("--- Markdown docs ---");
    println!("{}", MyApp::markdown_docs());
    println!("--- End docs ---\n");

    app.cli_run()
}
