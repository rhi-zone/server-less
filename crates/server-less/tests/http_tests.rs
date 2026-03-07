//! Integration tests for the HTTP macro.

#![allow(dead_code)]
#![allow(unused_variables)]
// `response` and `route` are attribute macros consumed by #[http]; after
// strip_http_attrs removes them from the re-emitted impl block, they no longer
// appear in the compiled output, so rustc cannot see them as "used".  The
// imports are still needed at the source level for the macro attribute syntax.
#![allow(unused_imports)]

use serde::{Deserialize, Serialize};
use server_less::{http, response, route, server};
#[allow(unused_imports)]
use server_less::IntoErrorCode as _;

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
// server(skip) Tests
// ============================================================================

#[derive(Clone)]
struct ServerSkipService;

#[http(prefix = "/api")]
impl ServerSkipService {
    /// Public endpoint — must appear in routes and OpenAPI
    pub fn get_public(&self) -> String {
        "public".to_string()
    }

    /// Internal helper — must NOT appear in routes or OpenAPI
    #[server(skip)]
    pub fn get_internal(&self) -> String {
        "internal".to_string()
    }
}

#[test]
fn test_server_skip_not_in_openapi() {
    let spec = ServerSkipService::openapi_spec();
    let paths = spec.get("paths").unwrap().as_object().unwrap();

    // Public method should appear
    assert!(
        paths.contains_key("/api/publics"),
        "get_public should generate /api/publics, got: {:?}",
        paths.keys().collect::<Vec<_>>()
    );

    // Skipped method must not appear in any path
    for path_key in paths.keys() {
        assert!(
            !path_key.contains("internal"),
            "#[server(skip)] method must not appear in OpenAPI paths: {}",
            path_key
        );
    }
}

#[test]
fn test_server_skip_router_still_created() {
    let service = ServerSkipService;
    let _router = service.http_router();
}

// ============================================================================
// Mount Point Tests
// ============================================================================

/// Child service for HTTP mount testing
#[derive(Clone)]
struct UserApi;

#[http]
impl UserApi {
    /// List all users
    fn list_users(&self) -> Vec<String> {
        vec!["alice".to_string(), "bob".to_string()]
    }

    /// Get a user by ID
    fn get_user(&self, id: String) -> String {
        format!("User: {}", id)
    }

    /// Create a user
    fn create_user(&self, name: String) -> String {
        format!("Created: {}", name)
    }
}

/// Another child service
#[derive(Clone)]
struct PostApi;

#[http]
impl PostApi {
    /// List all posts
    fn list_posts(&self) -> Vec<String> {
        vec!["post1".to_string()]
    }
}

/// Parent with static mounts
#[derive(Clone)]
struct HttpApp {
    user_api: UserApi,
    post_api: PostApi,
}

#[http]
impl HttpApp {
    /// Health check
    fn get_health(&self) -> String {
        "ok".to_string()
    }

    /// Mount user API
    fn users(&self) -> &UserApi {
        &self.user_api
    }

    /// Mount post API
    fn posts(&self) -> &PostApi {
        &self.post_api
    }
}

#[test]
fn test_http_static_mount_router_created() {
    let app = HttpApp {
        user_api: UserApi,
        post_api: PostApi,
    };
    let _router = app.http_router();
}

#[test]
fn test_http_static_mount_openapi_paths() {
    let spec = HttpApp::openapi_spec();
    let paths = spec.get("paths").unwrap().as_object().unwrap();

    let path_keys: Vec<_> = paths.keys().collect();

    // Leaf path on parent (get_health → GET /healths)
    assert!(
        paths.contains_key("/healths"),
        "Expected /healths path, got: {:?}",
        path_keys
    );

    // Mounted child paths should be prefixed with mount name.
    // UserApi mounts at /users: list_users → GET /users/users, get_user → GET /users/users/{id}
    assert!(
        path_keys.iter().any(|p| p.starts_with("/users/")),
        "Expected child paths under /users/, got: {:?}",
        path_keys
    );

    // PostApi mounts at /posts: list_posts → GET /posts/posts
    assert!(
        path_keys.iter().any(|p| p.starts_with("/posts/")),
        "Expected child paths under /posts/, got: {:?}",
        path_keys
    );
}

