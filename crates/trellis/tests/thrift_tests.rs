//! Integration tests for the Thrift schema generation macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use rhizome_trellis::thrift;

#[derive(Clone)]
struct UserService;

#[thrift(namespace = "users")]
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
}

#[test]
fn test_thrift_schema_generated() {
    let schema = UserService::thrift_schema();

    // Check namespace
    assert!(
        schema.contains("namespace rs users"),
        "Should have namespace"
    );

    // Check service
    assert!(
        schema.contains("service UserService"),
        "Should have service"
    );
}

#[test]
fn test_thrift_methods() {
    let schema = UserService::thrift_schema();

    // Check methods (snake_case)
    assert!(schema.contains("get_user"), "Should have get_user method");
    assert!(
        schema.contains("list_users"),
        "Should have list_users method"
    );
    assert!(
        schema.contains("create_user"),
        "Should have create_user method"
    );
    assert!(
        schema.contains("delete_user"),
        "Should have delete_user method"
    );
}

#[test]
fn test_thrift_structs() {
    let schema = UserService::thrift_schema();

    // Check Args structs
    assert!(
        schema.contains("struct GetUserArgs"),
        "Should have GetUserArgs"
    );
    assert!(
        schema.contains("struct CreateUserArgs"),
        "Should have CreateUserArgs"
    );
}

#[test]
fn test_thrift_fields() {
    let schema = UserService::thrift_schema();

    // CreateUser should have name and email fields
    assert!(
        schema.contains("string name"),
        "CreateUser should have name field"
    );
    assert!(
        schema.contains("string email"),
        "CreateUser should have email field"
    );
}

#[test]
fn test_thrift_doc_comments() {
    let schema = UserService::thrift_schema();

    // Doc comments should be preserved
    assert!(
        schema.contains("// Get user by ID"),
        "Should preserve doc comments"
    );
}

// Test default namespace
#[derive(Clone)]
struct SimpleService;

#[thrift]
impl SimpleService {
    pub fn do_thing(&self) -> String {
        "done".to_string()
    }
}

#[test]
fn test_thrift_default_namespace() {
    let schema = SimpleService::thrift_schema();

    // Default namespace should be snake_case struct name
    assert!(
        schema.contains("namespace rs simple_service"),
        "Should have default namespace"
    );
}

// Test various types
#[derive(Clone)]
struct TypeService;

#[thrift]
impl TypeService {
    pub fn get_int(&self) -> i32 {
        42
    }

    pub fn get_long(&self) -> i64 {
        123456789
    }

    pub fn get_double(&self) -> f64 {
        3.5
    }

    pub fn get_bool(&self) -> bool {
        true
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        vec![1, 2, 3]
    }

    pub fn do_nothing(&self) {}
}

#[test]
fn test_thrift_return_types() {
    let schema = TypeService::thrift_schema();

    // Check various Thrift types
    assert!(schema.contains("i32 get_int"), "Should map i32 to i32");
    assert!(schema.contains("i64 get_long"), "Should map i64 to i64");
    assert!(
        schema.contains("double get_double"),
        "Should map f64 to double"
    );
    assert!(schema.contains("bool get_bool"), "Should map bool to bool");
    assert!(
        schema.contains("binary get_bytes"),
        "Should map Vec<u8> to binary"
    );
    assert!(schema.contains("void do_nothing"), "Should map () to void");
}

// Test with schema validation
#[derive(Clone)]
struct ValidatedThriftService;

#[thrift(
    namespace = "validated",
    schema = "../fixtures/validated_service.thrift"
)]
impl ValidatedThriftService {
    /// Get greeting
    pub fn get_greeting(&self) -> String {
        "hello".to_string()
    }

    /// Create item
    pub fn create_item(&self, name: String) -> String {
        name
    }
}

#[test]
fn test_thrift_schema_validation_passes() {
    ValidatedThriftService::assert_schema_matches();
}

#[test]
fn test_thrift_schema_validation_result() {
    let result = ValidatedThriftService::validate_schema();
    assert!(result.is_ok(), "Validation should pass: {:?}", result);
}
