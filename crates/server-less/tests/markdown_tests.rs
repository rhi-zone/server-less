//! Integration tests for the Markdown documentation generation macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::markdown;

#[derive(Clone)]
struct UserService;

#[markdown(title = "User Service API")]
impl UserService {
    /// Create a new user account
    pub fn create_user(&self, name: String, email: String) -> String {
        name
    }

    /// Get a user by their unique ID
    pub fn get_user(&self, id: String) -> Option<String> {
        Some(id)
    }

    /// List all users with optional pagination
    pub fn list_users(&self, limit: Option<u32>) -> Vec<String> {
        vec![]
    }

    /// Delete a user account
    pub fn delete_user(&self, id: String) -> bool {
        true
    }
}

#[test]
fn test_markdown_title() {
    let docs = UserService::markdown_docs();
    assert!(docs.contains("# User Service API"));
}

#[test]
fn test_markdown_methods_section() {
    let docs = UserService::markdown_docs();
    assert!(docs.contains("## Methods"));
}

#[test]
fn test_markdown_method_names() {
    let docs = UserService::markdown_docs();
    assert!(docs.contains("### Create User"));
    assert!(docs.contains("### Get User"));
    assert!(docs.contains("### List Users"));
    assert!(docs.contains("### Delete User"));
}

#[test]
fn test_markdown_doc_comments() {
    let docs = UserService::markdown_docs();
    assert!(docs.contains("Create a new user account"));
    assert!(docs.contains("Get a user by their unique ID"));
}

#[test]
fn test_markdown_parameters() {
    let docs = UserService::markdown_docs();
    assert!(docs.contains("`name`"));
    assert!(docs.contains("`email`"));
    assert!(docs.contains("(optional)"));
}

#[test]
fn test_markdown_code_blocks() {
    let docs = UserService::markdown_docs();
    assert!(docs.contains("```"));
    assert!(docs.contains("create_user("));
}

// Test default title
#[derive(Clone)]
struct SimpleService;

#[markdown]
impl SimpleService {
    pub fn ping(&self) -> String {
        "pong".to_string()
    }
}

#[test]
fn test_markdown_default_title() {
    let docs = SimpleService::markdown_docs();
    assert!(docs.contains("# SimpleService API"));
}

// Test async methods
#[derive(Clone)]
struct AsyncService;

#[markdown]
impl AsyncService {
    /// Fetch data asynchronously
    pub async fn fetch_data(&self) -> String {
        "data".to_string()
    }
}

#[test]
fn test_markdown_async_badge() {
    let docs = AsyncService::markdown_docs();
    assert!(docs.contains("*async*"));
}

// Test without types
#[derive(Clone)]
struct NoTypesService;

#[markdown(types = false)]
impl NoTypesService {
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[test]
fn test_markdown_no_types() {
    let docs = NoTypesService::markdown_docs();
    // Should have parameter names but not types
    assert!(docs.contains("add(a, b)"));
    // Should not have i32 in signature
    assert!(!docs.contains("i32"));
}

// Test return type descriptions
#[derive(Clone)]
struct TypesService;

#[markdown]
impl TypesService {
    pub fn get_string(&self) -> String {
        "test".to_string()
    }

    pub fn get_optional(&self) -> Option<String> {
        None
    }

    pub fn get_result(&self) -> Result<String, String> {
        Ok("ok".to_string())
    }

    pub fn get_list(&self) -> Vec<String> {
        vec![]
    }

    pub fn get_bool(&self) -> bool {
        true
    }

    pub fn get_number(&self) -> i32 {
        42
    }
}

#[test]
fn test_markdown_return_descriptions() {
    let docs = TypesService::markdown_docs();
    assert!(docs.contains("String"));
    assert!(docs.contains("Optional value"));
    assert!(docs.contains("Result"));
    assert!(docs.contains("Array"));
    assert!(docs.contains("Boolean"));
    assert!(docs.contains("Integer"));
}
