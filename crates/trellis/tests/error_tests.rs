//! Tests for the TrellisError derive macro.

use trellis::{ErrorCode, IntoErrorCode, TrellisError};

// Test basic derive with code inference
#[derive(Debug, TrellisError)]
enum BasicError {
    NotFound,
    InvalidInput,
    Unauthorized,
    InternalError,
}

#[test]
fn test_basic_error_code_inference() {
    assert_eq!(BasicError::NotFound.error_code(), ErrorCode::NotFound);
    assert_eq!(BasicError::InvalidInput.error_code(), ErrorCode::InvalidInput);
    assert_eq!(BasicError::Unauthorized.error_code(), ErrorCode::Unauthenticated);
    assert_eq!(BasicError::InternalError.error_code(), ErrorCode::Internal);
}

#[test]
fn test_basic_error_message() {
    assert_eq!(BasicError::NotFound.message(), "Not found");
    assert_eq!(BasicError::InvalidInput.message(), "Invalid input");
}

#[test]
fn test_basic_error_display() {
    assert_eq!(format!("{}", BasicError::NotFound), "Not found");
}

// Test explicit code attribute
#[derive(Debug, TrellisError)]
enum ExplicitCodeError {
    #[error(code = NotFound)]
    Missing,
    #[error(code = Forbidden)]
    AccessDenied,
    #[error(code = 429)] // HTTP status
    TooManyRequests,
}

#[test]
fn test_explicit_error_codes() {
    assert_eq!(ExplicitCodeError::Missing.error_code(), ErrorCode::NotFound);
    assert_eq!(ExplicitCodeError::AccessDenied.error_code(), ErrorCode::Forbidden);
    assert_eq!(ExplicitCodeError::TooManyRequests.error_code(), ErrorCode::RateLimited);
}

// Test custom messages
#[derive(Debug, TrellisError)]
enum CustomMessageError {
    #[error(code = NotFound, message = "The requested resource was not found")]
    ResourceNotFound,
    #[error(message = "Access denied to this resource")]
    Forbidden,
}

#[test]
fn test_custom_messages() {
    assert_eq!(
        CustomMessageError::ResourceNotFound.message(),
        "The requested resource was not found"
    );
    assert_eq!(
        CustomMessageError::Forbidden.message(),
        "Access denied to this resource"
    );
}

// Test with tuple variants
#[derive(Debug, TrellisError)]
enum TupleVariantError {
    #[error(code = InvalidInput)]
    ValidationFailed(String),
    #[error(code = NotFound)]
    ItemNotFound(u64),
}

#[test]
fn test_tuple_variants() {
    let err = TupleVariantError::ValidationFailed("bad email".to_string());
    assert_eq!(err.error_code(), ErrorCode::InvalidInput);

    let err = TupleVariantError::ItemNotFound(42);
    assert_eq!(err.error_code(), ErrorCode::NotFound);
}

#[test]
fn test_tuple_variant_display() {
    let err = TupleVariantError::ValidationFailed("bad email".to_string());
    assert_eq!(format!("{}", err), "ValidationFailed: bad email");
}

// Test with struct variants
#[derive(Debug, TrellisError)]
enum StructVariantError {
    #[error(code = InvalidInput, message = "Validation failed")]
    ValidationError { field: String, reason: String },
}

#[test]
fn test_struct_variants() {
    let err = StructVariantError::ValidationError {
        field: "email".to_string(),
        reason: "invalid format".to_string(),
    };
    assert_eq!(err.error_code(), ErrorCode::InvalidInput);
    assert_eq!(err.message(), "Validation failed");
}

// Test that std::error::Error is implemented
#[test]
fn test_std_error_impl() {
    let err: Box<dyn std::error::Error> = Box::new(BasicError::NotFound);
    assert_eq!(err.to_string(), "Not found");
}

// Test HTTP status code mapping
#[derive(Debug, TrellisError)]
enum HttpStatusError {
    #[error(code = 400)]
    BadRequest,
    #[error(code = 401)]
    Unauthorized,
    #[error(code = 403)]
    Forbidden,
    #[error(code = 404)]
    NotFound,
    #[error(code = 409)]
    Conflict,
    #[error(code = 500)]
    Internal,
    #[error(code = 503)]
    Unavailable,
}

#[test]
fn test_http_status_mapping() {
    assert_eq!(HttpStatusError::BadRequest.error_code(), ErrorCode::InvalidInput);
    assert_eq!(HttpStatusError::Unauthorized.error_code(), ErrorCode::Unauthenticated);
    assert_eq!(HttpStatusError::Forbidden.error_code(), ErrorCode::Forbidden);
    assert_eq!(HttpStatusError::NotFound.error_code(), ErrorCode::NotFound);
    assert_eq!(HttpStatusError::Conflict.error_code(), ErrorCode::Conflict);
    assert_eq!(HttpStatusError::Internal.error_code(), ErrorCode::Internal);
    assert_eq!(HttpStatusError::Unavailable.error_code(), ErrorCode::Unavailable);
}
