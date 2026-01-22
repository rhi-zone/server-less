//! Integration tests for the Cap'n Proto schema generation macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use trellis::capnp;

#[derive(Clone)]
struct UserService;

#[capnp(id = "0x85150b117366d14b")]
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
fn test_capnp_schema_generated() {
    let schema = UserService::capnp_schema();

    // Check schema ID
    assert!(
        schema.contains("@0x85150b117366d14b"),
        "Should have schema ID"
    );

    // Check interface
    assert!(
        schema.contains("interface UserService"),
        "Should have interface"
    );
}

#[test]
fn test_capnp_methods() {
    let schema = UserService::capnp_schema();

    // Check methods (lowerCamelCase)
    assert!(schema.contains("getUser @0"), "Should have getUser method");
    assert!(
        schema.contains("listUsers @1"),
        "Should have listUsers method"
    );
    assert!(
        schema.contains("createUser @2"),
        "Should have createUser method"
    );
    assert!(
        schema.contains("deleteUser @3"),
        "Should have deleteUser method"
    );
}

#[test]
fn test_capnp_structs() {
    let schema = UserService::capnp_schema();

    // Check Params structs
    assert!(
        schema.contains("struct GetUserParams"),
        "Should have GetUserParams"
    );
    assert!(
        schema.contains("struct CreateUserParams"),
        "Should have CreateUserParams"
    );

    // Check Result structs
    assert!(
        schema.contains("struct GetUserResult"),
        "Should have GetUserResult"
    );
    assert!(
        schema.contains("struct CreateUserResult"),
        "Should have CreateUserResult"
    );
}

#[test]
fn test_capnp_fields() {
    let schema = UserService::capnp_schema();

    // CreateUser should have name and email fields
    assert!(
        schema.contains("name @0 :Text"),
        "CreateUser should have name field: {}",
        schema
    );
    assert!(
        schema.contains("email @1 :Text"),
        "CreateUser should have email field"
    );
}

#[test]
fn test_capnp_doc_comments() {
    let schema = UserService::capnp_schema();

    // Doc comments should be preserved
    assert!(
        schema.contains("# Get user by ID"),
        "Should preserve doc comments"
    );
}

// Test default schema ID
#[derive(Clone)]
struct SimpleService;

#[capnp]
impl SimpleService {
    pub fn do_thing(&self) -> String {
        "done".to_string()
    }
}

#[test]
fn test_capnp_default_id() {
    let schema = SimpleService::capnp_schema();

    // Default ID should be placeholder
    assert!(
        schema.contains("@0x0000000000000000"),
        "Should have default placeholder ID"
    );
}

// Test various types
#[derive(Clone)]
struct TypeService;

#[capnp]
impl TypeService {
    pub fn get_int(&self) -> i32 {
        42
    }

    pub fn get_long(&self) -> i64 {
        123456789
    }

    pub fn get_float(&self) -> f64 {
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
fn test_capnp_return_types() {
    let schema = TypeService::capnp_schema();

    // Check various Cap'n Proto types
    assert!(schema.contains(":Int32"), "Should map i32 to Int32");
    assert!(schema.contains(":Int64"), "Should map i64 to Int64");
    assert!(schema.contains(":Float64"), "Should map f64 to Float64");
    assert!(schema.contains(":Bool"), "Should map bool to Bool");
    assert!(
        schema.contains(":Data"),
        "Should map Vec<u8> to Data, got: {}",
        schema
    );
}

// ============================================================================
// Schema Validation Tests (schema-first mode)
// ============================================================================

#[derive(Clone)]
struct ValidatedCapnpService;

#[capnp(
    id = "0xabcd1234abcd1234",
    schema = "../fixtures/validated_service.capnp"
)]
impl ValidatedCapnpService {
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
fn test_capnp_schema_validation_passes() {
    // Should not panic - schema matches
    ValidatedCapnpService::assert_schema_matches();
}

#[test]
fn test_capnp_schema_validation_result() {
    // Should return Ok - schema matches
    let result = ValidatedCapnpService::validate_schema();
    assert!(result.is_ok(), "Validation should pass: {:?}", result);
}

// Test that validation detects mismatches
#[derive(Clone)]
struct MismatchedCapnpService;

// This service doesn't match the validated_service.capnp
#[capnp(
    id = "0xabcd1234abcd1234",
    schema = "../fixtures/validated_service.capnp"
)]
impl MismatchedCapnpService {
    /// Different method
    pub fn different_method(&self) -> String {
        "different".to_string()
    }
}

#[test]
fn test_capnp_schema_validation_fails_on_mismatch() {
    // Should return Err - schema doesn't match
    let result = MismatchedCapnpService::validate_schema();
    assert!(
        result.is_err(),
        "Validation should fail for mismatched service"
    );

    let err = result.unwrap_err();
    assert!(err.contains("mismatch"), "Error should mention mismatch");
}
