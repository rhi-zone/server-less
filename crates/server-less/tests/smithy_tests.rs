//! Integration tests for the Smithy IDL schema generation macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::smithy;

#[derive(Clone)]
struct UserService;

#[smithy(namespace = "com.example.users", version = "2024-01-15")]
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
fn test_smithy_schema_generated() {
    let schema = UserService::smithy_schema();

    // Check Smithy version
    assert!(
        schema.contains("$version: \"2\""),
        "Should have Smithy 2 version"
    );

    // Check namespace
    assert!(
        schema.contains("namespace com.example.users"),
        "Should have namespace"
    );
}

#[test]
fn test_smithy_service_definition() {
    let schema = UserService::smithy_schema();

    // Check service definition
    assert!(
        schema.contains("service UserService"),
        "Should have service definition"
    );

    // Check version
    assert!(
        schema.contains("version: \"2024-01-15\""),
        "Should have version"
    );
}

#[test]
fn test_smithy_operations() {
    let schema = UserService::smithy_schema();

    // Check operations list in service
    assert!(
        schema.contains("operations:"),
        "Should have operations list"
    );

    // Check operation definitions (PascalCase)
    assert!(
        schema.contains("operation GetUser"),
        "Should have GetUser operation"
    );
    assert!(
        schema.contains("operation ListUsers"),
        "Should have ListUsers operation"
    );
    assert!(
        schema.contains("operation CreateUser"),
        "Should have CreateUser operation"
    );
    assert!(
        schema.contains("operation DeleteUser"),
        "Should have DeleteUser operation"
    );
    assert!(
        schema.contains("operation UpdateUser"),
        "Should have UpdateUser operation"
    );
}

#[test]
fn test_smithy_structures() {
    let schema = UserService::smithy_schema();

    // Check input structures
    assert!(
        schema.contains("structure GetUserInput"),
        "Should have GetUserInput"
    );
    assert!(
        schema.contains("structure CreateUserInput"),
        "Should have CreateUserInput"
    );

    // Check output structures
    assert!(
        schema.contains("structure GetUserOutput"),
        "Should have GetUserOutput"
    );
    assert!(
        schema.contains("structure CreateUserOutput"),
        "Should have CreateUserOutput"
    );
}

#[test]
fn test_smithy_required_fields() {
    let schema = UserService::smithy_schema();

    // Required fields should have @required
    assert!(
        schema.contains("@required"),
        "Should have @required for non-optional fields"
    );
}

#[test]
fn test_smithy_doc_comments() {
    let schema = UserService::smithy_schema();

    // Doc comments should be preserved
    assert!(
        schema.contains("/// Get user by ID"),
        "Should preserve doc comments"
    );
}

// Test default namespace
#[derive(Clone)]
struct SimpleService;

#[smithy]
impl SimpleService {
    pub fn do_thing(&self) -> String {
        "done".to_string()
    }
}

#[test]
fn test_smithy_default_namespace() {
    let schema = SimpleService::smithy_schema();

    // Default namespace should be based on struct name
    assert!(
        schema.contains("namespace com.example.simple_service"),
        "Should have default namespace, got:\n{}",
        schema
    );
}

// Test various return types
#[derive(Clone)]
struct TypeService;

#[smithy]
impl TypeService {
    pub fn get_int(&self) -> i32 {
        42
    }

    pub fn get_long(&self) -> i64 {
        42
    }

    pub fn get_float(&self) -> f32 {
        3.5
    }

    pub fn get_double(&self) -> f64 {
        3.5
    }

    pub fn get_bool(&self) -> bool {
        true
    }

    pub fn do_nothing(&self) {}
}

#[test]
fn test_smithy_return_types() {
    let schema = TypeService::smithy_schema();

    // Check various Smithy types
    assert!(
        schema.contains("result: Integer"),
        "Should map i32 to Integer"
    );
    assert!(schema.contains("result: Long"), "Should map i64 to Long");
    assert!(schema.contains("result: Float"), "Should map f32 to Float");
    assert!(
        schema.contains("result: Double"),
        "Should map f64 to Double"
    );
    assert!(
        schema.contains("result: Boolean"),
        "Should map bool to Boolean"
    );
}

#[test]
fn test_smithy_empty_output() {
    let schema = TypeService::smithy_schema();

    // Unit return type should produce empty structure
    assert!(
        schema.contains("structure DoNothingOutput {}"),
        "Should have empty output for unit return"
    );
}