/// Verify http_openapi_paths() directly includes mounted child paths.
#[test]
fn test_http_mount_openapi_paths_composed() {
    let paths = HttpApp::http_openapi_paths();
    let path_strs: Vec<&str> = paths.iter().map(|p| p.path.as_str()).collect();

    assert!(
        path_strs.iter().any(|p| p.starts_with("/users/")),
        "Mounted UserApi paths should appear under /users/: {:?}",
        path_strs
    );
    assert!(
        path_strs.iter().any(|p| p.starts_with("/posts/")),
        "Mounted PostApi paths should appear under /posts/: {:?}",
        path_strs
    );
    // Parent leaf path
    assert!(
        path_strs.contains(&"/healths"),
        "Parent leaf path /healths should be included: {:?}",
        path_strs
    );
}

/// HttpMount trait test
#[test]
fn test_http_mount_trait_implemented() {
    use server_less::HttpMount;

    // Verify the trait is implemented and the router can be created
    let router = <UserApi as HttpMount>::http_mount_router(std::sync::Arc::new(UserApi));
    let _ = router; // Router creation succeeds
}

// ============================================================================
// Parameter Customization Tests
// ============================================================================
// Custom attributes on function parameters (#[param(...)]) have been stable
// since Rust 1.63 (edition 2021+). This crate uses edition 2024 / MSRV 1.89,
// so all #[param] attributes can be tested here on stable without any caveats.

#[derive(Clone)]
struct HiddenHttpService;

#[http]
impl HiddenHttpService {
    /// Public endpoint
    pub fn get_public(&self) -> String {
        "public".to_string()
    }

    /// Hidden endpoint - routable but absent from OpenAPI spec
    #[server(hidden)]
    pub fn get_hidden(&self) -> String {
        "hidden".to_string()
    }
}

#[test]
fn test_http_server_hidden_not_in_openapi_paths() {
    let paths = HiddenHttpService::http_openapi_paths();
    // Only the public endpoint appears in OpenAPI
    let path_keys: Vec<_> = paths.iter().map(|p| p.path.as_str()).collect();
    assert!(
        path_keys.iter().any(|p| p.contains("public")),
        "public endpoint must appear in OpenAPI paths"
    );
    assert!(
        !path_keys.iter().any(|p| p.contains("hidden")),
        "hidden endpoint must not appear in OpenAPI paths"
    );
}

#[test]
fn test_http_server_hidden_not_in_openapi_spec() {
    let spec = HiddenHttpService::openapi_spec();
    let paths = spec.get("paths").unwrap().as_object().unwrap();
    // The spec paths must not mention the hidden endpoint
    let paths_str = serde_json::to_string(paths).unwrap();
    assert!(!paths_str.contains("hidden"), "hidden endpoint must not appear in openapi_spec paths");
}

#[test]
fn test_http_server_hidden_router_is_created() {
    // The router should still be created successfully (method is still routable)
    let svc = HiddenHttpService;
    let _router = svc.http_router();
}

// ============================================================================
// IntoErrorCode / HTTP Status Code Tests
// ============================================================================

/// Error type that uses `#[error(code = 422)]` to explicitly set the HTTP status.
#[derive(Debug, server_less::ServerlessError)]
enum ValidationError {
    #[error(code = 422, message = "Input failed validation")]
    InputInvalid,
    #[error(code = 409, message = "Resource already exists")]
    AlreadyExists,
}

#[derive(Clone)]
struct ValidationService;

#[http(prefix = "/api")]
impl ValidationService {
    /// Trigger a 422 validation error
    pub fn create_validated(&self, fail: bool) -> Result<String, ValidationError> {
        if fail {
            Err(ValidationError::InputInvalid)
        } else {
            Ok("ok".to_string())
        }
    }

    /// Trigger a 409 conflict error
    pub fn create_unique(&self, exists: bool) -> Result<String, ValidationError> {
        if exists {
            Err(ValidationError::AlreadyExists)
        } else {
            Ok("created".to_string())
        }
    }
}

#[tokio::test]
async fn test_serverless_error_code_422_maps_to_http_422() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let router = ValidationService.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/validateds")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"fail":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Expected HTTP 422 from #[error(code = 422)], got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_serverless_error_code_409_maps_to_http_409() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let router = ValidationService.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/uniques")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"exists":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::CONFLICT,
        "Expected HTTP 409 from #[error(code = 409)], got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_serverless_error_ok_returns_200() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let router = ValidationService.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/validateds")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"fail":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected HTTP 200 for Ok case, got: {}",
        response.status()
    );
}

// Custom attributes on function parameters (#[param(...)]) have been stable
// since Rust 1.63 (edition 2021+). This crate uses edition 2024 / MSRV 1.89,
// so all #[param] attributes can be tested here on stable without any caveats.

#[derive(Clone)]
struct ParamService;

