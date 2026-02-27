# Error Mapping

How `#[derive(ServerlessError)]` maps error variants to protocol error codes.

## The Problem

Every error type needs to map to HTTP status codes, gRPC codes, CLI exit codes, and so on. Doing this by hand is boilerplate:

```rust
impl IntoErrorCode for UserError {
    fn error_code(&self) -> ErrorCode {
        match self {
            UserError::NotFound => ErrorCode::NotFound,
            UserError::InvalidEmail => ErrorCode::InvalidInput,
            UserError::Forbidden => ErrorCode::Forbidden,
            UserError::AlreadyExists => ErrorCode::Conflict,
        }
    }

    fn message(&self) -> String {
        match self {
            UserError::NotFound => "Not found".to_string(),
            // ...
        }
    }
}
```

Four variants. Already tedious. And every protocol derive needs this to be implemented before it can handle your `Result<T, E>`.

## Convention-Based Inference

The simple case:

```rust
#[derive(Debug, ServerlessError)]
enum UserError {
    NotFound,
    InvalidEmail,
    Forbidden,
    AlreadyExists,
}
```

No attributes needed. The macro infers the error code from each variant name. `NotFound` → `ErrorCode::NotFound` → HTTP 404. `InvalidEmail` → `ErrorCode::InvalidInput` → HTTP 400. The name tells you what you need to know.

This is the same philosophy as HTTP verb inference for method names. Convention handles the common case; attributes handle the rest.

## Inference Rules

`ErrorCode::infer_from_name` checks the lowercased variant name for substrings, in priority order:

| Substring match | ErrorCode | HTTP | gRPC |
|----------------|-----------|------|------|
| `notfound`, `not_found`, `missing` | `NotFound` | 404 | NOT_FOUND |
| `invalid`, `validation`, `parse` | `InvalidInput` | 400 | INVALID_ARGUMENT |
| `unauthorized`, `unauthenticated` | `Unauthenticated` | 401 | UNAUTHENTICATED |
| `forbidden`, `permission`, `denied` | `Forbidden` | 403 | PERMISSION_DENIED |
| `conflict`, `exists`, `duplicate` | `Conflict` | 409 | ALREADY_EXISTS |
| `ratelimit`, `rate_limit`, `throttle` | `RateLimited` | 429 | RESOURCE_EXHAUSTED |
| `unavailable`, `temporarily` | `Unavailable` | 503 | UNAVAILABLE |
| `unimplemented`, `not_implemented` | `NotImplemented` | 501 | UNIMPLEMENTED |
| _(no match)_ | `Internal` | 500 | INTERNAL |

The rules are intentionally broad. `UserNotFound`, `FileNotFound`, `ResourceMissing` all infer `NotFound` correctly. `AlreadyExists`, `DuplicateEmail`, `Conflict` all infer `Conflict`.

## Explicit Overrides

When the variant name doesn't follow convention, or when you want to be explicit:

```rust
#[derive(Debug, ServerlessError)]
enum StoreError {
    #[error(code = NotFound)]
    Missing,             // name alone wouldn't infer NotFound

    #[error(code = Forbidden)]
    AccessDenied,        // "Denied" would infer Forbidden, but explicit is fine too

    #[error(code = 409)] // HTTP status works too
    WriteConflict,
}
```

Named codes (`NotFound`, `Forbidden`, etc.) are the `ErrorCode` variants. Numeric codes are HTTP status codes - the macro maps them to `ErrorCode`:

| HTTP | ErrorCode |
|------|-----------|
| 400 | `InvalidInput` |
| 401 | `Unauthenticated` |
| 403 | `Forbidden` |
| 404 | `NotFound` |
| 409 | `Conflict` |
| 422 | `FailedPrecondition` |
| 429 | `RateLimited` |
| 500 | `Internal` |
| 501 | `NotImplemented` |
| 503 | `Unavailable` |

Unknown status codes fall back to `Internal`.

## Error Messages

The macro generates a `message()` method and a `Display` impl. By default, the variant name is converted from CamelCase to sentence case: `NotFound` → `"Not found"`, `InvalidEmail` → `"Invalid email"`.

Override with `message`:

```rust
#[derive(Debug, ServerlessError)]
enum UserError {
    #[error(code = NotFound, message = "The requested user does not exist")]
    UserNotFound,
}
```

For tuple variants with a single `String` field, `Display` includes the field value:

```rust
#[derive(Debug, ServerlessError)]
enum UserError {
    #[error(code = InvalidInput)]
    ValidationFailed(String),
}

// Display: "ValidationFailed: email is not valid"
```

For struct variants and multi-field tuple variants, `Display` falls back to `message()`.

## What Gets Generated

```rust
#[derive(Debug, ServerlessError)]
enum UserError {
    NotFound,
    InvalidEmail,
}

// Generates:
impl IntoErrorCode for UserError {
    fn error_code(&self) -> ErrorCode {
        match self {
            Self::NotFound => ErrorCode::infer_from_name("NotFound"),
            Self::InvalidEmail => ErrorCode::infer_from_name("InvalidEmail"),
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NotFound => "Not found".to_string(),
            Self::InvalidEmail => "Invalid email".to_string(),
        }
    }
}

impl Display for UserError { ... }
impl std::error::Error for UserError {}
```

Run `cargo expand` to see the exact output for your type.

## The Alternative

The alternative is a required trait with no inference - every variant must be annotated:

```rust
// What we didn't do
#[derive(Debug, ServerlessError)]
enum UserError {
    #[error(code = NotFound)]
    NotFound,            // required even though it's obvious
    #[error(code = InvalidInput)]
    InvalidEmail,        // required even though it's obvious
    #[error(code = Forbidden)]
    Forbidden,           // required even though it's obvious
}
```

This is the explicit-only approach. It's consistent and never surprising. It's also tedious for the common case where the variant name is self-documenting. Convention handles that case without ceremony.

The cost: inference can be wrong. `RateLimit` infers correctly; `Throttle` also infers `RateLimited` - that's fine. `InternalServerError` contains neither a known keyword nor a miss, so it falls back to `Internal` - also fine. The cases where inference gets it wrong are cases where the variant name is unusual, and unusual names are exactly when you'd reach for `#[error(code = ...)]`.

## Protocol Dispatch

`ErrorCode` is protocol-agnostic. Each protocol derive converts it appropriately:

| Protocol | How ErrorCode is used |
|----------|----------------------|
| HTTP | `error_code.http_status()` → status code + JSON body |
| gRPC | `error_code.grpc_code()` → gRPC status string |
| CLI | `error_code.exit_code()` → process exit code + stderr |

The error type is defined once. All protocols get the right behavior.
