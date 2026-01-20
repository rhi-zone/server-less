//! Integration tests for the HTTP macro.

use serde::{Deserialize, Serialize};
use trellis::{http, route};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Item {
    id: String,
    name: String,
}

#[derive(Debug)]
#[allow(dead_code)]
enum ItemError {
    NotFound,
    Invalid,
}

impl std::fmt::Display for ItemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemError::NotFound => write!(f, "Item not found"),
            ItemError::Invalid => write!(f, "Invalid item"),
        }
    }
}

impl std::error::Error for ItemError {}

#[derive(Clone)]
struct ItemService {
    items: std::sync::Arc<std::sync::Mutex<Vec<Item>>>,
}

impl ItemService {
    fn new() -> Self {
        Self {
            items: std::sync::Arc::new(std::sync::Mutex::new(vec![Item {
                id: "1".to_string(),
                name: "Test".to_string(),
            }])),
        }
    }
}

#[http(prefix = "/api/v1")]
impl ItemService {
    /// List all items
    pub fn list_items(&self) -> Vec<Item> {
        self.items.lock().unwrap().clone()
    }

    /// Get item by ID
    pub fn get_item(&self, item_id: String) -> Option<Item> {
        self.items.lock().unwrap().iter().find(|i| i.id == item_id).cloned()
    }

    /// Create an item
    pub fn create_item(&self, name: String) -> Result<Item, ItemError> {
        if name.is_empty() {
            return Err(ItemError::Invalid);
        }
        let mut items = self.items.lock().unwrap();
        let item = Item {
            id: (items.len() + 1).to_string(),
            name,
        };
        items.push(item.clone());
        Ok(item)
    }
}

#[test]
fn test_http_router_created() {
    let service = ItemService::new();
    let _router = service.http_router();
    // Router is created successfully
}

#[test]
fn test_openapi_spec_generated() {
    let spec = ItemService::openapi_spec();

    // Check basic structure
    assert_eq!(spec.get("openapi").unwrap(), "3.0.0");

    let info = spec.get("info").unwrap();
    assert_eq!(info.get("title").unwrap(), "ItemService");

    // Check paths exist
    let paths = spec.get("paths").unwrap().as_object().unwrap();
    assert!(paths.contains_key("/api/v1/items"));
}

#[test]
fn test_openapi_contains_operations() {
    let spec = ItemService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // GET /api/v1/items should have get and post
    let items_path = paths.get("/api/v1/items").unwrap();
    assert!(items_path.get("get").is_some());
    assert!(items_path.get("post").is_some());

    // Check operation details
    let get_op = items_path.get("get").unwrap();
    assert_eq!(get_op.get("operationId").unwrap(), "list_items");
    assert_eq!(get_op.get("summary").unwrap(), "List all items");
}

// HTTP handler tests would require setting up an actual test server
// For comprehensive HTTP testing, consider using axum::TestClient
// These tests verify the macro generates the expected code structure

// ============================================================================
// Attribute Override Tests
// ============================================================================

#[derive(Clone)]
struct OverrideService;

#[http(prefix = "/api")]
impl OverrideService {
    /// Custom path method
    #[route(path = "/custom-endpoint")]
    pub fn my_method(&self) -> String {
        "custom".to_string()
    }

    /// Override to POST even though name suggests GET
    #[route(method = "POST")]
    pub fn get_data(&self, payload: String) -> String {
        payload
    }

    /// Both method and path override
    #[route(method = "PUT", path = "/special/{id}")]
    pub fn do_something(&self, id: String) -> String {
        id
    }

    /// Skipped method - should not appear in router or OpenAPI
    #[route(skip)]
    pub fn internal_helper(&self) -> String {
        "internal".to_string()
    }

    /// Hidden from OpenAPI but still in router
    #[route(hidden)]
    pub fn secret_endpoint(&self) -> String {
        "secret".to_string()
    }

    /// Normal inferred method
    pub fn list_things(&self) -> Vec<String> {
        vec![]
    }
}

#[test]
fn test_custom_path_in_openapi() {
    let spec = OverrideService::openapi_spec();
    let paths = spec.get("paths").unwrap().as_object().unwrap();

    // Custom path should be present
    assert!(
        paths.contains_key("/api/custom-endpoint"),
        "Expected /api/custom-endpoint, got: {:?}",
        paths.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_method_override_in_openapi() {
    let spec = OverrideService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // get_data should be POST despite the name
    let data_path = paths.get("/api/datas").unwrap();
    assert!(
        data_path.get("post").is_some(),
        "Expected POST for get_data, got: {:?}",
        data_path
    );
}

#[test]
fn test_combined_override_in_openapi() {
    let spec = OverrideService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // do_something should be PUT at /special/{id}
    let special_path = paths.get("/api/special/{id}").unwrap();
    assert!(
        special_path.get("put").is_some(),
        "Expected PUT for do_something, got: {:?}",
        special_path
    );
}

#[test]
fn test_skipped_method_not_in_openapi() {
    let spec = OverrideService::openapi_spec();
    let paths = spec.get("paths").unwrap().as_object().unwrap();

    // internal_helper should not generate any path
    for (path, _) in paths {
        assert!(
            !path.contains("internal") && !path.contains("helper"),
            "Skipped method should not appear in OpenAPI: {}",
            path
        );
    }
}

#[test]
fn test_hidden_method_not_in_openapi() {
    let spec = OverrideService::openapi_spec();
    let paths = spec.get("paths").unwrap().as_object().unwrap();

    // secret_endpoint should not appear in OpenAPI
    for (path, _) in paths {
        assert!(
            !path.contains("secret"),
            "Hidden method should not appear in OpenAPI: {}",
            path
        );
    }
}

#[test]
fn test_normal_method_still_works() {
    let spec = OverrideService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // list_things should be inferred as GET /things
    assert!(
        paths.get("/api/things").is_some(),
        "list_things should generate /api/things"
    );
}

#[test]
fn test_override_service_router_created() {
    let service = OverrideService;
    let _router = service.http_router();
}
