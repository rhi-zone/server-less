//! Error types for OpenAPI composition.

use thiserror::Error;

/// Errors that can occur during OpenAPI composition.
///
/// `#[non_exhaustive]`: new error variants may be added in minor releases, so
/// downstream `match`es must include a wildcard arm.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OpenApiError {
    /// Schema conflict: same name, different definitions.
    #[error("Schema conflict for '{name}': defined differently in multiple specs")]
    SchemaConflict { name: String },

    /// Invalid OpenAPI spec structure.
    #[error("Invalid OpenAPI spec: {message}")]
    InvalidSpec { message: String },

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