#[http(prefix = "/api")]
impl ParamService {
    /// Search with custom wire name
    ///
    /// Uses #[param(name = "q")] to map `query` → wire key "q".
    pub fn search_items(
        &self,
        #[param(name = "q")] query: String,
    ) -> Vec<String> {
        vec![query]
    }

    /// Explicit query location
    ///
    /// Forces `filter` into the query string even on a POST-like name.
    #[route(method = "GET", path = "/param-query")]
    pub fn get_param_query(
        &self,
        #[param(query)] filter: String,
    ) -> String {
        filter
    }

    /// Explicit path location
    ///
    /// Forces `slug` to be extracted from the path segment.
    #[route(path = "/by-slug/{slug}")]
    pub fn get_by_slug(
        &self,
        #[param(path)] slug: String,
    ) -> Option<String> {
        Some(slug)
    }

    /// Explicit body location on a GET-inferred method
    ///
    /// Overrides the default (query) to body for `payload`.
    #[route(method = "POST", path = "/param-body")]
    pub fn post_param_body(
        &self,
        #[param(body)] payload: String,
    ) -> String {
        payload
    }

    /// Default value when query param absent
    ///
    /// `page` defaults to 1 and `size` defaults to 20 if not supplied.
    pub fn list_with_defaults(
        &self,
        #[param(default = 1)] page: u32,
        #[param(default = 20)] size: u32,
    ) -> Vec<String> {
        vec![format!("page={page},size={size}")]
    }

    /// Header-sourced parameter
    ///
    /// `api_key` is read from the `X-Api-Key` header.
    pub fn get_secured(
        &self,
        #[param(header, name = "X-Api-Key")] api_key: Option<String>,
    ) -> String {
        api_key.unwrap_or_default()
    }

    /// Help text annotation
    ///
    /// #[param(help = "...")] carries description metadata parsed into
    /// ParamInfo::help_text and forwarded to OpenApiParameter::description.
    pub fn find_users(
        &self,
        #[param(help = "Substring to match against user names")] name: Option<String>,
    ) -> Vec<String> {
        vec![name.unwrap_or_default()]
    }
}

// ── compile / router-creation tests ──────────────────────────────────────────

#[test]
fn test_param_service_router_created() {
    let service = ParamService;
    let _router = service.http_router();
}

// ── OpenAPI structural tests ──────────────────────────────────────────────────

/// #[param(name = "q")] — the OpenAPI parameter name should be "q", not "query".
#[test]
fn test_param_custom_wire_name_in_openapi() {
    let paths = ParamService::http_openapi_paths();

    let search = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("search_items".to_string()))
        .expect("search_items path missing");

    let param_names: Vec<&str> = search
        .operation
        .parameters
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    assert!(
        param_names.contains(&"q"),
        "#[param(name = \"q\")] should rename the wire parameter to 'q'; got: {:?}",
        param_names
    );
    assert!(
        !param_names.contains(&"query"),
        "original Rust name 'query' should not appear as the wire name; got: {:?}",
        param_names
    );
}

/// #[param(query)] — parameter should appear as a query parameter in OpenAPI.
#[test]
fn test_param_explicit_query_in_openapi() {
    let paths = ParamService::http_openapi_paths();

    let route = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("get_param_query".to_string()))
        .expect("get_param_query path missing");

    let filter_param = route
        .operation
        .parameters
        .iter()
        .find(|p| p.name == "filter")
        .expect("'filter' parameter missing from get_param_query OpenAPI spec");

    assert_eq!(
        filter_param.location, "query",
        "#[param(query)] should place the parameter in query location; got: {:?}",
        filter_param.location
    );
}

/// #[param(path)] — parameter should appear as a path parameter in OpenAPI.
#[test]
fn test_param_explicit_path_in_openapi() {
    let paths = ParamService::http_openapi_paths();

    let route = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("get_by_slug".to_string()))
        .expect("get_by_slug path missing");

    let slug_param = route
        .operation
        .parameters
        .iter()
        .find(|p| p.name == "slug")
        .expect("'slug' parameter missing from get_by_slug OpenAPI spec");

    assert_eq!(
        slug_param.location, "path",
        "#[param(path)] should place the parameter in path location; got: {:?}",
        slug_param.location
    );
    assert!(
        slug_param.required,
        "path parameters must be required in OpenAPI"
    );
}

