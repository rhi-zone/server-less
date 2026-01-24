//! OpenAPI composition utilities for server-less.
//!
//! This crate provides types and a builder for composing OpenAPI specs from multiple sources.
//!
//! # Example
//!
//! ```ignore
//! use server_less::OpenApiBuilder;
//!
//! let spec = OpenApiBuilder::new()
//!     .title("My API")
//!     .version("1.0.0")
//!     .merge(UserService::openapi_spec())
//!     .merge(OrderService::openapi_spec())
//!     .build()?;
//! ```

mod builder;
mod error;
mod types;

pub use builder::OpenApiBuilder;
pub use error::OpenApiError;
pub use types::*;

/// Result type for OpenAPI operations.
pub type Result<T> = std::result::Result<T, OpenApiError>;
