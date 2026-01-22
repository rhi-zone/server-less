//! Example HTTP service demonstrating the #[http] macro.

use serde::{Deserialize, Serialize};
use server_less::http;

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

#[http(prefix = "/api")]
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
}

#[tokio::main]
async fn main() {
    let service = UserService::new();

    // Create router
    let app = service.http_router();

    // Print OpenAPI spec
    println!("OpenAPI spec:");
    println!(
        "{}",
        serde_json::to_string_pretty(&UserService::openapi_spec()).unwrap()
    );

    println!("\nStarting server on http://localhost:3000");
    println!("Try: curl http://localhost:3000/api/users");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