/// #[param(body)] — parameter should appear in the request body, not parameters list.
#[test]
fn test_param_explicit_body_in_openapi() {
    let paths = ParamService::http_openapi_paths();

    let route = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("post_param_body".to_string()))
        .expect("post_param_body path missing");

    // Body parameters are encoded in requestBody, not the parameters array.
    assert!(
        route.operation.request_body.is_some(),
        "#[param(body)] should produce a requestBody; got: {:?}",
        route.operation
    );

    // The parameters array should NOT contain 'payload'.
    let param_names: Vec<&str> = route
        .operation
        .parameters
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert!(
        !param_names.contains(&"payload"),
        "#[param(body)] param should not appear in parameters array; got: {:?}",
        param_names
    );

    // Verify the body schema includes the 'payload' property.
    let body = route.operation.request_body.as_ref().unwrap();
    let schema = body
        .get("content")
        .and_then(|c| c.get("application/json"))
        .and_then(|j| j.get("schema"))
        .and_then(|s| s.get("properties"))
        .expect("requestBody should have content.application/json.schema.properties");

    assert!(
        schema.get("payload").is_some(),
        "'payload' should be a property in the request body schema; got: {:?}",
        schema
    );
}

/// #[param(default = N)] — parameter should be marked optional (not required) in OpenAPI
/// because a default value means the caller may omit it.
#[test]
fn test_param_default_value_not_required_in_openapi() {
    let paths = ParamService::http_openapi_paths();

    let route = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("list_with_defaults".to_string()))
        .expect("list_with_defaults path missing");

    for param in &route.operation.parameters {
        if param.name == "page" || param.name == "size" {
            assert!(
                !param.required,
                "#[param(default = ...)] parameter '{}' should be optional in OpenAPI; \
                 got required=true",
                param.name
            );
        }
    }

    let param_names: Vec<&str> = route
        .operation
        .parameters
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert!(
        param_names.contains(&"page"),
        "expected 'page' parameter; got: {:?}",
        param_names
    );
    assert!(
        param_names.contains(&"size"),
        "expected 'size' parameter; got: {:?}",
        param_names
    );
}

/// #[param(header, name = "X-Api-Key")] — parameter should appear as a header
/// parameter with the correct wire name in the OpenAPI spec.
#[test]
fn test_param_header_in_openapi() {
    let paths = ParamService::http_openapi_paths();

    let route = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("get_secured".to_string()))
        .expect("get_secured path missing");

    let header_param = route
        .operation
        .parameters
        .iter()
        .find(|p| p.name == "X-Api-Key")
        .expect("'X-Api-Key' header parameter missing from get_secured OpenAPI spec");

    assert_eq!(
        header_param.location, "header",
        "#[param(header)] should place the parameter in header location; got: {:?}",
        header_param.location
    );
}

/// #[param(help = "...")] — the route compiles and appears in the OpenAPI spec.
///
#[test]
fn test_param_help_route_in_openapi() {
    let paths = ParamService::http_openapi_paths();

    let route = paths
        .iter()
        .find(|p| p.operation.operation_id == Some("find_users".to_string()))
        .expect("find_users path missing");

    // The parameter must at least appear in the OpenAPI spec.
    let name_param = route
        .operation
        .parameters
        .iter()
        .find(|p| p.name == "name");
    assert!(
        name_param.is_some(),
        "'name' parameter should appear in find_users OpenAPI spec; got: {:?}",
        route.operation.parameters
    );

    assert_eq!(
        name_param.unwrap().description.as_deref(),
        Some("Substring to match against user names"),
        "#[param(help = \"...\")] should populate the OpenAPI parameter description"
    );
}

// ============================================================================
// #[http(debug = true)] Tests
// ============================================================================

/// Service with debug enabled on the impl block — all methods get debug logging.
#[derive(Clone)]
struct DebugService;

#[http(debug = true)]
impl DebugService {
    /// List items (debug on whole impl block)
    pub fn list_debug_items(&self) -> Vec<String> {
        vec!["a".to_string(), "b".to_string()]
    }

    /// Get item by ID (also gets debug from impl block)
    pub fn get_debug_item(&self, item_id: String) -> Option<String> {
        if item_id == "1" {
            Some("found".to_string())
        } else {
            None
        }
    }
}

#[test]
fn test_debug_impl_block_compiles_and_router_created() {
    // Verifies that #[http(debug = true)] on an impl block compiles without error
    // and produces a working router.
    let service = DebugService;
    let _router = service.http_router();
}

