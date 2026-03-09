//! Error handling and protocol-specific error mapping.

use std::fmt;

/// Protocol-agnostic error code that maps to HTTP status, gRPC code, CLI exit code, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// 400 Bad Request / INVALID_ARGUMENT / exit 1
    InvalidInput,
    /// 401 Unauthorized / UNAUTHENTICATED / exit 1
    Unauthenticated,
    /// 403 Forbidden / PERMISSION_DENIED / exit 1
    Forbidden,
    /// 404 Not Found / NOT_FOUND / exit 1
    NotFound,
    /// 409 Conflict / ALREADY_EXISTS / exit 1
    Conflict,
    /// 422 Unprocessable Entity / FAILED_PRECONDITION / exit 1
    FailedPrecondition,
    /// 429 Too Many Requests / RESOURCE_EXHAUSTED / exit 1
    RateLimited,
    /// 500 Internal Server Error / INTERNAL / exit 1
    Internal,
    /// 501 Not Implemented / UNIMPLEMENTED / exit 1
    NotImplemented,
    /// 503 Service Unavailable / UNAVAILABLE / exit 1
    Unavailable,
}

impl ErrorCode {
    /// Convert to HTTP status code
    pub fn http_status(&self) -> u16 {
        match self {
            ErrorCode::InvalidInput => 400,
            ErrorCode::Unauthenticated => 401,
            ErrorCode::Forbidden => 403,
            ErrorCode::NotFound => 404,
            ErrorCode::Conflict => 409,
            ErrorCode::FailedPrecondition => 422,
            ErrorCode::RateLimited => 429,
            ErrorCode::Internal => 500,
            ErrorCode::NotImplemented => 501,
            ErrorCode::Unavailable => 503,
        }
    }

    /// Convert to CLI exit code
    pub fn exit_code(&self) -> i32 {
        match self {
            ErrorCode::NotFound => 1,
            ErrorCode::InvalidInput => 2,
            ErrorCode::Unauthenticated | ErrorCode::Forbidden => 3,
            ErrorCode::Conflict | ErrorCode::FailedPrecondition => 4,
            ErrorCode::RateLimited => 5,
            ErrorCode::Internal | ErrorCode::Unavailable => 1,
            ErrorCode::NotImplemented => 1,
        }
    }

    /// Convert to gRPC status code name
    pub fn grpc_code(&self) -> &'static str {
        match self {
            ErrorCode::InvalidInput => "INVALID_ARGUMENT",
            ErrorCode::Unauthenticated => "UNAUTHENTICATED",
            ErrorCode::Forbidden => "PERMISSION_DENIED",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::Conflict => "ALREADY_EXISTS",
            ErrorCode::FailedPrecondition => "FAILED_PRECONDITION",
            ErrorCode::RateLimited => "RESOURCE_EXHAUSTED",
            ErrorCode::Internal => "INTERNAL",
            ErrorCode::NotImplemented => "UNIMPLEMENTED",
            ErrorCode::Unavailable => "UNAVAILABLE",
        }
    }

    /// Convert to JSON-RPC error code.
    ///
    /// Standard codes: -32700 parse error, -32600 invalid request,
    /// -32601 method not found, -32602 invalid params, -32603 internal error.
    /// Server-defined codes are in the range -32000 to -32099.
    pub fn jsonrpc_code(&self) -> i32 {
        match self {
            ErrorCode::InvalidInput => -32602,
            ErrorCode::Unauthenticated => -32000,
            ErrorCode::Forbidden => -32001,
            ErrorCode::NotFound => -32002,
            ErrorCode::Conflict => -32003,
            ErrorCode::FailedPrecondition => -32004,
            ErrorCode::RateLimited => -32005,
            ErrorCode::Internal => -32603,
            ErrorCode::NotImplemented => -32601,
            ErrorCode::Unavailable => -32006,
        }
    }

    /// Infer error code from type/variant name (convention-based)
    pub fn infer_from_name(name: &str) -> Self {
        let name_lower = name.to_lowercase();

        if name_lower.contains("notfound")
            || name_lower.contains("not_found")
            || name_lower.contains("missing")
        {
            ErrorCode::NotFound
        } else if name_lower.contains("invalid")
            || name_lower.contains("validation")
            || name_lower.contains("parse")
        {
            ErrorCode::InvalidInput
        } else if name_lower.contains("unauthorized") || name_lower.contains("unauthenticated") {
            ErrorCode::Unauthenticated
        } else if name_lower.contains("forbidden")
            || name_lower.contains("permission")
            || name_lower.contains("denied")
        {
            ErrorCode::Forbidden
        } else if name_lower.contains("conflict")
            || name_lower.contains("exists")
            || name_lower.contains("duplicate")
        {
            ErrorCode::Conflict
        } else if name_lower.contains("ratelimit")
            || name_lower.contains("rate_limit")
            || name_lower.contains("throttle")
        {
            ErrorCode::RateLimited
        } else if name_lower.contains("unavailable") || name_lower.contains("temporarily") {
            ErrorCode::Unavailable
        } else if name_lower.contains("unimplemented") || name_lower.contains("not_implemented") {
            ErrorCode::NotImplemented
        } else {
            ErrorCode::Internal
        }
    }
}

