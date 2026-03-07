//! Integration tests for the GraphQL macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use server_less::{graphql, graphql_enum, serve};

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

// ============================================================================
// Mount / Composition Tests
//
// A parent service can expose a child service's queries/mutations by returning
// `&ChildService` from a method. The macro inlines all child fields into the
// parent's query/mutation Objects so a single schema contains everything.
// ============================================================================

/// Child service with its own queries and mutations.
#[derive(Clone)]
struct ProductService {
    tax_rate: f64,
}

impl ProductService {
    fn new() -> Self {
        Self { tax_rate: 0.1 }
    }
}

#[graphql]
impl ProductService {
    /// Get product name
    pub fn get_product_name(&self) -> String {
        "Widget".to_string()
    }

    /// Get product price
    pub fn get_product_price(&self) -> i32 {
        100
    }

    /// Create product
    pub fn create_product(&self, name: String) -> String {
        format!("Created: {}", name)
    }
}

/// Parent service that mounts `ProductService` as a child.
#[derive(Clone)]
struct CatalogService {
    product_service: ProductService,
}

impl CatalogService {
    fn new() -> Self {
        Self {
            product_service: ProductService::new(),
        }
    }
}

#[graphql]
impl CatalogService {
    /// Get catalog name
    pub fn get_catalog_name(&self) -> String {
        "Main Catalog".to_string()
    }

    /// Get catalog version
    pub fn get_catalog_version(&self) -> i32 {
        1
    }

    /// Update catalog description
    pub fn update_catalog_description(&self, description: String) -> String {
        format!("Updated: {}", description)
    }

    /// Mount: expose ProductService fields in this schema
    pub fn products(&self) -> &ProductService {
        &self.product_service
    }
}

#[test]
fn test_graphql_mount_schema_created() {
    let service = CatalogService::new();
    let schema = service.graphql_schema();
    let _ = schema;
}

#[test]
fn test_graphql_mount_sdl_contains_parent_fields() {
    let service = CatalogService::new();
    let sdl = service.graphql_sdl();

    // Parent query fields should be present
    assert!(
        sdl.contains("getCatalogName"),
        "SDL should have getCatalogName from parent. SDL:\n{}",
        sdl
    );
    assert!(
        sdl.contains("getCatalogVersion"),
        "SDL should have getCatalogVersion from parent. SDL:\n{}",
        sdl
    );
}

#[test]
fn test_graphql_mount_sdl_contains_child_fields() {
    let service = CatalogService::new();
    let sdl = service.graphql_sdl();

    // Child query fields should be inlined into parent's schema
    assert!(
        sdl.contains("getProductName"),
        "SDL should have getProductName from child ProductService. SDL:\n{}",
        sdl
    );
    assert!(
        sdl.contains("getProductPrice"),
        "SDL should have getProductPrice from child ProductService. SDL:\n{}",
        sdl
    );
}

#[test]
fn test_graphql_mount_sdl_contains_child_mutations() {
    let service = CatalogService::new();
    let sdl = service.graphql_sdl();

    // Child mutation fields should be inlined into parent's mutation type
    assert!(
        sdl.contains("createProduct"),
        "SDL should have createProduct mutation from child ProductService. SDL:\n{}",
        sdl
    );
    // Parent's own mutation should also be present
    assert!(
        sdl.contains("updateCatalogDescription"),
        "SDL should have updateCatalogDescription mutation from parent. SDL:\n{}",
        sdl
    );
}

