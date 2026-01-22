//! Integration tests for the Connect protocol schema generation macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::connect;

#[derive(Clone)]
struct UserService;

#[connect(package = "users.v1")]
impl UserService {
    /// Get user by ID
    pub fn get_user(&self, id: String) -> String {
        id
    }

    /// List all users
    pub fn list_users(&self) -> Vec<String> {
        vec![]
    }

    /// Create a new user
    pub fn create_user(&self, name: String, email: String) -> String {
        name
    }

    /// Delete a user
    pub fn delete_user(&self, id: String) -> bool {
        true
    }

    /// Update user email
    pub fn update_user(&self, id: String, email: Option<String>) -> String {
        id
    }
}

#[test]
fn test_connect_schema_generated() {
    let schema = UserService::connect_schema();

    // Check package
    assert!(schema.contains("package users.v1;"), "Should have package");

    // Check service
    assert!(
        schema.contains("service UserService"),
        "Should have service"
    );
}

#[test]
fn test_connect_rpc_methods() {
    let schema = UserService::connect_schema();

    // Check RPC methods (CamelCase)
    assert!(schema.contains("rpc GetUser"), "Should have GetUser rpc");
    assert!(
        schema.contains("rpc ListUsers"),
        "Should have ListUsers rpc"
    );
    assert!(
        schema.contains("rpc CreateUser"),
        "Should have CreateUser rpc"
    );
    assert!(
        schema.contains("rpc DeleteUser"),
        "Should have DeleteUser rpc"
    );
    assert!(
        schema.contains("rpc UpdateUser"),
        "Should have UpdateUser rpc"
    );
}

#[test]
fn test_connect_messages() {
    let schema = UserService::connect_schema();

    // Check request messages
    assert!(
        schema.contains("message GetUserRequest"),
        "Should have GetUserRequest"
    );
    assert!(
        schema.contains("message CreateUserRequest"),
        "Should have CreateUserRequest"
    );

    // Check response messages
    assert!(
        schema.contains("message GetUserResponse"),
        "Should have GetUserResponse"
    );
    assert!(
        schema.contains("message CreateUserResponse"),
        "Should have CreateUserResponse"
    );
}

#[test]
fn test_connect_paths() {
    let paths = UserService::connect_paths();

    // Should have Connect-style paths
    assert!(paths.len() == 5, "Should have 5 paths");
    assert!(
        paths.contains(&"/users.v1.UserService/GetUser"),
        "Should have GetUser path"
    );
    assert!(
        paths.contains(&"/users.v1.UserService/CreateUser"),
        "Should have CreateUser path"
    );
    assert!(
        paths.contains(&"/users.v1.UserService/ListUsers"),
        "Should have ListUsers path"
    );
    assert!(
        paths.contains(&"/users.v1.UserService/DeleteUser"),
        "Should have DeleteUser path"
    );
    assert!(
        paths.contains(&"/users.v1.UserService/UpdateUser"),
        "Should have UpdateUser path"
    );
}

#[test]
fn test_connect_fields() {
    let schema = UserService::connect_schema();

    // CreateUser should have name and email fields
    assert!(
        schema.contains("string name = 1"),
        "CreateUser should have name field"
    );
    assert!(
        schema.contains("string email = 2"),
        "CreateUser should have email field"
    );
}

#[test]
fn test_connect_optional_fields() {
    let schema = UserService::connect_schema();

    // UpdateUser should have optional email
    assert!(
        schema.contains("optional"),
        "Should have optional field for Option<T>"
    );
}

#[test]
fn test_connect_doc_comments() {
    let schema = UserService::connect_schema();

    // Doc comments should be preserved
    assert!(
        schema.contains("// Get user by ID"),
        "Should preserve doc comments"
    );
}

// Test default package name
#[derive(Clone)]
struct SimpleService;

#[connect]
impl SimpleService {
    pub fn do_thing(&self) -> String {
        "done".to_string()
    }
}

#[test]
fn test_connect_default_package() {
    let schema = SimpleService::connect_schema();

    // Default package should be snake_case struct name
    assert!(
        schema.contains("package simple_service;"),
        "Should have default package, got:\n{}",
        schema
    );
}

#[test]
fn test_connect_default_paths() {
    let paths = SimpleService::connect_paths();

    // Path should use default package
    assert!(
        paths.contains(&"/simple_service.SimpleService/DoThing"),
        "Should have path with default package"
    );
}

// Test various return types
#[derive(Clone)]
struct TypeService;

#[connect]
impl TypeService {
    pub fn get_int(&self) -> i32 {
        42
    }

    pub fn get_float(&self) -> f64 {
        3.5
    }

    pub fn get_bool(&self) -> bool {
        true
    }

    pub fn do_nothing(&self) {}
}

#[test]
fn test_connect_return_types() {
    let schema = TypeService::connect_schema();

    // Check various proto types
    assert!(schema.contains("int32 result"), "Should map i32 to int32");
    assert!(schema.contains("double result"), "Should map f64 to double");
    assert!(schema.contains("bool result"), "Should map bool to bool");
}
