//! OpenAPI spec builder for composing multiple specs.

use crate::Result;
use crate::error::OpenApiError;
use crate::types::{OpenApiPath, OpenApiSchema};
use serde_json::{Map, Value};

/// Builder for composing OpenAPI specs from multiple sources.
///
/// # Example
///
/// ```ignore
/// use server_less::OpenApiBuilder;
///
/// let spec = OpenApiBuilder::new()
///     .title("My API")
///     .version("1.0.0")
///     .merge(UserService::openapi_spec())
///     .merge(OrderService::openapi_spec())
///     .build()?;
/// ```
///
/// # Conflict Resolution
///
/// - **Paths**: Last write wins (later `merge()` calls override earlier ones for same path+method).
/// - **Schemas**: Identical schemas are deduplicated; different schemas with same name cause an error.
#[derive(Debug, Clone)]
pub struct OpenApiBuilder {
    title: Option<String>,
    version: Option<String>,
    description: Option<String>,
    paths: Map<String, Value>,
    schemas: Map<String, Value>,
}

impl Default for OpenApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenApiBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            title: None,
            version: None,
            description: None,
            paths: Map::new(),
            schemas: Map::new(),
        }
    }

    /// Set the API title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the API version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the API description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Merge an OpenAPI spec (as JSON value).
    ///
    /// This extracts paths and schemas from the spec and merges them.
    ///
    /// # Conflict Resolution
    ///
    /// - Paths: Last write wins
    /// - Schemas: Identical schemas dedupe, different schemas error
    pub fn merge(mut self, spec: Value) -> Result<Self> {
        // Extract and merge paths
        if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
            for (path, methods) in paths {
                if let Some(methods_obj) = methods.as_object() {
                    let path_entry = self
                        .paths
                        .entry(path.clone())
                        .or_insert_with(|| Value::Object(Map::new()));

                    if let Some(path_obj) = path_entry.as_object_mut() {
                        for (method, operation) in methods_obj {
                            // Last write wins for paths
                            path_obj.insert(method.clone(), operation.clone());
                        }
                    }
                }
            }
        }

        // Extract and merge schemas with conflict detection
        if let Some(components) = spec.get("components").and_then(|c| c.as_object())
            && let Some(schemas) = components.get("schemas").and_then(|s| s.as_object())
        {
            for (name, schema) in schemas {
                self.merge_schema(name.clone(), schema.clone())?;
            }
        }

        // Also check for schemas at top level (some generators put them there)
        if let Some(schemas) = spec.get("schemas").and_then(|s| s.as_object()) {
            for (name, schema) in schemas {
                self.merge_schema(name.clone(), schema.clone())?;
            }
        }

        Ok(self)
    }

    /// Merge typed paths.
    pub fn merge_paths(mut self, paths: Vec<OpenApiPath>) -> Self {
        for path_def in paths {
            let path_entry = self
                .paths
                .entry(path_def.path.clone())
                .or_insert_with(|| Value::Object(Map::new()));

            if let Some(path_obj) = path_entry.as_object_mut() {
                // Convert operation to JSON
                let operation = serde_json::to_value(&path_def.operation)
                    .unwrap_or_else(|_| Value::Object(Map::new()));
                path_obj.insert(path_def.method.to_lowercase(), operation);
            }
        }
        self
    }

    /// Merge typed schemas.
    pub fn merge_schemas(mut self, schemas: Vec<OpenApiSchema>) -> Result<Self> {
        for schema_def in schemas {
            self.merge_schema(schema_def.name, schema_def.schema)?;
        }
        Ok(self)
    }

    /// Merge a single schema with conflict detection.
    fn merge_schema(&mut self, name: String, schema: Value) -> Result<()> {
        if let Some(existing) = self.schemas.get(&name) {
            // Check if schemas are identical
            if existing != &schema {
                return Err(OpenApiError::SchemaConflict { name });
            }
            // Identical - already present, nothing to do
        } else {
            self.schemas.insert(name, schema);
        }
        Ok(())
    }

    /// Build the final OpenAPI spec.
    pub fn build(self) -> Value {
        let mut spec = Map::new();

        // OpenAPI version
        spec.insert("openapi".to_string(), Value::String("3.0.0".to_string()));

        // Info object
        let mut info = Map::new();
        info.insert(
            "title".to_string(),
            Value::String(self.title.unwrap_or_else(|| "API".to_string())),
        );
        info.insert(
            "version".to_string(),
            Value::String(self.version.unwrap_or_else(|| "0.1.0".to_string())),
        );
        if let Some(desc) = self.description {
            info.insert("description".to_string(), Value::String(desc));
        }
        spec.insert("info".to_string(), Value::Object(info));

        // Paths
        if !self.paths.is_empty() {
            spec.insert("paths".to_string(), Value::Object(self.paths));
        }

        // Components/schemas
        if !self.schemas.is_empty() {
            let mut components = Map::new();
            components.insert("schemas".to_string(), Value::Object(self.schemas));
            spec.insert("components".to_string(), Value::Object(components));
        }

        Value::Object(spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_builder() {
        let spec = OpenApiBuilder::new()
            .title("Test API")
            .version("1.0.0")
            .description("A test API")
            .build();

        assert_eq!(spec["info"]["title"], "Test API");
        assert_eq!(spec["info"]["version"], "1.0.0");
        assert_eq!(spec["info"]["description"], "A test API");
        assert_eq!(spec["openapi"], "3.0.0");
    }

    #[test]
    fn test_merge_paths() {
        let spec1 = json!({
            "paths": {
                "/users": {
                    "get": {"summary": "List users"}
                }
            }
        });

        let spec2 = json!({
            "paths": {
                "/orders": {
                    "get": {"summary": "List orders"}
                }
            }
        });

        let combined = OpenApiBuilder::new()
            .merge(spec1)
            .unwrap()
            .merge(spec2)
            .unwrap()
            .build();

        assert!(combined["paths"]["/users"]["get"].is_object());
        assert!(combined["paths"]["/orders"]["get"].is_object());
    }

    #[test]
    fn test_path_override() {
        let spec1 = json!({
            "paths": {
                "/users": {
                    "get": {"summary": "First"}
                }
            }
        });

        let spec2 = json!({
            "paths": {
                "/users": {
                    "get": {"summary": "Second"}
                }
            }
        });

        let combined = OpenApiBuilder::new()
            .merge(spec1)
            .unwrap()
            .merge(spec2)
            .unwrap()
            .build();

        // Last write wins
        assert_eq!(combined["paths"]["/users"]["get"]["summary"], "Second");
    }

    #[test]
    fn test_schema_deduplication() {
        let spec1 = json!({
            "components": {
                "schemas": {
                    "User": {"type": "object", "properties": {"name": {"type": "string"}}}
                }
            }
        });

        let spec2 = json!({
            "components": {
                "schemas": {
                    "User": {"type": "object", "properties": {"name": {"type": "string"}}}
                }
            }
        });

        // Identical schemas should dedupe without error
        let result = OpenApiBuilder::new().merge(spec1).unwrap().merge(spec2);
        assert!(result.is_ok());

        let combined = result.unwrap().build();
        assert!(combined["components"]["schemas"]["User"].is_object());
    }

    #[test]
    fn test_schema_conflict() {
        let spec1 = json!({
            "components": {
                "schemas": {
                    "User": {"type": "object", "properties": {"name": {"type": "string"}}}
                }
            }
        });

        let spec2 = json!({
            "components": {
                "schemas": {
                    "User": {"type": "object", "properties": {"id": {"type": "integer"}}}
                }
            }
        });

        // Different schemas should error
        let result = OpenApiBuilder::new().merge(spec1).unwrap().merge(spec2);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, OpenApiError::SchemaConflict { name } if name == "User"));
    }

    #[test]
    fn test_merge_typed_paths() {
        use crate::types::{OpenApiOperation, OpenApiPath};

        let paths = vec![
            OpenApiPath::new("/users", "get").with_operation(OpenApiOperation::new("List users")),
            OpenApiPath::new("/users", "post").with_operation(OpenApiOperation::new("Create user")),
        ];

        let spec = OpenApiBuilder::new()
            .title("Test")
            .merge_paths(paths)
            .build();

        assert_eq!(spec["paths"]["/users"]["get"]["summary"], "List users");
        assert_eq!(spec["paths"]["/users"]["post"]["summary"], "Create user");
    }

    #[test]
    fn test_merge_typed_schemas() {
        use crate::types::OpenApiSchema;

        let schemas = vec![OpenApiSchema::new(
            "User",
            json!({"type": "object", "properties": {"name": {"type": "string"}}}),
        )];

        let spec = OpenApiBuilder::new()
            .title("Test")
            .merge_schemas(schemas)
            .unwrap()
            .build();

        assert!(spec["components"]["schemas"]["User"].is_object());
    }
}
