# Route & Response Attributes

Method-level attributes for overriding HTTP routing and response behavior.

## `#[route(...)]`

Overrides the inferred HTTP method, path, and OpenAPI visibility for a single method.

### Attributes

| Attribute | Type | Effect |
|-----------|------|--------|
| `method = "POST"` | String | Override HTTP method (case-insensitive, normalized to uppercase) |
| `path = "/custom/{id}"` | String | Override the inferred path |
| `skip` | Flag | Exclude from both the HTTP router and OpenAPI spec |
| `hidden` | Flag | Exclude from OpenAPI spec but keep in the router |
| `tags = "users,admin"` | String | Comma-separated OpenAPI operation tags |
| `deprecated` | Flag | Mark the OpenAPI operation as deprecated |

### Examples

```rust
#[http]
impl MyService {
    // Override just the path
    #[route(path = "/custom-endpoint")]
    pub fn my_method(&self) -> String { ... }

    // Override the HTTP method (inference would pick GET for "get_*")
    #[route(method = "POST")]
    pub fn get_data(&self, payload: String) -> String { ... }

    // Override both method and path
    #[route(method = "PUT", path = "/special/{id}")]
    pub fn do_something(&self, id: String) -> String { ... }

    // Skip entirely — not in router, not in OpenAPI
    #[route(skip)]
    pub fn internal_helper(&self) -> String { ... }

    // Keep in router but hide from OpenAPI docs
    #[route(hidden)]
    pub fn secret_endpoint(&self) -> String { ... }

    // Add OpenAPI tags and deprecation notice
    #[route(tags = "admin,legacy", deprecated)]
    pub fn old_endpoint(&self) -> String { ... }
}
```

### Inference without `#[route]`

When `#[route]` is absent, the HTTP method and path are inferred from the method name. See [Impl-First Design](impl-first.md) for the full naming convention table. `#[route]` only overrides specific fields — unspecified fields still use inference.

## `#[response(...)]`

Overrides the HTTP response behavior for a single method.

### Attributes

| Attribute | Type | Effect |
|-----------|------|--------|
| `status = 201` | Integer | Override the default HTTP status code |
| `content_type = "..."` | String | Override the response content type |
| `header = "..." , value = "..."` | String pair | Add a custom response header |
| `description = "..."` | String | Custom OpenAPI response description |

### Examples

```rust
#[http]
impl MyService {
    // Return 201 Created instead of default 200
    #[response(status = 201)]
    pub fn create_resource(&self, name: String) -> String { ... }

    // Custom content type for non-JSON responses
    #[response(content_type = "text/plain")]
    pub fn get_readme(&self) -> String { ... }

    // Add a custom response header
    #[response(header = "x-request-id", value = "generated")]
    pub fn get_resource(&self, id: String) -> String { ... }

    // Combine multiple overrides
    #[response(status = 201, description = "User created successfully")]
    pub fn create_user(&self, name: String, email: String) -> User { ... }
}
```

### Defaults without `#[response]`

| Return type | Default status |
|-------------|---------------|
| `T` | 200 OK |
| `()` | 204 No Content |
| `Option<T>` | 200 OK or 404 Not Found |
| `Result<T, E>` | 200 OK or error status from E |

Content type defaults to `application/json` for all serializable types.

## Composing `#[route]` and `#[response]`

Both attributes can be used together on the same method:

```rust
#[route(method = "POST", path = "/api/upload")]
#[response(status = 201, header = "x-upload-id", value = "generated")]
pub fn upload(&self, data: Vec<u8>) -> UploadResult { ... }
```

## See Also

- [Impl-First Design](impl-first.md) — naming conventions and inference rules
- [Param Attributes](param-attributes.md) — `#[param]` for parameter-level overrides
- [Inference vs Configuration](inference-vs-configuration.md) — full override reference
