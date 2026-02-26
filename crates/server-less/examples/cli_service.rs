//! Example CLI service demonstrating the #[cli] macro.
//!
//! # Flat commands
//!
//! ```bash
//! cargo run --example cli_service -- users list-users
//! cargo run --example cli_service -- users create-user --name "Bob" --email "bob@test.com"
//! ```
//!
//! # Mounted subcommand groups
//!
//! ```bash
//! cargo run --example cli_service -- admin users list-users
//! cargo run --example cli_service -- admin health
//! ```

use serde::{Deserialize, Serialize};
use server_less::cli;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug)]
pub enum UserError {
    NotFound,
    InvalidEmail,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::NotFound => write!(f, "User not found"),
            UserError::InvalidEmail => write!(f, "Invalid email"),
        }
    }
}

impl std::error::Error for UserError {}

#[derive(Clone)]
pub struct UserService {
    users: std::sync::Arc<std::sync::Mutex<Vec<User>>>,
}

impl Default for UserService {
    fn default() -> Self {
        Self::new()
    }
}

impl UserService {
    pub fn new() -> Self {
        Self {
            users: std::sync::Arc::new(std::sync::Mutex::new(vec![User {
                id: "1".to_string(),
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            }])),
        }
    }
}

#[cli(name = "user-cli", version = "0.1.0", about = "Manage users")]
impl UserService {
    /// List all users
    pub fn list_users(&self) -> Vec<User> {
        self.users.lock().unwrap().clone()
    }

    /// Create a new user
    pub fn create_user(&self, name: String, email: String) -> Result<User, UserError> {
        if !email.contains('@') {
            return Err(UserError::InvalidEmail);
        }
        let mut users = self.users.lock().unwrap();
        let user = User {
            id: (users.len() + 1).to_string(),
            name,
            email,
        };
        users.push(user.clone());
        Ok(user)
    }

    /// Get user by ID
    pub fn get_user(&self, user_id: String) -> Option<User> {
        let users = self.users.lock().unwrap();
        users.iter().find(|u| u.id == user_id).cloned()
    }

    /// Delete a user
    pub fn delete_user(&self, user_id: String) -> Result<(), UserError> {
        let mut users = self.users.lock().unwrap();
        let initial_len = users.len();
        users.retain(|u| u.id != user_id);
        if users.len() == initial_len {
            Err(UserError::NotFound)
        } else {
            Ok(())
        }
    }
}

// --- Mount point example: compose services into a parent CLI ---

#[derive(Default)]
pub struct AdminApp {
    users: UserService,
}

#[cli(name = "admin", version = "0.1.0", about = "Admin panel")]
impl AdminApp {
    /// Check system health
    pub fn health(&self) -> String {
        "ok".to_string()
    }

    /// User management commands
    pub fn users(&self) -> &UserService {
        &self.users
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = AdminApp::default();
    app.cli_run()
}
