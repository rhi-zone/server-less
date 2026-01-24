//! Typed OpenAPI structures.
//!
//! These types represent a subset of OpenAPI 3.0 used by server-less for spec generation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An OpenAPI path with its operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiPath {
    /// The path pattern (e.g., "/users/{id}").
    pub path: String,
    /// HTTP method (lowercase: "get", "post", etc.).
    pub method: String,
    /// The operation definition.
    pub operation: OpenApiOperation,
}

/// An OpenAPI operation (endpoint).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiOperation {
    /// Short summary of the operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Unique operation identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    /// Operation parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<OpenApiParameter>,
    /// Request body definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<Value>,
    /// Response definitions keyed by status code.
    #[serde(default)]
    pub responses: serde_json::Map<String, Value>,
    /// Additional fields not explicitly modeled.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// An OpenAPI parameter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiParameter {
    /// Parameter name.
    pub name: String,
    /// Location: "path", "query", "header", or "cookie".
    #[serde(rename = "in")]
    pub location: String,
    /// Whether the parameter is required.
    #[serde(default)]
    pub required: bool,
    /// Parameter schema.
    #[serde(default)]
    pub schema: Value,
    /// Parameter description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Additional fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// An OpenAPI schema definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiSchema {
    /// Schema name (used as key in components/schemas).
    pub name: String,
    /// The schema definition.
    pub schema: Value,
}

impl OpenApiPath {
    /// Create a new path.
    pub fn new(path: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            method: method.into().to_lowercase(),
            operation: OpenApiOperation::default(),
        }
    }

    /// Set the operation for this path.
    pub fn with_operation(mut self, operation: OpenApiOperation) -> Self {
        self.operation = operation;
        self
    }
}

impl Default for OpenApiOperation {
    fn default() -> Self {
        Self {
            summary: None,
            operation_id: None,
            parameters: Vec::new(),
            request_body: None,
            responses: serde_json::Map::new(),
            extra: serde_json::Map::new(),
        }
    }
}

impl OpenApiOperation {
    /// Create a new operation with a summary.
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: Some(summary.into()),
            ..Default::default()
        }
    }

    /// Set the operation ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.operation_id = Some(id.into());
        self
    }

    /// Add a parameter.
    pub fn with_parameter(mut self, param: OpenApiParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add a response.
    pub fn with_response(mut self, status: impl Into<String>, response: Value) -> Self {
        self.responses.insert(status.into(), response);
        self
    }
}

impl OpenApiParameter {
    /// Create a path parameter.
    pub fn path(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: "path".to_string(),
            required: true, // Path params are always required
            schema: serde_json::json!({"type": "string"}),
            description: None,
            extra: serde_json::Map::new(),
        }
    }

    /// Create a query parameter.
    pub fn query(name: impl Into<String>, required: bool) -> Self {
        Self {
            name: name.into(),
            location: "query".to_string(),
            required,
            schema: serde_json::json!({"type": "string"}),
            description: None,
            extra: serde_json::Map::new(),
        }
    }

    /// Create a header parameter.
    pub fn header(name: impl Into<String>, required: bool) -> Self {
        Self {
            name: name.into(),
            location: "header".to_string(),
            required,
            schema: serde_json::json!({"type": "string"}),
            description: None,
            extra: serde_json::Map::new(),
        }
    }

    /// Set the schema.
    pub fn with_schema(mut self, schema: Value) -> Self {
        self.schema = schema;
        self
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

impl OpenApiSchema {
    /// Create a new schema.
    pub fn new(name: impl Into<String>, schema: Value) -> Self {
        Self {
            name: name.into(),
            schema,
        }
    }
}