/// Trait for converting errors to protocol-agnostic error codes.
///
/// Implement this for your error types, or use the derive macro.
pub trait IntoErrorCode {
    /// Get the error code for this error
    fn error_code(&self) -> ErrorCode;

    /// Get a human-readable message
    fn message(&self) -> String;

    /// Get the JSON-RPC numeric error code for this error.
    ///
    /// Defaults to the code derived from `error_code()`. Override this for
    /// per-variant JSON-RPC codes (e.g. `-32602` for invalid params).
    fn jsonrpc_code(&self) -> i32 {
        self.error_code().jsonrpc_code()
    }
}

/// Fallback trait used by [`HttpStatusHelper`] when the concrete error type
/// does not implement [`IntoErrorCode`].
///
/// This is part of the autoref specialization pattern. Generated HTTP handler
/// code brings this trait into scope with `use ... as _` so that
/// `HttpStatusHelper(&err).http_status_code()` resolves to 500 when the error
/// type does not implement `IntoErrorCode`, without requiring specialization.
///
/// **Not intended for direct use.** Call `HttpStatusHelper(&err).http_status_code()`
/// from generated code instead.
pub trait HttpStatusFallback {
    /// Returns the HTTP status code for this error, defaulting to 500.
    fn http_status_code(&self) -> u16;
}

/// Helper wrapper used by generated HTTP handler code to map error values to
/// HTTP status codes.
///
/// Method resolution picks the inherent impl (using [`IntoErrorCode`]) when the
/// wrapped type implements [`IntoErrorCode`], and falls back to the
/// [`HttpStatusFallback`] trait impl (which returns 500) otherwise.
///
/// # Example (generated code pattern)
///
/// ```ignore
/// use ::server_less::HttpStatusFallback as _;
/// let status_u16 = ::server_less::HttpStatusHelper(&err).http_status_code();
/// ```
pub struct HttpStatusHelper<'a, T>(pub &'a T);

impl<T: IntoErrorCode> HttpStatusHelper<'_, T> {
    /// Returns the HTTP status code derived from [`IntoErrorCode::error_code`].
    pub fn http_status_code(&self) -> u16 {
        self.0.error_code().http_status()
    }
}

impl<T> HttpStatusFallback for HttpStatusHelper<'_, T> {
    /// Fallback: returns 500 Internal Server Error for types that do not
    /// implement [`IntoErrorCode`].
    fn http_status_code(&self) -> u16 {
        500
    }
}

