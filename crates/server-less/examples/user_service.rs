//! Example user service demonstrating server-less macros.
//!
//! This example shows how to use the #[mcp] macro to generate MCP tools.

use serde::{Deserialize, Serialize};
use server_less::mcp;

// --- Domain types ---

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
    AlreadyExists,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::NotFound => write!(f, "User not found"),
            UserError::InvalidEmail => write!(f, "Invalid email address"),
            UserError::AlreadyExists => write!(f, "User already exists"),
        }
    }
}

impl std::error::Error for UserError {}

// --- Service implementation ---

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
            users: std::sync::Arc::new(std::sync::Mutex::new(vec![
                User {
                    id: "1".to_string(),
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                },
                User {
                    id: "2".to_string(),
                    name: "Bob".to_string(),
                    email: "bob@example.com".to_string(),
                },
            ])),
        }
    }
}

// Apply MCP macro to generate tool definitions and dispatcher
#[mcp(namespace = "user")]
impl UserService {
    /// Create a new user
    pub fn create_user(&self, name: String, email: String) -> Result<User, UserError> {
        if !email.contains('@') {
            return Err(UserError::InvalidEmail);
        }

        let mut users = self.users.lock().unwrap();

        if users.iter().any(|u| u.email == email) {
            return Err(UserError::AlreadyExists);
        }

        let user = User {
            id: (users.len() + 1).to_string(),
            name,
            email,
        };

        users.push(user.clone());
        Ok(user)
    }

    /// Get a user by ID
    pub fn get_user(&self, user_id: String) -> Option<User> {
        let users = self.users.lock().unwrap();
        users.iter().find(|u| u.id == user_id).cloned()
    }

    /// List all users
    pub fn list_users(&self) -> Vec<User> {
        let users = self.users.lock().unwrap();
        users.clone()
    }

    /// Delete a user by ID
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

    /// Search users by name
    pub fn search_users(&self, query: String, limit: Option<u32>) -> Vec<User> {
        let users = self.users.lock().unwrap();
        let limit = limit.unwrap_or(10) as usize;

        users
            .iter()
            .filter(|u| u.name.to_lowercase().contains(&query.to_lowercase()))
            .take(limit)
            .cloned()
            .collect()
    }
}

fn main() {
    let service = UserService::new();

    // 1. Show available MCP tools
    println!("=== MCP Tools ===");
    let tools = UserService::mcp_tools();
    println!("{}", serde_json::to_string_pretty(&tools).unwrap());

    // 2. Call list_users tool
    println!("\n=== Calling list_users ===");
    let result = service.mcp_call("user_list_users", serde_json::json!({}));
    println!("Result: {:?}", result);

    // 3. Call create_user tool
    println!("\n=== Calling create_user ===");
    let args = serde_json::json!({
        "name": "Charlie",
        "email": "charlie@example.com"
    });
    let result = service.mcp_call("user_create_user", args);
    println!("Result: {:?}", result);

    // 4. Call search_users tool
    println!("\n=== Calling search_users ===");
    let args = serde_json::json!({
        "query": "alice",
        "limit": 5
    });
    let result = service.mcp_call("user_search_users", args);
    println!("Result: {:?}", result);

    // 5. Call get_user tool
    println!("\n=== Calling get_user ===");
    let args = serde_json::json!({
        "user_id": "1"
    });
    let result = service.mcp_call("user_get_user", args);
    println!("Result: {:?}", result);

    println!("\n=== Done ===");
}
