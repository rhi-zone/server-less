//! Integration tests for the GraphQL macro.

use trellis::graphql;

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