// Implement for common error types
impl IntoErrorCode for std::io::Error {
    fn error_code(&self) -> ErrorCode {
        match self.kind() {
            std::io::ErrorKind::NotFound => ErrorCode::NotFound,
            std::io::ErrorKind::PermissionDenied => ErrorCode::Forbidden,
            std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => {
                ErrorCode::InvalidInput
            }
            _ => ErrorCode::Internal,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl IntoErrorCode for String {
    fn error_code(&self) -> ErrorCode {
        ErrorCode::Internal
    }

    fn message(&self) -> String {
        self.clone()
    }
}

impl IntoErrorCode for &str {
    fn error_code(&self) -> ErrorCode {
        ErrorCode::Internal
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl IntoErrorCode for Box<dyn std::error::Error> {
    fn error_code(&self) -> ErrorCode {
        ErrorCode::Internal
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl IntoErrorCode for Box<dyn std::error::Error + Send + Sync> {
    fn error_code(&self) -> ErrorCode {
        ErrorCode::Internal
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

/// A generic error response that can be serialized and sent over the wire.
///
/// Produced by protocol macros when a handler returns an `Err(_)` value.
/// Serializes to `{"code": "NOT_FOUND", "message": "..."}` (details omitted when absent).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    /// Machine-readable error code (e.g. `"NOT_FOUND"`, `"INVALID_PARAMS"`).
    pub code: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured details about the error (omitted from serialization when absent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new `ErrorResponse` from an `ErrorCode` and a message.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: format!("{:?}", code).to_uppercase(),
            message: message.into(),
            details: None,
        }
    }

    /// Attach structured details to this error response.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ErrorResponse {}

/// Error type for schema validation failures.
///
/// Used by schema validation methods (validate_schema) in generated code.
#[derive(Debug, Clone)]
pub struct SchemaValidationError {
    /// Schema type (proto, capnp, thrift, smithy, etc.)
    pub schema_type: String,
    /// Lines present in expected schema but missing from generated
    pub missing_lines: Vec<String>,
    /// Lines present in generated schema but not in expected
    pub extra_lines: Vec<String>,
}

impl SchemaValidationError {
    /// Create a new schema validation error
    pub fn new(schema_type: impl Into<String>) -> Self {
        Self {
            schema_type: schema_type.into(),
            missing_lines: Vec::new(),
            extra_lines: Vec::new(),
        }
    }

    /// Add a line that's missing from the generated schema
    pub fn add_missing(&mut self, line: impl Into<String>) {
        self.missing_lines.push(line.into());
    }

    /// Add a line that's extra in the generated schema
    pub fn add_extra(&mut self, line: impl Into<String>) {
        self.extra_lines.push(line.into());
    }

    /// Check if there are any differences
    pub fn has_differences(&self) -> bool {
        !self.missing_lines.is_empty() || !self.extra_lines.is_empty()
    }
}

impl fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} schema validation failed:", self.schema_type)?;

        if !self.missing_lines.is_empty() {
            writeln!(f, "\nExpected methods/messages not found in generated:")?;
            for line in &self.missing_lines {
                writeln!(f, "  - {}", line)?;
            }
        }

        if !self.extra_lines.is_empty() {
            writeln!(f, "\nGenerated methods/messages not in expected:")?;
            for line in &self.extra_lines {
                writeln!(f, "  + {}", line)?;
            }
        }

        // Add helpful hints
        writeln!(f)?;
        writeln!(f, "Hints:")?;

        if !self.missing_lines.is_empty() && !self.extra_lines.is_empty() {
            writeln!(
                f,
                "  - Method signature or type may have changed. Check parameter names and types."
            )?;
        }

        if !self.missing_lines.is_empty() {
            writeln!(
                f,
                "  - Missing items may indicate removed or renamed methods in Rust code."
            )?;
        }

        if !self.extra_lines.is_empty() {
            writeln!(
                f,
                "  - Extra items may indicate new methods added. Update the schema file."
            )?;
        }

        writeln!(
            f,
            "  - Run `write_{schema}()` to regenerate the schema file.",
            schema = self.schema_type.to_lowercase()
        )?;

        Ok(())
    }
}

impl std::error::Error for SchemaValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_inference() {
        assert_eq!(ErrorCode::infer_from_name("NotFound"), ErrorCode::NotFound);
        assert_eq!(
            ErrorCode::infer_from_name("UserNotFound"),
            ErrorCode::NotFound
        );
        assert_eq!(
            ErrorCode::infer_from_name("InvalidEmail"),
            ErrorCode::InvalidInput
        );
        assert_eq!(
            ErrorCode::infer_from_name("Forbidden"),
            ErrorCode::Forbidden
        );
        assert_eq!(
            ErrorCode::infer_from_name("AlreadyExists"),
            ErrorCode::Conflict
        );
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(ErrorCode::NotFound.http_status(), 404);
        assert_eq!(ErrorCode::InvalidInput.http_status(), 400);
        assert_eq!(ErrorCode::Internal.http_status(), 500);
    }
}
