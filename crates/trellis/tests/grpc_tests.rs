//! Integration tests for the gRPC proto generation macro.

use trellis::grpc;

#[derive(Clone)]
struct UserService;

#[grpc(package = "users.v1")]
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
fn test_proto_schema_generated() {
    let proto = UserService::proto_schema();

    // Check package
    assert!(proto.contains("package users.v1;"), "Should have package");

    // Check service
    assert!(proto.contains("service UserService"), "Should have service");
}

#[test]
fn test_proto_rpc_methods() {
    let proto = UserService::proto_schema();

    // Check RPC methods (CamelCase)
    assert!(proto.contains("rpc GetUser"), "Should have GetUser rpc");
    assert!(proto.contains("rpc ListUsers"), "Should have ListUsers rpc");
    assert!(proto.contains("rpc CreateUser"), "Should have CreateUser rpc");
    assert!(proto.contains("rpc DeleteUser"), "Should have DeleteUser rpc");
    assert!(proto.contains("rpc UpdateUser"), "Should have UpdateUser rpc");
}

#[test]
fn test_proto_messages() {
    let proto = UserService::proto_schema();

    // Check request messages
    assert!(
        proto.contains("message GetUserRequest"),
        "Should have GetUserRequest"
    );
    assert!(
        proto.contains("message CreateUserRequest"),
        "Should have CreateUserRequest"
    );

    // Check response messages
    assert!(
        proto.contains("message GetUserResponse"),
        "Should have GetUserResponse"
    );
    assert!(
        proto.contains("message CreateUserResponse"),
        "Should have CreateUserResponse"
    );
}

#[test]
fn test_proto_fields() {
    let proto = UserService::proto_schema();

    // CreateUser should have name and email fields
    assert!(
        proto.contains("string name = 1"),
        "CreateUser should have name field"
    );
    assert!(
        proto.contains("string email = 2"),
        "CreateUser should have email field"
    );
}

#[test]
fn test_proto_optional_fields() {
    let proto = UserService::proto_schema();

    // UpdateUser should have optional email
    assert!(
        proto.contains("optional"),
        "Should have optional field for Option<T>"
    );
}

#[test]
fn test_proto_doc_comments() {
    let proto = UserService::proto_schema();

    // Doc comments should be preserved
    assert!(
        proto.contains("// Get user by ID"),
        "Should preserve doc comments"
    );
}

// Test default package name
#[derive(Clone)]
struct SimpleService;

#[grpc]
impl SimpleService {
    pub fn do_thing(&self) -> String {
        "done".to_string()
    }
}

#[test]
fn test_proto_default_package() {
    let proto = SimpleService::proto_schema();

    // Default package should be snake_case struct name
    assert!(
        proto.contains("package simple_service;"),
        "Should have default package, got:\n{}",
        proto
    );
}

// Test various return types
#[derive(Clone)]
struct TypeService;

#[grpc]
impl TypeService {
    pub fn get_int(&self) -> i32 {
        42
    }

    pub fn get_float(&self) -> f64 {
        3.14
    }

    pub fn get_bool(&self) -> bool {
        true
    }

    pub fn do_nothing(&self) {}
}

#[test]
fn test_proto_return_types() {
    let proto = TypeService::proto_schema();

    // Check various proto types
    assert!(proto.contains("int32 result"), "Should map i32 to int32");
    assert!(proto.contains("double result"), "Should map f64 to double");
    assert!(proto.contains("bool result"), "Should map bool to bool");
}

// ============================================================================
// Schema Validation Tests (schema-first mode)
// ============================================================================

#[derive(Clone)]
struct ValidatedService;

#[grpc(package = "validated.v1", schema = "../fixtures/validated_service.proto")]
impl ValidatedService {
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
fn test_schema_validation_passes() {
    // Should not panic - schema matches
    ValidatedService::assert_schema_matches();
}

#[test]
fn test_schema_validation_result() {
    // Should return Ok - schema matches
    let result = ValidatedService::validate_schema();
    assert!(result.is_ok(), "Validation should pass: {:?}", result);
}

// Test that validation detects mismatches
#[derive(Clone)]
struct MismatchedService;

// This service doesn't match the validated_service.proto
#[grpc(package = "validated.v1", schema = "../fixtures/validated_service.proto")]
impl MismatchedService {
    /// Different method
    pub fn different_method(&self) -> String {
        "different".to_string()
    }
}

#[test]
fn test_schema_validation_fails_on_mismatch() {
    // Should return Err - schema doesn't match
    let result = MismatchedService::validate_schema();
    assert!(result.is_err(), "Validation should fail for mismatched service");

    let err = result.unwrap_err();
    assert!(err.contains("mismatch"), "Error should mention mismatch");
}
