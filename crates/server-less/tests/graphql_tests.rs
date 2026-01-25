//! Integration tests for the GraphQL macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use server_less::{graphql, graphql_enum};

#[derive(Clone)]
struct SimpleService {
    prefix: String,
}

impl SimpleService {
    fn new() -> Self {
        Self {
            prefix: "Hello".to_string(),
        }
    }
}

#[graphql]
impl SimpleService {
    /// Get greeting
    pub fn get_greeting(&self) -> String {
        format!("{}, World!", self.prefix)
    }

    /// List items
    pub fn list_items(&self) -> Vec<String> {
        vec!["a".to_string(), "b".to_string()]
    }

    /// Create item
    pub fn create_item(&self, name: String) -> String {
        format!("Created: {}", name)
    }

    /// Get count
    pub fn get_count(&self) -> i32 {
        42
    }

    /// Check active
    pub fn is_active(&self) -> bool {
        true
    }
}

#[test]
fn test_graphql_schema_created() {
    let service = SimpleService::new();
    let schema = service.graphql_schema();
    // Schema is created successfully
    let _ = schema;
}

#[test]
fn test_graphql_sdl_generated() {
    let service = SimpleService::new();
    let sdl = service.graphql_sdl();

    // Check SDL contains expected types
    assert!(
        sdl.contains("SimpleServiceQuery"),
        "SDL should have Query type, got:\n{}",
        sdl
    );
    assert!(
        sdl.contains("SimpleServiceMutation"),
        "SDL should have Mutation type"
    );

    // Check query methods (camelCase)
    assert!(
        sdl.contains("getGreeting"),
        "SDL should have getGreeting query"
    );
    assert!(sdl.contains("listItems"), "SDL should have listItems query");

    // Check mutation methods
    assert!(
        sdl.contains("createItem"),
        "SDL should have createItem mutation"
    );
}

#[test]
fn test_graphql_router_created() {
    let service = SimpleService::new();
    let router = service.graphql_router();
    // Router is created successfully
    let _ = router;
}

// Test query-only service (no mutations)
#[derive(Clone)]
struct ReadOnlyService;

#[graphql]
impl ReadOnlyService {
    /// Get info
    pub fn get_info(&self) -> String {
        "read only".to_string()
    }

    /// List things
    pub fn list_things(&self) -> Vec<String> {
        vec![]
    }
}

#[test]
fn test_graphql_query_only_service() {
    let service = ReadOnlyService;
    let sdl = service.graphql_sdl();

    // Should have query type
    assert!(sdl.contains("ReadOnlyServiceQuery"));
}

// Test actual query execution
#[tokio::test]
async fn test_graphql_execute_query() {
    let service = SimpleService::new();
    let schema = service.graphql_schema();

    let result = schema.execute("{ getGreeting }").await;
    assert!(
        result.errors.is_empty(),
        "Query should succeed: {:?}",
        result.errors
    );

    // The result should contain our greeting
    let data = result.data.into_json().unwrap();
    assert!(data["getGreeting"].as_str().is_some());
}

