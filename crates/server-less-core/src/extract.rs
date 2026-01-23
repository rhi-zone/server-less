//! Context and parameter extraction types.

use std::collections::HashMap;

#[cfg(feature = "ws")]
use std::sync::Arc;
#[cfg(feature = "ws")]
use tokio::sync::Mutex;

/// Protocol-agnostic request context.
///
/// Provides unified access to protocol-specific metadata like headers, user info, and trace IDs.
///
/// # Automatic Injection
///
/// When using protocol macros (`#[http]`, `#[ws]`, etc.), methods can receive a `Context`
/// parameter that is automatically populated with request metadata:
///
/// ```ignore
/// use server_less::{http, Context};
///
/// #[http]
/// impl UserService {
///     async fn create_user(&self, ctx: Context, name: String) -> Result<User> {
///         // Access request metadata
///         let user_id = ctx.user_id()?;        // Authenticated user
///         let request_id = ctx.request_id()?;  // Request trace ID
///         let auth = ctx.authorization();      // Authorization header
///
///         // Create user with context...
///     }
/// }
/// ```
///
/// # Protocol-Specific Metadata
///
/// Different protocols populate Context with relevant data:
/// - **HTTP**: All headers via `header()`, request ID from `x-request-id`
/// - **gRPC**: Metadata fields (not yet implemented)
/// - **CLI**: Environment variables via `env()` (not yet implemented)
/// - **MCP**: Conversation context (not yet implemented)
///
/// # Name Collision
///
/// If you have your own `Context` type, qualify the server-less version:
/// ```ignore
/// fn handler(&self, ctx: server_less::Context) { }
/// ```
///
/// See the `#[http]` macro documentation for details on collision handling.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Key-value metadata (headers, env vars, etc.)
    metadata: HashMap<String, String>,
    /// The authenticated user ID, if any
    user_id: Option<String>,
    /// Request ID for tracing
    request_id: Option<String>,
}

impl Context {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create context with metadata
    pub fn with_metadata(metadata: HashMap<String, String>) -> Self {
        Self {
            metadata,
            ..Default::default()
        }
    }

    /// Get a metadata value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Set a metadata value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get the authenticated user ID
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Set the authenticated user ID
    pub fn set_user_id(&mut self, user_id: impl Into<String>) {
        self.user_id = Some(user_id.into());
    }

    /// Get the request ID
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Set the request ID
    pub fn set_request_id(&mut self, request_id: impl Into<String>) {
        self.request_id = Some(request_id.into());
    }

    /// Get all metadata
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    // HTTP-specific helpers

    /// Get an HTTP header value (case-insensitive lookup)
    pub fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.metadata
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }

    /// Get the Authorization header
    pub fn authorization(&self) -> Option<&str> {
        self.header("authorization")
    }

    /// Get the Content-Type header
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    // CLI-specific helpers

    /// Get an environment variable
    pub fn env(&self, name: &str) -> Option<&str> {
        self.get(&format!("env:{name}"))
    }
}

/// WebSocket sender for server-push messaging.
///
/// Allows WebSocket handlers to send messages to the client independently of
/// the request/response cycle, enabling true bidirectional communication.
///
/// # Automatic Injection
///
/// Methods in `#[ws]` impl blocks can receive a `WsSender` parameter that is
/// automatically injected:
///
/// ```ignore
/// use server_less::{ws, WsSender};
///
/// #[ws(path = "/chat")]
/// impl ChatService {
///     async fn join_room(&self, sender: WsSender, room: String) -> String {
///         // Store sender for later use
///         self.rooms.add_user(room, sender);
///         "Joined room".to_string()
///     }
///
///     async fn broadcast(&self, room: String, message: String) {
///         // Send to all users in room (senders stored earlier)
///         for sender in self.rooms.get_senders(&room) {
///             sender.send_json(&json!({"type": "broadcast", "msg": message})).await.ok();
///         }
///     }
/// }
/// ```
///
/// # Thread Safety
///
/// `WsSender` is cheaply cloneable (via `Arc`) and thread-safe, so you can:
/// - Store it in application state
/// - Clone and send it to background tasks
/// - Share it across threads
///
/// ```ignore
/// // Clone and use in background task
/// let sender_clone = sender.clone();
/// tokio::spawn(async move {
///     sender_clone.send("Background message").await.ok();
/// });
/// ```
#[cfg(feature = "ws")]
#[derive(Clone)]
pub struct WsSender {
    sender: Arc<
        Mutex<futures::stream::SplitSink<axum::extract::ws::WebSocket, axum::extract::ws::Message>>,
    >,
}

#[cfg(feature = "ws")]
impl WsSender {
    /// Create a new WebSocket sender (internal use by macros)
    #[doc(hidden)]
    pub fn new(
        sender: futures::stream::SplitSink<
            axum::extract::ws::WebSocket,
            axum::extract::ws::Message,
        >,
    ) -> Self {
        Self {
            sender: Arc::new(Mutex::new(sender)),
        }
    }

    /// Send a text message to the WebSocket client
    ///
    /// # Errors
    ///
    /// Returns an error if the connection is closed or the message cannot be sent.
    ///
    /// # Example
    ///
    /// ```ignore
    /// sender.send("Hello, client!").await?;
    /// ```
    pub async fn send(&self, text: impl Into<String>) -> Result<(), String> {
        use futures::sink::SinkExt;
        let mut guard = self.sender.lock().await;
        guard
            .send(axum::extract::ws::Message::Text(text.into().into()))
            .await
            .map_err(|e| format!("Failed to send WebSocket message: {}", e))
    }

    /// Send a JSON value to the WebSocket client
    ///
    /// The value is serialized to JSON and sent as a text message.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the connection is closed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use serde_json::json;
    ///
    /// sender.send_json(&json!({
    ///     "type": "notification",
    ///     "message": "New message received"
    /// })).await?;
    /// ```
    pub async fn send_json<T: serde::Serialize>(&self, value: &T) -> Result<(), String> {
        let json =
            serde_json::to_string(value).map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        self.send(json).await
    }

    /// Close the WebSocket connection
    ///
    /// Sends a close frame to the client and terminates the connection.
    ///
    /// # Example
    ///
    /// ```ignore
    /// sender.close().await?;
    /// ```
    pub async fn close(&self) -> Result<(), String> {
        use futures::sink::SinkExt;
        let mut guard = self.sender.lock().await;
        guard
            .send(axum::extract::ws::Message::Close(None))
            .await
            .map_err(|e| format!("Failed to close WebSocket: {}", e))
    }
}

/// Marker type for parameters that should be extracted from the URL path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<T>(pub T);

impl<T> Path<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Marker type for parameters that should be extracted from query string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query<T>(pub T);

impl<T> Query<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Marker type for parameters that should be extracted from request body
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_metadata() {
        let mut ctx = Context::new();
        ctx.set("Content-Type", "application/json");
        ctx.set("X-Request-Id", "abc123");

        assert_eq!(ctx.get("Content-Type"), Some("application/json"));
        assert_eq!(ctx.header("content-type"), Some("application/json"));
    }

    #[test]
    fn test_context_user() {
        let mut ctx = Context::new();
        assert!(ctx.user_id().is_none());

        ctx.set_user_id("user_123");
        assert_eq!(ctx.user_id(), Some("user_123"));
    }
}
