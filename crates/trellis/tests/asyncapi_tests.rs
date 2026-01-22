//! Integration tests for the AsyncAPI specification generation macro.

#![allow(dead_code)]
#![allow(unused_variables)]

use rhizome_trellis::asyncapi;

#[derive(Clone)]
struct ChatService;

#[asyncapi(
    title = "Chat API",
    version = "2.0.0",
    server = "ws://chat.example.com"
)]
impl ChatService {
    /// Send a message to a chat room
    pub fn send_message(&self, room: String, content: String) -> bool {
        true
    }

    /// Get message history for a room
    pub fn get_history(&self, room: String, limit: Option<u32>) -> Vec<String> {
        vec![]
    }

    /// Join a chat room
    pub fn join_room(&self, room: String, user: String) -> bool {
        true
    }
}

#[test]
fn test_asyncapi_spec_structure() {
    let spec = ChatService::asyncapi_spec();

    assert_eq!(spec["asyncapi"], "2.6.0");
    assert_eq!(spec["info"]["title"], "Chat API");
    assert_eq!(spec["info"]["version"], "2.0.0");
}

#[test]
fn test_asyncapi_server() {
    let spec = ChatService::asyncapi_spec();

    assert_eq!(spec["servers"]["default"]["url"], "ws://chat.example.com");
    assert_eq!(spec["servers"]["default"]["protocol"], "ws");
}

#[test]
fn test_asyncapi_channels() {
    let spec = ChatService::asyncapi_spec();

    assert!(spec["channels"]["sendMessage"].is_object());
    assert!(spec["channels"]["getHistory"].is_object());
    assert!(spec["channels"]["joinRoom"].is_object());
}

#[test]
fn test_asyncapi_channel_operations() {
    let spec = ChatService::asyncapi_spec();

    // Each channel should have publish and subscribe
    let send_msg = &spec["channels"]["sendMessage"];
    assert!(send_msg["publish"].is_object());
    assert!(send_msg["subscribe"].is_object());
}

#[test]
fn test_asyncapi_messages() {
    let spec = ChatService::asyncapi_spec();

    // Should have request/response messages
    assert!(spec["components"]["messages"]["SendMessageRequest"].is_object());
    assert!(spec["components"]["messages"]["SendMessageResponse"].is_object());
}

#[test]
fn test_asyncapi_message_payload() {
    let spec = ChatService::asyncapi_spec();

    let request = &spec["components"]["messages"]["SendMessageRequest"];
    assert!(request["payload"]["properties"]["room"].is_object());
    assert!(request["payload"]["properties"]["content"].is_object());
}

#[test]
fn test_asyncapi_json_output() {
    let json = ChatService::asyncapi_json();

    assert!(json.contains("asyncapi"));
    assert!(json.contains("Chat API"));
    assert!(json.contains("channels"));
}

// Test default values
#[derive(Clone)]
struct SimpleService;

#[asyncapi]
impl SimpleService {
    pub fn ping(&self) -> String {
        "pong".to_string()
    }
}

#[test]
fn test_asyncapi_defaults() {
    let spec = SimpleService::asyncapi_spec();

    // Default title should be struct name
    assert_eq!(spec["info"]["title"], "SimpleService");
    // Default version
    assert_eq!(spec["info"]["version"], "1.0.0");
    // Default server
    assert_eq!(spec["servers"]["default"]["url"], "ws://localhost:8080");
}

// Combined with ws
#[derive(Clone)]
struct CombinedService;

#[rhizome_trellis::ws(path = "/ws")]
#[asyncapi(title = "Combined WebSocket API")]
impl CombinedService {
    pub fn echo(&self, message: String) -> String {
        message
    }
}

#[test]
fn test_asyncapi_with_ws() {
    // Both macros work together
    let spec = CombinedService::asyncapi_spec();
    assert_eq!(spec["info"]["title"], "Combined WebSocket API");

    let methods = CombinedService::ws_methods();
    assert!(methods.contains(&"echo"));
}
