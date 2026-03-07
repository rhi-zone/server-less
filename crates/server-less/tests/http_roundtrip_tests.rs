//! HTTP round-trip tests using tower::ServiceExt::oneshot.
//!
//! These tests verify actual request → response behavior of generated HTTP handlers,
//! complementing the macro expansion tests in http_tests.rs.

#![allow(dead_code)]
// `response` is an attribute macro consumed by #[http]; after strip_http_attrs
// removes it from the re-emitted impl block the import appears unused to rustc.
#![allow(unused_imports)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde::{Deserialize, Serialize};
use server_less::{http, response, IntoErrorCode as _};
use tower::ServiceExt;

// Helper to read a response body as JSON
async fn body_json<T: serde::de::DeserializeOwned>(response: axum::http::Response<Body>) -> T {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn body_string(response: axum::http::Response<Body>) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// ============================================================================
// Test Service
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Item {
    id: String,
    name: String,
}

#[derive(Debug, server_less::ServerlessError)]
enum ItemError {
    #[error(code = NotFound, message = "Item not found")]
    NotFound,
    #[error(code = InvalidInput, message = "Invalid item")]
    Invalid,
}

#[derive(Clone)]
struct ItemService {
    items: std::sync::Arc<std::sync::Mutex<Vec<Item>>>,
}

impl ItemService {
    fn new() -> Self {
        Self {
            items: std::sync::Arc::new(std::sync::Mutex::new(vec![
                Item {
                    id: "1".to_string(),
                    name: "Alpha".to_string(),
                },
                Item {
                    id: "2".to_string(),
                    name: "Beta".to_string(),
                },
            ])),
        }
    }
}

#[http(prefix = "/api")]
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

    /// Delete an item
    pub fn delete_item(&self, item_id: String) {
        self.items.lock().unwrap().retain(|i| i.id != item_id);
    }

    /// Update an item
    pub fn update_item(&self, item_id: String, name: String) -> Result<Item, ItemError> {
        let mut items = self.items.lock().unwrap();
        if let Some(item) = items.iter_mut().find(|i| i.id == item_id) {
            item.name = name.clone();
            Ok(item.clone())
        } else {
            Err(ItemError::NotFound)
        }
    }
}

fn app() -> axum::Router {
    ItemService::new().http_router()
}

// ============================================================================
// GET Tests
// ============================================================================

#[tokio::test]
async fn test_get_list() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let items: Vec<Item> = body_json(response).await;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "Alpha");
    assert_eq!(items[1].name, "Beta");
}

#[tokio::test]
async fn test_get_by_id() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/items/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let item: Item = body_json(response).await;
    assert_eq!(item.id, "1");
    assert_eq!(item.name, "Alpha");
}

#[tokio::test]
async fn test_get_by_id_not_found() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/items/999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// POST Tests
// ============================================================================

#[tokio::test]
async fn test_post_create() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Gamma"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let item: Item = body_json(response).await;
    assert_eq!(item.name, "Gamma");
    assert_eq!(item.id, "3");
}

#[tokio::test]
async fn test_post_create_validation_error() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/items")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // ItemError::Invalid → #[error(code = InvalidInput)] → 400
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = body_json(response).await;
    assert!(body.get("error").is_some());
    assert!(body.get("message").is_some());
}

// ============================================================================
// PUT Tests
// ============================================================================

#[tokio::test]
async fn test_put_update() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/items/1")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Updated"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let item: Item = body_json(response).await;
    assert_eq!(item.name, "Updated");
    assert_eq!(item.id, "1");
}

#[tokio::test]
async fn test_put_update_not_found() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/items/999")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Nope"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // ItemError::NotFound → #[error(code = NotFound)] → 404
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// DELETE Tests
// ============================================================================

#[tokio::test]
async fn test_delete() {
    let service = ItemService::new();
    let router = service.clone().http_router();

    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/items/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Unit return → 204 No Content
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify item was removed
    let items = service.items.lock().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "2");
}

// ============================================================================
// 404 for Unknown Routes
// ============================================================================

#[tokio::test]
async fn test_unknown_route() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Response Customization
// ============================================================================

#[derive(Clone)]
struct CustomResponseService;

#[http(prefix = "/api")]
impl CustomResponseService {
    /// Create with 201
    #[response(status = 201)]
    pub fn create_resource(&self, name: String) -> String {
        name
    }

    /// Delete with 204
    #[response(status = 204)]
    pub fn delete_resource(&self, _id: String) {
        // no-op
    }

    /// Response with custom header
    #[response(header = "x-custom", value = "hello")]
    pub fn get_resource(&self, id: String) -> String {
        format!("resource-{}", id)
    }
}

#[tokio::test]
async fn test_custom_status_201() {
    let router = CustomResponseService.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/resources")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"new"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_custom_header() {
    let router = CustomResponseService.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/resources/42")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("x-custom").unwrap(), "hello");
}

// ============================================================================
// Query Parameter Tests
// ============================================================================

#[derive(Clone)]
struct SearchService;

#[http(prefix = "/api")]
impl SearchService {
    /// Search with query params
    pub fn search_items(&self, q: String, limit: Option<u32>) -> Vec<String> {
        let limit = limit.unwrap_or(10) as usize;
        vec![q; limit.min(3)]
    }
}

#[tokio::test]
async fn test_query_params() {
    let router = SearchService.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/items?q=hello&limit=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let results: Vec<String> = body_json(response).await;
    assert_eq!(results, vec!["hello", "hello"]);
}

#[tokio::test]
async fn test_query_params_optional_missing() {
    let router = SearchService.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/items?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let results: Vec<String> = body_json(response).await;
    // limit defaults to 10, capped at 3 by min()
    assert_eq!(results, vec!["test", "test", "test"]);
}

// ============================================================================
// Mount Point Tests
// ============================================================================

#[derive(Clone)]
struct ChildApi;

#[http]
impl ChildApi {
    /// List child resources
    fn list_things(&self) -> Vec<String> {
        vec!["child-thing".to_string()]
    }
}

#[derive(Clone)]
struct ParentApp {
    child: ChildApi,
}

#[http]
impl ParentApp {
    /// Health check
    fn get_health(&self) -> String {
        "ok".to_string()
    }

    /// Mount child
    fn child(&self) -> &ChildApi {
        &self.child
    }
}

#[tokio::test]
async fn test_mount_parent_route() {
    let router = ParentApp { child: ChildApi }.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/healths")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response).await;
    assert!(body.contains("ok"));
}

#[tokio::test]
async fn test_mount_child_route() {
    let router = ParentApp { child: ChildApi }.http_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/child/things")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let things: Vec<String> = body_json(response).await;
    assert_eq!(things, vec!["child-thing"]);
}
