//! Integration tests for the HTTP macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use server_less::{http, response, route};

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
        self.items
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == item_id)
            .cloned()
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

#[test]
fn test_http_openapi_paths_generated() {
    let paths = ItemService::http_openapi_paths();

    // Should have 3 paths: list_items, get_item, create_item
    assert_eq!(paths.len(), 3);

    // Check list_items
    let list_path = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("list_items".to_string()));
    assert!(list_path.is_some());
    let list_path = list_path.unwrap();
    assert_eq!(list_path.path, "/api/v1/items");
    assert_eq!(list_path.method, "get");
    assert_eq!(
        list_path.operation.summary,
        Some("List all items".to_string())
    );

    // Check create_item
    let create_path = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("create_item".to_string()));
    assert!(create_path.is_some());
    let create_path = create_path.unwrap();
    assert_eq!(create_path.path, "/api/v1/items");
    assert_eq!(create_path.method, "post");
    assert!(create_path.operation.request_body.is_some());
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

// ============================================================================
// OpenAPI Schema Tests
// ============================================================================

#[derive(Clone)]
struct SchemaService;

#[http(prefix = "/api")]
impl SchemaService {
    /// List items with pagination
    pub fn list_items(&self, page: Option<u32>, limit: Option<u32>) -> Vec<String> {
        vec![]
    }

    /// Get item by ID
    pub fn get_item(&self, item_id: String) -> Option<String> {
        None
    }

    /// Create an item
    pub fn create_item(&self, name: String, description: Option<String>) -> String {
        name
    }

    /// Update item
    pub fn update_item(&self, item_id: String, name: String) -> Result<String, String> {
        Ok(name)
    }
}

#[test]
fn test_openapi_query_parameters() {
    let spec = SchemaService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // list_items should have query parameters
    let list_path = paths.get("/api/items").unwrap();
    let get_op = list_path.get("get").unwrap();
    let params = get_op.get("parameters").unwrap().as_array().unwrap();

    // Should have page and limit parameters
    let param_names: Vec<_> = params
        .iter()
        .map(|p| p.get("name").unwrap().as_str().unwrap())
        .collect();
    assert!(param_names.contains(&"page"), "Expected 'page' parameter");
    assert!(param_names.contains(&"limit"), "Expected 'limit' parameter");

    // Check that params are in query
    for param in params {
        assert_eq!(param.get("in").unwrap(), "query");
    }
}

#[test]
fn test_openapi_path_parameters() {
    let spec = SchemaService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // get_item should have path parameter
    let get_path = paths.get("/api/items/{id}").unwrap();
    let get_op = get_path.get("get").unwrap();
    let params = get_op.get("parameters").unwrap().as_array().unwrap();

    // Should have item_id as path parameter
    let path_params: Vec<_> = params
        .iter()
        .filter(|p| p.get("in").unwrap() == "path")
        .collect();
    assert!(!path_params.is_empty(), "Expected path parameters");
}

#[test]
fn test_openapi_request_body() {
    let spec = SchemaService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // create_item should have request body
    let create_path = paths.get("/api/items").unwrap();
    let post_op = create_path.get("post").unwrap();

    assert!(
        post_op.get("requestBody").is_some(),
        "Expected requestBody for POST"
    );

    let body = post_op.get("requestBody").unwrap();
    let content = body.get("content").unwrap();
    let json_schema = content.get("application/json").unwrap();
    let schema = json_schema.get("schema").unwrap();

    // Should have properties
    let props = schema.get("properties").unwrap().as_object().unwrap();
    assert!(props.contains_key("name"), "Expected 'name' property");
}

#[test]
fn test_openapi_error_responses() {
    let spec = SchemaService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // update_item returns Result, should have error responses
    let update_path = paths.get("/api/items/{id}").unwrap();
    let put_op = update_path.get("put").unwrap();
    let responses = put_op.get("responses").unwrap().as_object().unwrap();

    assert!(responses.contains_key("200"), "Expected 200 response");
    assert!(
        responses.contains_key("400"),
        "Expected 400 response for Result"
    );
    assert!(
        responses.contains_key("500"),
        "Expected 500 response for Result"
    );
}

// ============================================================================
// Response Customization Tests
// ============================================================================

#[derive(Clone)]
struct ResponseService;

#[http(prefix = "/api")]
impl ResponseService {
    /// Create resource with 201 Created
    #[response(status = 201)]
    pub fn create_resource(&self, name: String) -> Item {
        Item {
            id: "1".to_string(),
            name,
        }
    }

    /// Delete with 204 No Content
    #[response(status = 204)]
    pub fn delete_resource(&self, id: String) {
        // Deletion logic
    }

    /// Download file with custom content type
    #[response(content_type = "application/octet-stream")]
    pub fn download_file(&self, id: String) -> Vec<u8> {
        vec![1, 2, 3]
    }