#[tokio::test]
async fn test_graphql_execute_query_with_int() {
    let service = SimpleService::new();
    let schema = service.graphql_schema();

    let result = schema.execute("{ getCount }").await;
    assert!(
        result.errors.is_empty(),
        "Query should succeed: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn test_graphql_execute_query_with_bool() {
    let service = SimpleService::new();
    let schema = service.graphql_schema();

    let result = schema.execute("{ isActive }").await;
    assert!(
        result.errors.is_empty(),
        "Query should succeed: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn test_graphql_execute_mutation() {
    let service = SimpleService::new();
    let schema = service.graphql_schema();

    let result = schema
        .execute(r#"mutation { createItem(name: "test") }"#)
        .await;
    assert!(
        result.errors.is_empty(),
        "Mutation should succeed: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn test_graphql_execute_list_query() {
    let service = SimpleService::new();
    let schema = service.graphql_schema();

    let result = schema.execute("{ listItems }").await;
    assert!(
        result.errors.is_empty(),
        "List query should succeed: {:?}",
        result.errors
    );
}

// Test custom struct objects
#[derive(Clone, Debug, Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
    email: String,
    active: bool,
}

#[derive(Clone)]
struct UserService;

#[graphql]
impl UserService {
    /// Get user by ID
    pub fn get_user(&self, id: i32) -> User {
        User {
            id,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            active: true,
        }
    }

    /// List all users
    pub fn list_users(&self) -> Vec<User> {
        vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                active: true,
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
                active: false,
            },
        ]
    }

    /// Create user
    pub fn create_user(&self, name: String, email: String) -> User {
        User {
            id: 99,
            name,
            email,
            active: true,
        }
    }
}

#[tokio::test]
async fn test_graphql_custom_struct_query() {
    let service = UserService;
    let schema = service.graphql_schema();

    let result = schema.execute("{ getUser(id: 1) }").await;
    assert!(
        result.errors.is_empty(),
        "Custom struct query should succeed: {:?}",
        result.errors
    );

    // The result should be a proper object, not a string
    let data = result.data.into_json().unwrap();
    let user = &data["getUser"];

    // Verify we get an object with fields
    assert!(user.is_object(), "Should return object, got: {:?}", user);
    assert_eq!(user["id"], 1, "Should have correct id field");
    assert_eq!(user["name"], "Alice", "Should have correct name field");
    assert_eq!(
        user["email"], "alice@example.com",
        "Should have correct email field"
    );
    assert_eq!(user["active"], true, "Should have correct active field");
}

#[tokio::test]
async fn test_graphql_custom_struct_list_query() {
    let service = UserService;
    let schema = service.graphql_schema();

    let result = schema.execute("{ listUsers }").await;
    assert!(
        result.errors.is_empty(),
        "Custom struct list query should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    let users = &data["listUsers"];

    assert!(users.is_array(), "Should return array");
    let users_array = users.as_array().unwrap();
    assert_eq!(users_array.len(), 2, "Should have 2 users");

    // Check first user
    assert!(users_array[0].is_object(), "User should be object");
    assert_eq!(users_array[0]["id"], 1);
    assert_eq!(users_array[0]["name"], "Alice");
}

#[tokio::test]
async fn test_graphql_custom_struct_mutation() {
    let service = UserService;
    let schema = service.graphql_schema();

    let result = schema
        .execute(r#"mutation { createUser(name: "Charlie", email: "charlie@example.com") }"#)
        .await;
    assert!(
        result.errors.is_empty(),
        "Custom struct mutation should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    let user = &data["createUser"];

    assert!(user.is_object(), "Should return object");
    assert_eq!(user["id"], 99);
    assert_eq!(user["name"], "Charlie");
    assert_eq!(user["email"], "charlie@example.com");
    assert_eq!(user["active"], true);
}

#[test]
fn test_graphql_openapi_paths_generated() {
    let paths = SimpleService::graphql_openapi_paths();

    // Should have 2 paths: POST /graphql (query) and GET /graphql (playground)
    assert_eq!(paths.len(), 2);

    // Find the POST endpoint
    let post_path = paths.iter().find(|p| p.method == "post").unwrap();
    assert_eq!(post_path.path, "/graphql");
    assert!(
        post_path
            .operation
            .summary
            .as_ref()
            .unwrap()
            .contains("query")
    );
    assert!(post_path.operation.request_body.is_some());
    assert!(post_path.operation.responses.contains_key("200"));

    // Find the GET endpoint (playground)
    let get_path = paths.iter().find(|p| p.method == "get").unwrap();
    assert_eq!(get_path.path, "/graphql");
    assert!(
        get_path
            .operation
            .summary
            .as_ref()
            .unwrap()
            .contains("Playground")
    );
}

// ============================================================================
// Custom Scalar Tests
// ============================================================================

#[derive(Clone)]
struct JsonService;

#[graphql]
impl JsonService {
    /// Get raw JSON data
    pub fn get_data(&self) -> serde_json::Value {
        serde_json::json!({"key": "value"})
    }

    /// Echo JSON data
    pub fn create_entry(&self, data: serde_json::Value) -> serde_json::Value {
        data
    }
}

#[test]
fn test_graphql_json_scalar_schema() {
    let service = JsonService;
    let sdl = service.graphql_sdl();

    // The JSON scalar should be registered in the schema
    assert!(
        sdl.contains("scalar JSON"),
        "Should register JSON scalar type. SDL:\n{}",
        sdl
    );
}

#[tokio::test]
async fn test_graphql_json_scalar_query() {
    let service = JsonService;
    let schema = service.graphql_schema();

    let result = schema.execute("{ getData }").await;
    assert!(
        result.errors.is_empty(),
        "JSON scalar query should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    assert!(data["getData"].is_object(), "Should return JSON object");
    assert_eq!(data["getData"]["key"], "value");
}

// ============================================================================
// Enum Type Tests
// ============================================================================

#[graphql_enum]
#[derive(Clone, Debug)]
enum Priority {
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

#[test]
fn test_graphql_enum_type_definition() {
    let enum_type = Priority::__graphql_enum_type();
    // The enum type should have been created (we can't easily inspect it,
    // but at least it compiles and returns the right type)
    let _ = enum_type;
}

#[test]
fn test_graphql_enum_to_value() {
    let value = Priority::High.__to_graphql_value();
    // The value should be an Enum variant in SCREAMING_SNAKE_CASE
    assert_eq!(
        value,
        async_graphql::Value::Enum(async_graphql::Name::new("HIGH"))
    );
}

#[test]
fn test_graphql_enum_all_variants() {
    assert_eq!(
        Priority::Low.__to_graphql_value(),
        async_graphql::Value::Enum(async_graphql::Name::new("LOW"))
    );
    assert_eq!(
        Priority::Medium.__to_graphql_value(),
        async_graphql::Value::Enum(async_graphql::Name::new("MEDIUM"))
    );
    assert_eq!(
        Priority::High.__to_graphql_value(),
        async_graphql::Value::Enum(async_graphql::Name::new("HIGH"))
    );
    assert_eq!(
        Priority::Critical.__to_graphql_value(),
        async_graphql::Value::Enum(async_graphql::Name::new("CRITICAL"))
    );
}

#[derive(Clone)]
struct PriorityService;

#[graphql(enums(Priority))]
impl PriorityService {
    /// Get default priority
    pub fn get_default_priority(&self) -> String {
        // For now returns as String until full enum return type support
        "HIGH".to_string()
    }
}

#[test]
fn test_graphql_enum_registered_in_schema() {
    let service = PriorityService;
    let sdl = service.graphql_sdl();

    // The Priority enum should be registered in the SDL
    assert!(
        sdl.contains("enum Priority"),
        "Should register Priority enum type. SDL:\n{}",
        sdl
    );

    // Should have SCREAMING_SNAKE_CASE variants
    assert!(
        sdl.contains("LOW"),
        "Should have LOW variant. SDL:\n{}",
        sdl
    );
    assert!(
        sdl.contains("MEDIUM"),
        "Should have MEDIUM variant. SDL:\n{}",
        sdl
    );
    assert!(
        sdl.contains("HIGH"),
        "Should have HIGH variant. SDL:\n{}",
        sdl
    );
    assert!(
        sdl.contains("CRITICAL"),
        "Should have CRITICAL variant. SDL:\n{}",
        sdl
    );
}

// ============================================================================
// Nested Object Tests
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NestedProfile {
    bio: String,
    avatar_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserWithProfile {
    id: i32,
    name: String,
    profile: NestedProfile,
}

#[derive(Clone)]
struct NestedService;

#[graphql]
impl NestedService {
    /// Get user with nested profile
    pub fn get_user_with_profile(&self, id: i32) -> UserWithProfile {
        UserWithProfile {
            id,
            name: "Alice".to_string(),
            profile: NestedProfile {
                bio: "Software engineer".to_string(),
                avatar_url: "https://example.com/avatar.jpg".to_string(),
            },
        }
    }

    /// Get list of users with profiles
    pub fn list_users_with_profiles(&self) -> Vec<UserWithProfile> {
        vec![
            UserWithProfile {
                id: 1,
                name: "Alice".to_string(),
                profile: NestedProfile {
                    bio: "Engineer".to_string(),
                    avatar_url: "https://example.com/alice.jpg".to_string(),
                },
            },
            UserWithProfile {
                id: 2,
                name: "Bob".to_string(),
                profile: NestedProfile {
                    bio: "Designer".to_string(),
                    avatar_url: "https://example.com/bob.jpg".to_string(),
                },
            },
        ]
    }
}

#[tokio::test]
async fn test_graphql_nested_object_query() {
    let service = NestedService;
    let schema = service.graphql_schema();

    // Query nested object
    let result = schema.execute("{ getUserWithProfile(id: 42) }").await;

    assert!(
        result.errors.is_empty(),
        "Query should succeed: {:?}",
        result.errors
    );

    // Convert to JSON for easier inspection
    let json: serde_json::Value = serde_json::to_value(&result.data).unwrap();
    let user = &json["getUserWithProfile"];

    // Check top-level fields
    assert_eq!(user["id"], 42);
    assert_eq!(user["name"], "Alice");

    // Check nested profile object (should NOT be a string)
    let profile = &user["profile"];
    assert!(
        profile.is_object(),
        "Profile should be an object, not a string. Got: {:?}",
        profile
    );
    assert_eq!(profile["bio"], "Software engineer");
    assert_eq!(profile["avatar_url"], "https://example.com/avatar.jpg");
}

#[tokio::test]
async fn test_graphql_nested_object_in_list() {
    let service = NestedService;
    let schema = service.graphql_schema();

    // Query list of nested objects
    let result = schema.execute("{ listUsersWithProfiles }").await;

    assert!(
        result.errors.is_empty(),
        "Query should succeed: {:?}",
        result.errors
    );

    // Convert to JSON for easier inspection
    let json: serde_json::Value = serde_json::to_value(&result.data).unwrap();
    let users = json["listUsersWithProfiles"]
        .as_array()
        .expect("Should be an array");

    assert_eq!(users.len(), 2);

    // Check first user's nested profile
    let alice = &users[0];
    assert_eq!(alice["name"], "Alice");
    let alice_profile = &alice["profile"];
    assert!(
        alice_profile.is_object(),
        "Profile should be an object. Got: {:?}",
        alice_profile
    );
    assert_eq!(alice_profile["bio"], "Engineer");

    // Check second user's nested profile
    let bob = &users[1];
    assert_eq!(bob["name"], "Bob");
    let bob_profile = &bob["profile"];
    assert!(
        bob_profile.is_object(),
        "Profile should be an object. Got: {:?}",
        bob_profile
    );
    assert_eq!(bob_profile["bio"], "Designer");
}

// ============================================================================
// Input Type Tests
// ============================================================================

use server_less::graphql_input;

#[graphql_input]
#[derive(Clone, Debug, Deserialize)]
struct CreateUserInput {
    /// User's name
    name: String,
    /// User's email address
    email: String,
    /// Optional age
    age: Option<i32>,
}

#[derive(Clone)]
struct InputService;

#[graphql(inputs(CreateUserInput))]
impl InputService {
    /// Get service status
    pub fn get_status(&self) -> String {
        "running".to_string()
    }

    /// Create a user
    pub fn create_user(&self, input: CreateUserInput) -> String {
        format!("Created: {} <{}>", input.name, input.email)
    }
}

#[test]
fn test_graphql_input_type_generated() {
    // Verify the input type helper exists
    let input_type = CreateUserInput::__graphql_input_type();
    assert_eq!(input_type.type_name(), "CreateUserInput");
}

#[test]
fn test_graphql_input_schema_registration() {
    let service = InputService;
    let sdl = service.graphql_sdl();

    // Should have input type in schema
    assert!(
        sdl.contains("input CreateUserInput"),
        "Should register CreateUserInput input type. SDL:\n{}",
        sdl
    );

    // Check fields
    assert!(
        sdl.contains("name: String!"),
        "Should have name field. SDL:\n{}",
        sdl
    );
    assert!(
        sdl.contains("email: String!"),
        "Should have email field. SDL:\n{}",
        sdl
    );
    // Optional field should not have !
    assert!(
        sdl.contains("age: Int"),
        "Should have age field. SDL:\n{}",
        sdl
    );
}