#[tokio::test]
async fn test_graphql_mount_execute_parent_query() {
    let service = CatalogService::new();
    let schema = service.graphql_schema();

    let result = schema.execute("{ getCatalogName }").await;
    assert!(
        result.errors.is_empty(),
        "Parent query should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    assert_eq!(data["getCatalogName"], "Main Catalog");
}

#[tokio::test]
async fn test_graphql_mount_execute_child_query() {
    let service = CatalogService::new();
    let schema = service.graphql_schema();

    // Child's query field is now accessible directly through the parent schema
    let result = schema.execute("{ getProductName }").await;
    assert!(
        result.errors.is_empty(),
        "Child query through parent schema should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    assert_eq!(data["getProductName"], "Widget");
}

#[tokio::test]
async fn test_graphql_mount_execute_child_query_int() {
    let service = CatalogService::new();
    let schema = service.graphql_schema();

    let result = schema.execute("{ getProductPrice }").await;
    assert!(
        result.errors.is_empty(),
        "Child int query through parent schema should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    assert_eq!(data["getProductPrice"], 100);
}

#[tokio::test]
async fn test_graphql_mount_execute_child_mutation() {
    let service = CatalogService::new();
    let schema = service.graphql_schema();

    let result = schema
        .execute(r#"mutation { createProduct(name: "Gadget") }"#)
        .await;
    assert!(
        result.errors.is_empty(),
        "Child mutation through parent schema should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    assert_eq!(data["createProduct"], "Created: Gadget");
}

#[tokio::test]
async fn test_graphql_mount_execute_parent_mutation() {
    let service = CatalogService::new();
    let schema = service.graphql_schema();

    let result = schema
        .execute(r#"mutation { updateCatalogDescription(description: "New desc") }"#)
        .await;
    assert!(
        result.errors.is_empty(),
        "Parent mutation should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    assert_eq!(data["updateCatalogDescription"], "Updated: New desc");
}

/// Child-only service with queries but no mutations.
#[derive(Clone)]
struct TagService;

#[graphql]
impl TagService {
    /// List all tags
    pub fn list_tags(&self) -> Vec<String> {
        vec!["rust".to_string(), "graphql".to_string()]
    }

    /// Get tag count
    pub fn get_tag_count(&self) -> i32 {
        2
    }
}

/// Parent service that mounts a query-only child (no child mutations).
#[derive(Clone)]
struct BlogService {
    tag_service: TagService,
}

impl BlogService {
    fn new() -> Self {
        Self {
            tag_service: TagService,
        }
    }
}

#[graphql]
impl BlogService {
    /// Get blog title
    pub fn get_blog_title(&self) -> String {
        "My Blog".to_string()
    }

    /// Publish post
    pub fn publish_post(&self, title: String) -> String {
        format!("Published: {}", title)
    }

    /// Mount: expose TagService fields (queries only, no mutations)
    pub fn tags(&self) -> &TagService {
        &self.tag_service
    }
}

#[test]
fn test_graphql_mount_query_only_child_sdl() {
    let service = BlogService::new();
    let sdl = service.graphql_sdl();

    // Both parent and child query fields should be present
    assert!(
        sdl.contains("getBlogTitle"),
        "SDL should have getBlogTitle. SDL:\n{}",
        sdl
    );
    assert!(
        sdl.contains("listTags"),
        "SDL should have listTags from child TagService. SDL:\n{}",
        sdl
    );
    assert!(
        sdl.contains("getTagCount"),
        "SDL should have getTagCount from child TagService. SDL:\n{}",
        sdl
    );
    // Parent mutation should still be present
    assert!(
        sdl.contains("publishPost"),
        "SDL should have publishPost mutation. SDL:\n{}",
        sdl
    );
}

#[tokio::test]
async fn test_graphql_mount_query_only_child_dispatch() {
    let service = BlogService::new();
    let schema = service.graphql_schema();

    let result = schema.execute("{ listTags }").await;
    assert!(
        result.errors.is_empty(),
        "Child list query through parent should succeed: {:?}",
        result.errors
    );

    let data = result.data.into_json().unwrap();
    let tags = data["listTags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);
}

// ============================================================================
// #[serve] + #[graphql] Integration Test
//
// Verifies that a service annotated with both #[graphql] and #[serve(graphql)]
// produces a working axum router that responds to GraphQL introspection queries.
// ============================================================================

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[derive(Clone)]
struct ServeGraphqlService;

#[graphql]
#[serve(graphql)]
impl ServeGraphqlService {
    /// Get server version
    pub fn get_server_version(&self) -> String {
        "1.0.0".to_string()
    }

    /// Ping the server
    pub fn get_ping(&self) -> String {
        "pong".to_string()
    }
}

#[tokio::test]
async fn test_serve_graphql_router_responds() {
    let service = ServeGraphqlService;
    let router = service.router();

    // Send a basic introspection query to the /graphql endpoint
    let query = serde_json::json!({
        "query": "{ __typename }"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&query).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "GraphQL endpoint should return 200"
    );
}

#[tokio::test]
async fn test_serve_graphql_introspection_query() {
    let service = ServeGraphqlService;
    let router = service.router();

    let query = serde_json::json!({
        "query": "{ getServerVersion getping: getPing }"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&query).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert!(
        body["errors"].is_null() || body["errors"].as_array().map(|a| a.is_empty()).unwrap_or(true),
        "GraphQL response should have no errors: {}",
        body
    );
    assert!(
        body["data"].is_object(),
        "GraphQL response should have data: {}",
        body
    );
}

#[tokio::test]
async fn test_serve_graphql_health_endpoint() {
    let service = ServeGraphqlService;
    let router = service.router();

    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "#[serve] health endpoint should respond with 200"
    );
}

#[tokio::test]
async fn test_serve_graphql_openapi_spec() {
    let spec = ServeGraphqlService::combined_openapi_spec();

    // The spec should include the GraphQL endpoint paths
    let paths = &spec["paths"];
    assert!(
        paths.is_object(),
        "OpenAPI spec should have paths. Spec: {}",
        serde_json::to_string_pretty(&spec).unwrap()
    );

    // GraphQL endpoint should be documented
    assert!(
        paths["/graphql"].is_object(),
        "OpenAPI spec should document /graphql endpoint. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}
