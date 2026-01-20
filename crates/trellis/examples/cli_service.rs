//! Example CLI service demonstrating the #[cli] macro.

use serde::{Deserialize, Serialize};
use trellis::cli;

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

impl UserService {
    pub fn new() -> Self {
        Self {
            users: std::sync::Arc::new(std::sync::Mutex::new(vec![
                User {
                    id: "1".to_string(),
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                },
            ])),
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = UserService::new();
    service.cli_run()
}
