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

    /// Infer error code from type/variant name (convention-based)
    pub fn infer_from_name(name: &str) -> Self {
        let name_lower = name.to_lowercase();

        if name_lower.contains("notfound") || name_lower.contains("not_found") || name_lower.contains("missing") {
            ErrorCode::NotFound
        } else if name_lower.contains("invalid") || name_lower.contains("validation") || name_lower.contains("parse") {
            ErrorCode::InvalidInput
        } else if name_lower.contains("unauthorized") || name_lower.contains("unauthenticated") {
            ErrorCode::Unauthenticated
        } else if name_lower.contains("forbidden") || name_lower.contains("permission") || name_lower.contains("denied") {
            ErrorCode::Forbidden
        } else if name_lower.contains("conflict") || name_lower.contains("exists") || name_lower.contains("duplicate") {
            ErrorCode::Conflict
        } else if name_lower.contains("ratelimit") || name_lower.contains("rate_limit") || name_lower.contains("throttle") {
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
}

// Implement for common error types
impl IntoErrorCode for std::io::Error {
    fn error_code(&self) -> ErrorCode {
        match self.kind() {
            std::io::ErrorKind::NotFound => ErrorCode::NotFound,
            std::io::ErrorKind::PermissionDenied => ErrorCode::Forbidden,
            std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => ErrorCode::InvalidInput,
            _ => ErrorCode::Internal,
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

/// A generic error response that can be serialized
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: format!("{:?}", code).to_uppercase(),
            message: message.into(),
            details: None,
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_inference() {
        assert_eq!(ErrorCode::infer_from_name("NotFound"), ErrorCode::NotFound);
        assert_eq!(ErrorCode::infer_from_name("UserNotFound"), ErrorCode::NotFound);
        assert_eq!(ErrorCode::infer_from_name("InvalidEmail"), ErrorCode::InvalidInput);
        assert_eq!(ErrorCode::infer_from_name("Forbidden"), ErrorCode::Forbidden);
        assert_eq!(ErrorCode::infer_from_name("AlreadyExists"), ErrorCode::Conflict);
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(ErrorCode::NotFound.http_status(), 404);
        assert_eq!(ErrorCode::InvalidInput.http_status(), 400);
        assert_eq!(ErrorCode::Internal.http_status(), 500);
    }
}