#[test]
fn test_debug_impl_block_openapi_still_works() {
    // Debug flag must not interfere with OpenAPI generation.
    let spec = DebugService::openapi_spec();
    assert_eq!(spec.get("openapi").unwrap(), "3.0.0");
    let paths = spec.get("paths").unwrap().as_object().unwrap();
    // list_debug_items → GET /debug_items, get_debug_item → GET /debug_items/{id}
    assert!(
        !paths.is_empty(),
        "Expected OpenAPI paths from DebugService; got empty"
    );
}

/// Service with debug enabled only on one method via per-method attribute.
#[derive(Clone)]
struct PerMethodDebugService;

#[http]
impl PerMethodDebugService {
    /// This method has debug logging enabled.
    #[http(debug = true)]
    pub fn list_verbose_items(&self) -> Vec<String> {
        vec!["x".to_string()]
    }

    /// This method does NOT have debug logging.
    pub fn list_quiet_items(&self) -> Vec<String> {
        vec!["y".to_string()]
    }
}

#[test]
fn test_per_method_debug_compiles_and_router_created() {
    // Verifies that #[http(debug = true)] on a single method compiles without error.
    let service = PerMethodDebugService;
    let _router = service.http_router();
}

#[test]
fn test_per_method_debug_openapi_still_works() {
    // Both methods should still appear in OpenAPI regardless of debug flag.
    let paths = PerMethodDebugService::http_openapi_paths();
    assert_eq!(
        paths.len(),
        2,
        "Both methods should appear in OpenAPI; got: {:?}",
        paths
            .iter()
            .map(|p| p.operation.operation_id.as_deref().unwrap_or("?"))
            .collect::<Vec<_>>()
    );
}

// ============================================================================
// #[http(trace = true)] Tests
// ============================================================================

/// Service with trace enabled on the impl block — all methods get param tracing.
#[derive(Clone)]
struct TraceService;

#[http(prefix = "/trace", trace = true)]
impl TraceService {
    /// List items (trace on whole impl block)
    pub fn list_trace_items(&self) -> Vec<String> {
        vec!["a".to_string(), "b".to_string()]
    }

    /// Get item by ID (also gets trace from impl block)
    pub fn get_trace_item(&self, item_id: String) -> Option<String> {
        if item_id == "1" {
            Some("found".to_string())
        } else {
            None
        }
    }

    /// Filter items with query params (tests query param tracing)
    #[route(path = "/filtered-trace-items")]
    pub fn find_trace_items(&self, query: String, limit: Option<u32>) -> Vec<String> {
        vec![format!("{}:{}", query, limit.unwrap_or(10))]
    }
}

#[test]
fn test_trace_impl_block_compiles_and_router_created() {
    // Verifies that #[http(trace = true)] on an impl block compiles without error
    // and produces a working router.
    let service = TraceService;
    let _router = service.http_router();
}

#[test]
fn test_trace_impl_block_openapi_still_works() {
    // Trace flag must not interfere with OpenAPI generation.
    let spec = TraceService::openapi_spec();
    assert_eq!(spec.get("openapi").unwrap(), "3.0.0");
    let paths = spec.get("paths").unwrap().as_object().unwrap();
    assert!(
        !paths.is_empty(),
        "Expected OpenAPI paths from TraceService; got empty"
    );
}

/// Service with trace enabled only on one method via per-method attribute.
#[derive(Clone)]
struct PerMethodTraceService;

#[http]
impl PerMethodTraceService {
    /// This method has trace logging enabled.
    #[http(trace = true)]
    pub fn list_verbose_trace(&self, kind: String) -> Vec<String> {
        vec![kind]
    }

    /// This method does NOT have trace logging.
    pub fn list_quiet_trace(&self) -> Vec<String> {
        vec!["y".to_string()]
    }
}

#[test]
fn test_per_method_trace_compiles_and_router_created() {
    // Verifies that #[http(trace = true)] on a single method compiles without error.
    let service = PerMethodTraceService;
    let _router = service.http_router();
}

#[test]
fn test_per_method_trace_openapi_still_works() {
    // Both methods should still appear in OpenAPI regardless of trace flag.
    let paths = PerMethodTraceService::http_openapi_paths();
    assert_eq!(
        paths.len(),
        2,
        "Both methods should appear in OpenAPI; got: {:?}",
        paths
            .iter()
            .map(|p| p.operation.operation_id.as_deref().unwrap_or("?"))
            .collect::<Vec<_>>()
    );
}

/// Verify that a traced handler actually returns the correct value at runtime.
#[tokio::test]
async fn test_trace_handler_returns_correct_response() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let router = TraceService.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/trace/trace-items/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Traced handler should return 200 OK; got: {}",
        response.status()
    );
}
