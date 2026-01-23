//! Tests for Context injection in protocol macros.
//!
//! This tests the automatic injection of server_less::Context into method handlers.

#![allow(dead_code)]
#![allow(unused_variables)]

use serde::{Deserialize, Serialize};
use server_less::{http, jsonrpc, ws};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
}

/// Test service using qualified Context (explicit)
#[derive(Clone)]
struct UserService;

#[http]
impl UserService {
    /// Create a user with context access
    pub fn create_user(&self, ctx: server_less::Context, name: String) -> User {
        // Should have access to context methods
        let request_id = ctx.request_id().unwrap_or("unknown");
        User {
            id: request_id.to_string(),
            name,
        }
    }

    /// Get user without context
    pub fn get_user(&self, user_id: String) -> Option<User> {
        Some(User {
            id: user_id,
            name: "Test".to_string(),
        })
    }

    /// Method with context and multiple params
    pub fn update_user(&self, ctx: server_less::Context, user_id: String, name: String) -> User {
        let request_id = ctx.request_id().unwrap_or("unknown");
        User { id: user_id, name }
    }
}

#[test]
fn test_context_injection_compiles() {
    // If this compiles, Context injection is working
    let service = UserService;
    let _router = service.http_router();
}

#[test]
fn test_openapi_with_context() {
    // Context should not appear in OpenAPI spec (it's injected, not user input)
    let spec = UserService::openapi_spec();

    let paths = spec.get("paths").unwrap().as_object().unwrap();
    let create_path = paths.get("/users").unwrap().as_object().unwrap();
    let post = create_path.get("post").unwrap().as_object().unwrap();

    // Check parameters - should only have 'name', not 'ctx'
    let params = post
        .get("requestBody")
        .and_then(|rb| rb.get("content"))
        .and_then(|c| c.get("application/json"))
        .and_then(|aj| aj.get("schema"))
        .and_then(|s| s.get("properties"))
        .and_then(|p| p.as_object());

    if let Some(props) = params {
        assert!(props.contains_key("name"), "Should have 'name' parameter");
        assert!(!props.contains_key("ctx"), "Should NOT expose 'ctx' in API");
    }
}

/// Test collision case - qualified Context
///
/// This demonstrates the two-pass detection:
/// - eval_with_auth uses qualified server_less::Context (injected)
/// - eval_no_context avoids the collision by not using Context at all
#[derive(Clone)]
struct InterpreterService;

#[http(prefix = "/api")]
impl InterpreterService {
    /// Uses server-less Context (qualified) - will be injected
    pub fn eval_with_auth(&self, ctx: server_less::Context, expr: String) -> String {
        let user = ctx.user_id().unwrap_or("anonymous");
        format!("User {} evaluating: {}", user, expr)
    }

    /// Doesn't use Context to avoid collision
    pub fn eval_no_context(&self, expr: String) -> String {
        format!("Evaluating: {}", expr)
    }
}

#[test]
fn test_qualified_context_collision() {
    // If this compiles, our two-pass detection works
    let service = InterpreterService;
    let _router = service.http_router();
}

/// Test with no Context at all
#[derive(Clone)]
struct SimpleService;

#[http]
impl SimpleService {
    pub fn hello(&self, name: String) -> String {
        format!("Hello, {}!", name)
    }
}

#[test]
fn test_no_context_works() {
    let service = SimpleService;
    let _router = service.http_router();
}

// ============================================================================
// JSON-RPC Context Tests
// ============================================================================

/// Test JSON-RPC service with Context support
#[derive(Clone)]
struct CalculatorService;

#[jsonrpc(path = "/rpc")]
impl CalculatorService {
    /// Add with request context
    fn add(&self, ctx: server_less::Context, a: i32, b: i32) -> i32 {
        // Context should be available here
        let _request_id = ctx.request_id();
        a + b
    }

    /// Subtract without context
    fn subtract(&self, a: i32, b: i32) -> i32 {
        a - b
    }

    /// Method with context and optional params
    fn multiply(&self, ctx: server_less::Context, a: i32, b: Option<i32>) -> i32 {
        let _request_id = ctx.request_id();
        a * b.unwrap_or(1)
    }
}

#[test]
fn test_jsonrpc_with_context() {
    // If this compiles, Context injection works for JSON-RPC
    let service = CalculatorService;
    let _router = service.jsonrpc_router();
}

/// Test JSON-RPC with collision detection
#[derive(Serialize, Deserialize)]
struct Context {
    interpreter_state: String,
}

#[derive(Clone)]
struct RpcInterpreter;

#[jsonrpc]
impl RpcInterpreter {
    /// Uses framework Context (qualified)
    fn eval(&self, ctx: server_less::Context, code: String) -> String {
        let _request_id = ctx.request_id();
        format!("Evaluated: {}", code)
    }

    /// Uses user's Context type (bare)
    fn execute(&self, ctx: Context, code: String) -> String {
        format!("Executed with {}: {}", ctx.interpreter_state, code)
    }
}

#[test]
fn test_jsonrpc_context_collision() {
    // If this compiles, two-pass detection works for JSON-RPC
    let service = RpcInterpreter;
    let _router = service.jsonrpc_router();
}

// ============================================================================
// WebSocket Context Tests
// ============================================================================

/// Test WebSocket service with Context support
#[derive(Clone)]
struct ChatService;

#[ws(path = "/chat")]
impl ChatService {
    /// Echo with request context from upgrade headers
    fn echo(&self, ctx: server_less::Context, message: String) -> String {
        // Context comes from WebSocket upgrade headers
        let request_id = ctx.request_id().unwrap_or("unknown");
        format!("[{}] Echo: {}", request_id, message)
    }

    /// Broadcast without context
    fn broadcast(&self, message: String) -> String {
        format!("Broadcast: {}", message)
    }

    /// Async method with context
    async fn async_echo(&self, ctx: server_less::Context, message: String) -> String {
        let _request_id = ctx.request_id();
        format!("Async: {}", message)
    }
}

#[test]
fn test_ws_with_context() {
    // If this compiles, Context injection works for WebSocket
    let service = ChatService;
    let _router = service.ws_router();
}

/// Test WebSocket with collision detection
#[derive(Clone)]
struct WsInterpreter;

#[ws]
impl WsInterpreter {
    /// Uses framework Context (qualified)
    fn eval(&self, ctx: server_less::Context, code: String) -> String {
        let _request_id = ctx.request_id();
        format!("Evaluated: {}", code)
    }

    /// Uses user's Context type (bare)
    fn execute(&self, ctx: Context, code: String) -> String {
        format!("Executed with {}: {}", ctx.interpreter_state, code)
    }
}

#[test]
fn test_ws_context_collision() {
    // If this compiles, two-pass detection works for WebSocket
    let service = WsInterpreter;
    let _router = service.ws_router();
}
