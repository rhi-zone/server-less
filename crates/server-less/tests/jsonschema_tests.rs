//! Integration tests for the JSON Schema generation macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use server_less::jsonschema;

#[derive(Clone)]
struct UserService;

#[jsonschema(title = "User API")]
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
fn test_jsonschema_generated() {
    let schema = UserService::json_schema();

    // Should have schema version
    assert!(
        schema["$schema"]
            .as_str()
            .unwrap()
            .contains("json-schema.org"),
        "Should have JSON Schema draft"
    );
}

#[test]
fn test_jsonschema_title() {
    let schema = UserService::json_schema();

    // Should have title
    assert_eq!(
        schema["title"].as_str().unwrap(),
        "User API",
        "Should have title"
    );
}

#[test]
fn test_jsonschema_definitions() {
    let schema = UserService::json_schema();
    let defs = &schema["definitions"];

    // Should have request definitions
    assert!(
        defs["Get_userRequest"].is_object(),
        "Should have GetUserRequest"
    );
    assert!(
        defs["Create_userRequest"].is_object(),
        "Should have CreateUserRequest"
    );

    // Should have response definitions
    assert!(
        defs["Get_userResponse"].is_object(),
        "Should have GetUserResponse"
    );
    assert!(
        defs["Create_userResponse"].is_object(),
        "Should have CreateUserResponse"
    );
}

#[test]
fn test_jsonschema_request_properties() {
    let schema = UserService::json_schema();
    let defs = &schema["definitions"];

    // CreateUser should have name and email properties
    let create_req = &defs["Create_userRequest"];
    let props = &create_req["properties"];
    assert!(props["name"].is_object(), "Should have name property");
    assert!(props["email"].is_object(), "Should have email property");
}

#[test]
fn test_jsonschema_required_fields() {
    let schema = UserService::json_schema();
    let defs = &schema["definitions"];

    // CreateUser should have required fields
    let create_req = &defs["Create_userRequest"];
    let required = create_req["required"].as_array().unwrap();
    assert!(
        required.iter().any(|v| v.as_str() == Some("name")),
        "name should be required"
    );
    assert!(
        required.iter().any(|v| v.as_str() == Some("email")),
        "email should be required"
    );
}

#[test]
fn test_jsonschema_string_type() {
    let schema = UserService::json_schema();
    let defs = &schema["definitions"];

    let get_req = &defs["Get_userRequest"];
    let id_type = &get_req["properties"]["id"]["type"];
    assert_eq!(id_type.as_str().unwrap(), "string", "id should be string");
}

#[test]
fn test_jsonschema_response_result() {
    let schema = UserService::json_schema();
    let defs = &schema["definitions"];

    // Response should have result property
    let get_resp = &defs["Get_userResponse"];
    assert!(
        get_resp["properties"]["result"].is_object(),
        "Should have result property"
    );
}

// Test default title
#[derive(Clone)]
struct SimpleService;

#[jsonschema]
impl SimpleService {
    pub fn do_thing(&self) -> String {
        "done".to_string()
    }
}

#[test]
fn test_jsonschema_default_title() {
    let schema = SimpleService::json_schema();

    // Default title should be struct name
    assert_eq!(
        schema["title"].as_str().unwrap(),
        "SimpleService",
        "Should have default title"
    );
}

// Test various return types
#[derive(Clone)]
struct TypeService;

#[jsonschema]
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

    pub fn get_array(&self) -> Vec<String> {
        vec![]
    }

    pub fn do_nothing(&self) {}
}

#[test]
fn test_jsonschema_integer_type() {
    let schema = TypeService::json_schema();
    let defs = &schema["definitions"];

    let resp = &defs["Get_intResponse"];
    let result_type = &resp["properties"]["result"]["type"];
    assert_eq!(
        result_type.as_str().unwrap(),
        "integer",
        "i32 should be integer"
    );
}

#[test]
fn test_jsonschema_number_type() {
    let schema = TypeService::json_schema();
    let defs = &schema["definitions"];

    let resp = &defs["Get_floatResponse"];
    let result_type = &resp["properties"]["result"]["type"];
    assert_eq!(
        result_type.as_str().unwrap(),
        "number",
        "f64 should be number"
    );
}

#[test]
fn test_jsonschema_boolean_type() {
    let schema = TypeService::json_schema();
    let defs = &schema["definitions"];

    let resp = &defs["Get_boolResponse"];
    let result_type = &resp["properties"]["result"]["type"];
    assert_eq!(
        result_type.as_str().unwrap(),
        "boolean",
        "bool should be boolean"
    );
}

#[test]
fn test_jsonschema_array_type() {
    let schema = TypeService::json_schema();
    let defs = &schema["definitions"];

    let resp = &defs["Get_arrayResponse"];
    let result_type = &resp["properties"]["result"]["type"];
    assert_eq!(
        result_type.as_str().unwrap(),
        "array",
        "Vec should be array"
    );
}

#[test]
fn test_jsonschema_empty_response() {
    let schema = TypeService::json_schema();
    let defs = &schema["definitions"];

    // Unit return should have empty properties
    let resp = &defs["Do_nothingResponse"];
    let props = resp["properties"].as_object().unwrap();
    assert!(props.is_empty(), "Unit return should have empty properties");
}