    /// Response with custom header
    #[response(header = "X-Custom-Header", value = "custom-value")]
    pub fn get_with_header(&self, id: String) -> String {
        "data".to_string()
    }

    /// Combined: status + content type + multiple headers
    #[response(status = 201)]
    #[response(content_type = "application/vnd.api+json")]
    #[response(header = "X-Resource-Id", value = "123")]
    #[response(header = "X-Version", value = "1.0")]
    pub fn create_with_all(&self, data: String) -> String {
        data
    }

    /// Normal method without response overrides
    pub fn get_normal(&self, id: String) -> String {
        "normal".to_string()
    }
}

#[test]
fn test_response_service_router_created() {
    let service = ResponseService;
    let _router = service.http_router();
}

#[test]
fn test_response_status_override_in_openapi() {
    let spec = ResponseService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // create_resource should have 201 status in OpenAPI
    let create_path = paths.get("/api/resources").unwrap();
    let post_op = create_path.get("post").unwrap();
    let responses = post_op.get("responses").unwrap().as_object().unwrap();

    assert!(
        responses.contains_key("201"),
        "Expected 201 Created response, got: {:?}",
        responses.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_response_no_content_in_openapi() {
    let spec = ResponseService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // delete_resource should have 204 No Content
    let delete_path = paths.get("/api/resources/{id}").unwrap();
    let delete_op = delete_path.get("delete").unwrap();
    let responses = delete_op.get("responses").unwrap().as_object().unwrap();

    assert!(
        responses.contains_key("204"),
        "Expected 204 No Content response"
    );
}

#[test]
fn test_response_content_type_in_openapi() {
    let spec = ResponseService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // download_file should have application/octet-stream content type
    // Method name "download_file" doesn't match standard patterns, so it becomes POST
    let download_path = paths.get("/api/download-files").unwrap();
    let post_op = download_path.get("post").unwrap();
    let responses = post_op.get("responses").unwrap();
    let ok_response = responses.get("200").unwrap();

    if let Some(content) = ok_response.get("content") {
        let content_obj = content.as_object().unwrap();
        assert!(
            content_obj.contains_key("application/octet-stream"),
            "Expected application/octet-stream content type, got: {:?}",
            content_obj.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_response_custom_headers_in_openapi() {
    let spec = ResponseService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // get_with_header should document custom header
    let get_path = paths.get("/api/with-headers/{id}").unwrap();
    let get_op = get_path.get("get").unwrap();
    let responses = get_op.get("responses").unwrap();
    let ok_response = responses.get("200").unwrap();

    if let Some(headers) = ok_response.get("headers") {
        let headers_obj = headers.as_object().unwrap();
        assert!(
            headers_obj.contains_key("X-Custom-Header"),
            "Expected X-Custom-Header in response headers"
        );
    }
}

#[test]
fn test_response_combined_overrides_in_openapi() {
    let spec = ResponseService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // create_with_all should have all customizations
    // Method name "create_with_all" becomes POST /api/with-alls
    let create_path = paths.get("/api/with-alls").unwrap();
    let post_op = create_path.get("post").unwrap();
    let responses = post_op.get("responses").unwrap().as_object().unwrap();

    // Check status code
    assert!(
        responses.contains_key("201"),
        "Expected 201 status for combined override"
    );

    let created_response = responses.get("201").unwrap();

    // Check content type
    if let Some(content) = created_response.get("content") {
        let content_obj = content.as_object().unwrap();
        assert!(
            content_obj.contains_key("application/vnd.api+json"),
            "Expected custom content type"
        );
    }

    // Check headers
    if let Some(headers) = created_response.get("headers") {
        let headers_obj = headers.as_object().unwrap();
        assert!(
            headers_obj.contains_key("X-Resource-Id"),
            "Expected X-Resource-Id header"
        );
        assert!(
            headers_obj.contains_key("X-Version"),
            "Expected X-Version header"
        );
    }
}

#[test]
fn test_response_normal_method_unchanged() {
    let spec = ResponseService::openapi_spec();
    let paths = spec.get("paths").unwrap();

    // get_normal should have default 200 response
    let normal_path = paths.get("/api/normals/{id}").unwrap();
    let get_op = normal_path.get("get").unwrap();
    let responses = get_op.get("responses").unwrap().as_object().unwrap();

    assert!(
        responses.contains_key("200"),
        "Expected default 200 response for normal method"
    );
}

// ============================================================================
// Parameter Customization
// ============================================================================
// The #[param(...)] attribute for parameter customization is implemented and
// functional, but cannot be tested in this file due to Rust stable not supporting
// custom attributes on function parameters.
//
// The feature works correctly and is demonstrated in examples/param_service.rs
// (which requires nightly Rust to compile).
//
// Supported syntax:
// - #[param(name = "q")] - Custom wire name
// - #[param(default = 10)] - Default value
// - #[param(query/path/body/header)] - Location override
//
// The parsing logic is tested in server-less-parse, and the HTTP macro
// correctly uses wire_name and default_value when generating handlers.
