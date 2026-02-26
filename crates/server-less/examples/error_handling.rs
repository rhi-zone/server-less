//! Example: ServerlessError in an HTTP service.
//!
//! Shows how `#[derive(ServerlessError)]` maps error variants to HTTP status codes
//! automatically. Error codes are inferred from variant names or set explicitly.
//!
//! Run: cargo run --example error_handling

use serde::{Deserialize, Serialize};
use server_less::{IntoErrorCode, ServerlessError, http};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}

/// Error type with automatic HTTP status mapping.
///
/// - `NotFound` → 404 (inferred from name)
/// - `InvalidInput` → 400 (inferred from name)
/// - `Unauthorized` → 401 (inferred from name)
/// - `RateLimited` → 429 (explicit code)
#[derive(Debug, ServerlessError)]
enum TaskError {
    NotFound,
    #[error(code = InvalidInput, message = "Title cannot be empty")]
    EmptyTitle,
    Unauthorized,
    #[error(code = 429)]
    RateLimited,
}

#[derive(Clone)]
struct TaskService {
    tasks: std::sync::Arc<std::sync::Mutex<Vec<Task>>>,
}

impl TaskService {
    fn new() -> Self {
        Self {
            tasks: std::sync::Arc::new(std::sync::Mutex::new(vec![Task {
                id: 1,
                title: "Write docs".to_string(),
                done: false,
            }])),
        }
    }
}

#[http(prefix = "/api")]
impl TaskService {
    /// List all tasks
    pub fn list_tasks(&self) -> Vec<Task> {
        self.tasks.lock().unwrap().clone()
    }

    /// Get task by ID (returns 404 if missing)
    pub fn get_task(&self, task_id: u64) -> Result<Task, TaskError> {
        self.tasks
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == task_id)
            .cloned()
            .ok_or(TaskError::NotFound)
    }

    /// Create a task (returns 400 if title is empty)
    pub fn create_task(&self, title: String) -> Result<Task, TaskError> {
        if title.is_empty() {
            return Err(TaskError::EmptyTitle);
        }
        let mut tasks = self.tasks.lock().unwrap();
        let task = Task {
            id: tasks.len() as u64 + 1,
            title,
            done: false,
        };
        tasks.push(task.clone());
        Ok(task)
    }

    /// Delete a task
    pub fn delete_task(&self, task_id: u64) -> Result<(), TaskError> {
        let mut tasks = self.tasks.lock().unwrap();
        let len_before = tasks.len();
        tasks.retain(|t| t.id != task_id);
        if tasks.len() == len_before {
            Err(TaskError::NotFound)
        } else {
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() {
    let service = TaskService::new();

    // Show error code mappings
    use server_less::IntoErrorCode;
    println!("Error code mappings:");
    println!(
        "  NotFound    → HTTP {}",
        TaskError::NotFound.error_code().http_status()
    );
    println!(
        "  EmptyTitle  → HTTP {}",
        TaskError::EmptyTitle.error_code().http_status()
    );
    println!(
        "  Unauthorized→ HTTP {}",
        TaskError::Unauthorized.error_code().http_status()
    );
    println!(
        "  RateLimited → HTTP {}",
        TaskError::RateLimited.error_code().http_status()
    );

    // Print OpenAPI spec
    println!("\nOpenAPI spec:");
    println!(
        "{}",
        serde_json::to_string_pretty(&TaskService::openapi_spec()).unwrap()
    );

    let app = service.http_router();
    println!("\nStarting server on http://localhost:3000");
    println!("Try:");
    println!("  curl localhost:3000/api/tasks");
    println!("  curl localhost:3000/api/tasks/1");
    println!("  curl localhost:3000/api/tasks/999        # 404");
    println!(
        "  curl -X POST -H 'Content-Type: application/json' -d '{{\"title\":\"\"}}' localhost:3000/api/tasks  # 400"
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
