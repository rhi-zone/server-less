//! Tests for the standalone #[openapi] macro.

#![allow(dead_code)]

use server_less::openapi;

// ============================================================================
// Standalone Mode Tests (no sibling protocols)
// ============================================================================

#[derive(Clone)]
struct StandaloneService;

#[openapi(prefix = "/api")]
impl StandaloneService {
    /// Get status
    pub fn get_status(&self) -> String {
        "ok".to_string()
    }

    /// List items
    pub fn list_items(&self) -> Vec<String> {
        vec![]
    }

    /// Create item
    pub fn create_item(&self, name: String) -> String {
        name
    }
}

#[test]
fn test_openapi_standalone_generates_spec() {
    let spec = StandaloneService::openapi_spec();

    assert_eq!(spec["openapi"], "3.0.0");
    assert_eq!(spec["info"]["title"], "StandaloneService");
}

#[test]
fn test_openapi_standalone_has_paths() {
    let spec = StandaloneService::openapi_spec();
    let paths = &spec["paths"];

    // Should have paths derived from method conventions
    assert!(paths.is_object(), "Should have paths object");

    // get_status -> GET /api/status
    assert!(
        paths["/api/status"]["get"].is_object(),
        "Should have GET /api/status. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );

    // list_items -> GET /api/items
    assert!(
        paths["/api/items"]["get"].is_object(),
        "Should have GET /api/items"
    );

    // create_item -> POST /api/item (or /api/items depending on convention)
    assert!(
        paths["/api/item"]["post"].is_object() || paths["/api/items"]["post"].is_object(),
        "Should have POST /api/item or /api/items. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}

// ============================================================================
// Protocol-Aware Mode Tests (with sibling protocols)
// ============================================================================

use server_less::{http, jsonrpc};

#[derive(Clone)]
struct ProtocolAwareService;

// NOTE: #[openapi] must come FIRST to detect sibling protocol attributes
#[openapi]
#[http(prefix = "/api", openapi = false)]
#[jsonrpc(path = "/rpc")]
impl ProtocolAwareService {
    /// Get status via HTTP
    pub fn get_status(&self) -> String {
        "ok".to_string()
    }

    /// Add numbers via JSON-RPC
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[test]
fn test_openapi_protocol_aware_detects_http() {
    let spec = ProtocolAwareService::openapi_spec();
    let paths = &spec["paths"];

    // Should have HTTP paths from #[http]
    assert!(
        paths["/api/status"]["get"].is_object(),
        "Should have HTTP endpoint. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}

#[test]
fn test_openapi_protocol_aware_detects_jsonrpc() {
    let spec = ProtocolAwareService::openapi_spec();
    let paths = &spec["paths"];

    // Should have JSON-RPC path from #[jsonrpc]
    assert!(
        paths["/rpc"]["post"].is_object(),
        "Should have JSON-RPC endpoint. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}

#[test]
fn test_openapi_protocol_aware_combined_spec() {
    let spec = ProtocolAwareService::openapi_spec();

    assert_eq!(spec["openapi"], "3.0.0");
    assert_eq!(spec["info"]["title"], "ProtocolAwareService");

    let paths = &spec["paths"];

    // Count total endpoints - should have both HTTP and JSON-RPC
    let path_count = paths.as_object().map(|o| o.len()).unwrap_or(0);
    assert!(
        path_count >= 2,
        "Should have at least 2 paths (HTTP + JSON-RPC). Got: {}",
        path_count
    );
}

// Test with WebSocket
use server_less::ws;

#[derive(Clone)]
struct HttpWsService;

#[openapi]
#[http(prefix = "/api", openapi = false)]
#[ws(path = "/ws")]
impl HttpWsService {
    pub fn get_info(&self) -> String {
        "info".to_string()
    }

    pub fn echo(&self, msg: String) -> String {
        msg
    }
}

#[test]
fn test_openapi_protocol_aware_with_ws() {
    let spec = HttpWsService::openapi_spec();
    let paths = &spec["paths"];

    // Should have HTTP path (get_info -> GET /api/infos)
    assert!(
        paths["/api/infos"]["get"].is_object(),
        "Should have HTTP endpoint. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );

    // Should have WebSocket path
    assert!(
        paths["/ws"]["get"].is_object(),
        "Should have WebSocket endpoint. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}

// Test with GraphQL
use server_less::graphql;

#[derive(Clone)]
struct HttpGraphqlService;

#[openapi]
#[http(prefix = "/api", openapi = false)]
#[graphql]
impl HttpGraphqlService {
    pub fn get_status(&self) -> String {
        "ok".to_string()
    }
}

#[test]
fn test_openapi_protocol_aware_with_graphql() {
    let spec = HttpGraphqlService::openapi_spec();
    let paths = &spec["paths"];

    // Should have HTTP path
    assert!(
        paths["/api/status"]["get"].is_object(),
        "Should have HTTP endpoint"
    );

    // Should have GraphQL paths
    assert!(
        paths["/graphql"]["post"].is_object(),
        "Should have GraphQL POST endpoint. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}

// Test HTTP-only with openapi (should still detect)
#[derive(Clone)]
struct HttpOnlyWithOpenapi;

#[openapi]
#[http(openapi = false)]
impl HttpOnlyWithOpenapi {
    pub fn list_things(&self) -> Vec<String> {
        vec![]
    }
}

#[test]
fn test_openapi_detects_single_protocol() {
    let spec = HttpOnlyWithOpenapi::openapi_spec();
    let paths = &spec["paths"];

    assert!(
        paths["/things"]["get"].is_object(),
        "Should have HTTP endpoint from detected #[http]. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}

// ============================================================================
// Enhanced Attributes Tests (tags, deprecated, description)
// ============================================================================

use server_less::response;
use server_less::route;

#[derive(Clone)]
struct EnhancedAttrsService;

#[openapi(prefix = "/api")]
impl EnhancedAttrsService {
    /// Get user by ID
    #[route(tags = "users,public", description = "Fetch a user by their unique ID")]
    pub fn get_user(&self, id: String) -> String {
        id
    }

    /// Create user (deprecated)
    #[route(tags = "users", deprecated)]
    #[response(description = "User created successfully")]
    pub fn create_user(&self, name: String) -> String {
        name
    }

    /// Hidden endpoint
    #[route(hidden)]
    pub fn internal_method(&self) -> String {
        "secret".to_string()
    }
}

#[test]
fn test_openapi_tags_attribute() {
    let spec = EnhancedAttrsService::openapi_spec();
    let paths = &spec["paths"];

    let get_user = &paths["/api/users/{id}"]["get"];
    assert!(get_user.is_object(), "Should have get_user endpoint");

    let tags = get_user["tags"].as_array();
    assert!(tags.is_some(), "Should have tags array");
    let tags = tags.unwrap();
    assert!(
        tags.iter().any(|t| t.as_str() == Some("users")),
        "Should have 'users' tag. Tags: {:?}",
        tags
    );
    assert!(
        tags.iter().any(|t| t.as_str() == Some("public")),
        "Should have 'public' tag. Tags: {:?}",
        tags
    );
}

#[test]
fn test_openapi_deprecated_attribute() {
    let spec = EnhancedAttrsService::openapi_spec();
    let paths = &spec["paths"];

    let create_user = &paths["/api/users"]["post"];
    assert!(create_user.is_object(), "Should have create_user endpoint");

    assert_eq!(
        create_user["deprecated"], true,
        "Should be marked as deprecated"
    );
}

#[test]
fn test_openapi_description_attribute() {
    let spec = EnhancedAttrsService::openapi_spec();
    let paths = &spec["paths"];

    let get_user = &paths["/api/users/{id}"]["get"];
    assert!(get_user.is_object(), "Should have get_user endpoint");

    assert_eq!(
        get_user["description"].as_str(),
        Some("Fetch a user by their unique ID"),
        "Should have description"
    );
}

#[test]
fn test_openapi_response_description_attribute() {
    let spec = EnhancedAttrsService::openapi_spec();
    let paths = &spec["paths"];

    let create_user = &paths["/api/users"]["post"];
    assert!(create_user.is_object(), "Should have create_user endpoint");

    let response_200 = &create_user["responses"]["200"];
    assert_eq!(
        response_200["description"].as_str(),
        Some("User created successfully"),
        "Should have custom response description"
    );
}

#[test]
fn test_openapi_hidden_excludes_from_spec() {
    let spec = EnhancedAttrsService::openapi_spec();
    let paths = &spec["paths"];

    // internal_method should NOT appear in the spec
    let internal = &paths["/api/internal-methods"]["get"];
    assert!(
        internal.is_null(),
        "Hidden endpoint should not appear in spec. Paths: {}",
        serde_json::to_string_pretty(paths).unwrap()
    );
}
