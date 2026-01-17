//! Schema validation helpers.
//!
//! This module provides utilities to validate `serde_json::Value` against a [`Schema`].
//! It helps providers validate input before processing and gives detailed error messages.
//!
//! # Example
//!
//! ```
//! use hemmer_provider_sdk::schema::{Schema, Attribute};
//! use hemmer_provider_sdk::validation::validate;
//! use serde_json::json;
//!
//! let schema = Schema::v0()
//!     .with_attribute("name", Attribute::required_string())
//!     .with_attribute("count", Attribute::optional_int64());
//!
//! // Valid input
//! let input = json!({
//!     "name": "test",
//!     "count": 42
//! });
//! let diagnostics = validate(&schema, &input);
//! assert!(diagnostics.is_empty());
//!
//! // Invalid input - wrong type for count
//! let input = json!({
//!     "name": "test",
//!     "count": "not a number"
//! });
//! let diagnostics = validate(&schema, &input);
//! assert_eq!(diagnostics.len(), 1);
//! assert_eq!(diagnostics[0].attribute, Some("count".to_string()));
//! ```

use crate::schema::{
    Attribute, AttributeType, Block, BlockNestingMode, Diagnostic, DiagnosticSeverity, NestedBlock,
    Schema,
};
use serde_json::Value;
use std::collections::HashMap;

/// Validate a JSON value against a schema.
///
/// Returns a list of diagnostics for any validation errors found.
/// An empty list means the value is valid.
///
/// # Validation Rules
///
/// - Required attributes must be present and non-null
/// - Optional attributes may be absent or null
/// - Computed attributes are skipped (provider sets these)
/// - Attribute types must match the schema
/// - Nested blocks are validated recursively with min/max item constraints
pub fn validate(schema: &Schema, value: &Value) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_block(&schema.block, value, "", &mut diagnostics);
    diagnostics
}

/// Validate a JSON value against a schema, returning Ok if valid or Err with diagnostics.
///
/// This is a convenience wrapper around [`validate`] that returns a Result.
pub fn validate_result(schema: &Schema, value: &Value) -> Result<(), Vec<Diagnostic>> {
    let diagnostics = validate(schema, value);
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

/// Check if a JSON value is valid against a schema.
///
/// Returns `true` if valid, `false` otherwise.
/// Use [`validate`] to get detailed error information.
pub fn is_valid(schema: &Schema, value: &Value) -> bool {
    validate(schema, value).is_empty()
}

fn validate_block(block: &Block, value: &Value, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    let obj = match value {
        Value::Object(map) => map,
        Value::Null => {
            // Null is valid for optional blocks, but we can't validate further
            return;
        },
        _ => {
            diagnostics.push(
                Diagnostic::error("Expected object")
                    .with_detail(format!("Got {}", value_type_name(value)))
                    .with_attribute_if_not_empty(path),
            );
            return;
        },
    };

    // Validate attributes
    for (name, attr) in &block.attributes {
        let attr_path = join_path(path, name);
        let attr_value = obj.get(name);
        validate_attribute(attr, attr_value, &attr_path, diagnostics);
    }

    // Validate nested blocks
    for (name, nested_block) in &block.blocks {
        let block_path = join_path(path, name);
        let block_value = obj.get(name);
        validate_nested_block(nested_block, block_value, &block_path, diagnostics);
    }
}

fn validate_attribute(
    attr: &Attribute,
    value: Option<&Value>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Skip computed-only attributes (provider sets these)
    if attr.flags.computed && !attr.flags.optional && !attr.flags.required {
        return;
    }

    match value {
        None | Some(Value::Null) => {
            // Check if required
            if attr.flags.required {
                diagnostics.push(
                    Diagnostic::error(format!("Missing required attribute '{}'", path))
                        .with_detail("This attribute is required and must be provided")
                        .with_attribute(path),
                );
            }
            // Optional attributes can be missing/null
        },
        Some(v) => {
            // Validate type
            validate_attribute_type(&attr.attr_type, v, path, diagnostics);
        },
    }
}

fn validate_attribute_type(
    attr_type: &AttributeType,
    value: &Value,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match attr_type {
        AttributeType::String => {
            if !value.is_string() {
                diagnostics.push(type_error(path, "string", value));
            }
        },
        AttributeType::Int64 => {
            if !is_int64(value) {
                diagnostics.push(type_error(path, "int64", value));
            }
        },
        AttributeType::Float64 => {
            if !value.is_number() {
                diagnostics.push(type_error(path, "float64", value));
            }
        },
        AttributeType::Bool => {
            if !value.is_boolean() {
                diagnostics.push(type_error(path, "bool", value));
            }
        },
        AttributeType::List(element_type) => {
            if let Some(arr) = value.as_array() {
                for (i, elem) in arr.iter().enumerate() {
                    let elem_path = format!("{}.{}", path, i);
                    validate_attribute_type(element_type, elem, &elem_path, diagnostics);
                }
            } else {
                diagnostics.push(type_error(path, "list", value));
            }
        },
        AttributeType::Set(element_type) => {
            // Sets are represented as arrays in JSON
            if let Some(arr) = value.as_array() {
                for (i, elem) in arr.iter().enumerate() {
                    let elem_path = format!("{}.{}", path, i);
                    validate_attribute_type(element_type, elem, &elem_path, diagnostics);
                }
            } else {
                diagnostics.push(type_error(path, "set", value));
            }
        },
        AttributeType::Map(value_type) => {
            if let Some(obj) = value.as_object() {
                for (key, val) in obj {
                    let key_path = format!("{}.{}", path, key);
                    validate_attribute_type(value_type, val, &key_path, diagnostics);
                }
            } else {
                diagnostics.push(type_error(path, "map", value));
            }
        },
        AttributeType::Object(attrs) => {
            if let Some(obj) = value.as_object() {
                validate_object_type(attrs, obj, path, diagnostics);
            } else {
                diagnostics.push(type_error(path, "object", value));
            }
        },
        AttributeType::Dynamic => {
            // Dynamic accepts any value
        },
    }
}

fn validate_object_type(
    attrs: &HashMap<String, AttributeType>,
    obj: &serde_json::Map<String, Value>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (name, attr_type) in attrs {
        let attr_path = join_path(path, name);
        if let Some(value) = obj.get(name) {
            validate_attribute_type(attr_type, value, &attr_path, diagnostics);
        }
        // Object attributes within a type don't have required/optional flags,
        // so we don't enforce presence
    }
}

fn validate_nested_block(
    nested: &NestedBlock,
    value: Option<&Value>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match nested.nesting_mode {
        BlockNestingMode::Single => {
            validate_single_block(nested, value, path, diagnostics);
        },
        BlockNestingMode::List => {
            validate_list_block(nested, value, path, diagnostics);
        },
        BlockNestingMode::Set => {
            // Sets are validated the same as lists for our purposes
            validate_list_block(nested, value, path, diagnostics);
        },
        BlockNestingMode::Map => {
            validate_map_block(nested, value, path, diagnostics);
        },
    }
}

fn validate_single_block(
    nested: &NestedBlock,
    value: Option<&Value>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match value {
        None | Some(Value::Null) => {
            if nested.min_items > 0 {
                diagnostics.push(
                    Diagnostic::error(format!("Missing required block '{}'", path))
                        .with_detail("At least one block is required")
                        .with_attribute(path),
                );
            }
        },
        Some(v) => {
            validate_block(&nested.block, v, path, diagnostics);
        },
    }
}

fn validate_list_block(
    nested: &NestedBlock,
    value: Option<&Value>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match value {
        None | Some(Value::Null) => {
            if nested.min_items > 0 {
                diagnostics.push(
                    Diagnostic::error(format!(
                        "Block '{}' requires at least {} item(s)",
                        path, nested.min_items
                    ))
                    .with_attribute(path),
                );
            }
        },
        Some(Value::Array(arr)) => {
            let len = arr.len() as u32;

            // Check min_items
            if len < nested.min_items {
                diagnostics.push(
                    Diagnostic::error(format!(
                        "Block '{}' requires at least {} item(s), got {}",
                        path, nested.min_items, len
                    ))
                    .with_attribute(path),
                );
            }

            // Check max_items (0 means unlimited)
            if nested.max_items > 0 && len > nested.max_items {
                diagnostics.push(
                    Diagnostic::error(format!(
                        "Block '{}' allows at most {} item(s), got {}",
                        path, nested.max_items, len
                    ))
                    .with_attribute(path),
                );
            }

            // Validate each block
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{}.{}", path, i);
                validate_block(&nested.block, item, &item_path, diagnostics);
            }
        },
        Some(v) => {
            diagnostics.push(
                Diagnostic::error(format!("Expected list for block '{}'", path))
                    .with_detail(format!("Got {}", value_type_name(v)))
                    .with_attribute(path),
            );
        },
    }
}

fn validate_map_block(
    nested: &NestedBlock,
    value: Option<&Value>,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match value {
        None | Some(Value::Null) => {
            if nested.min_items > 0 {
                diagnostics.push(
                    Diagnostic::error(format!(
                        "Block '{}' requires at least {} item(s)",
                        path, nested.min_items
                    ))
                    .with_attribute(path),
                );
            }
        },
        Some(Value::Object(obj)) => {
            let len = obj.len() as u32;

            // Check min_items
            if len < nested.min_items {
                diagnostics.push(
                    Diagnostic::error(format!(
                        "Block '{}' requires at least {} item(s), got {}",
                        path, nested.min_items, len
                    ))
                    .with_attribute(path),
                );
            }

            // Check max_items (0 means unlimited)
            if nested.max_items > 0 && len > nested.max_items {
                diagnostics.push(
                    Diagnostic::error(format!(
                        "Block '{}' allows at most {} item(s), got {}",
                        path, nested.max_items, len
                    ))
                    .with_attribute(path),
                );
            }

            // Validate each block
            for (key, item) in obj {
                let item_path = format!("{}.{}", path, key);
                validate_block(&nested.block, item, &item_path, diagnostics);
            }
        },
        Some(v) => {
            diagnostics.push(
                Diagnostic::error(format!("Expected map for block '{}'", path))
                    .with_detail(format!("Got {}", value_type_name(v)))
                    .with_attribute(path),
            );
        },
    }
}

// Helper functions

fn join_path(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_string()
    } else {
        format!("{}.{}", base, name)
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn is_int64(value: &Value) -> bool {
    match value {
        Value::Number(n) => {
            // Check if it's an integer (no fractional part)
            if let Some(i) = n.as_i64() {
                // It's already an i64
                let _ = i;
                true
            } else if let Some(f) = n.as_f64() {
                // Check if the float is actually an integer
                f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64
            } else {
                false
            }
        },
        _ => false,
    }
}

fn type_error(path: &str, expected: &str, got: &Value) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        summary: format!("Invalid type for attribute '{}'", path),
        detail: Some(format!(
            "Expected {}, got {}",
            expected,
            value_type_name(got)
        )),
        attribute: Some(path.to_string()),
    }
}

trait DiagnosticExt {
    fn with_attribute_if_not_empty(self, path: &str) -> Self;
}

impl DiagnosticExt for Diagnostic {
    fn with_attribute_if_not_empty(self, path: &str) -> Self {
        if path.is_empty() {
            self
        } else {
            self.with_attribute(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Attribute, AttributeFlags, Block, NestedBlock, Schema};
    use serde_json::json;

    #[test]
    fn test_validate_required_string() {
        let schema = Schema::v0().with_attribute("name", Attribute::required_string());

        // Valid
        let diagnostics = validate(&schema, &json!({"name": "test"}));
        assert!(diagnostics.is_empty());

        // Missing required
        let diagnostics = validate(&schema, &json!({}));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].attribute, Some("name".to_string()));

        // Null value
        let diagnostics = validate(&schema, &json!({"name": null}));
        assert_eq!(diagnostics.len(), 1);

        // Wrong type
        let diagnostics = validate(&schema, &json!({"name": 123}));
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].summary.contains("Invalid type"));
    }

    #[test]
    fn test_validate_optional_attribute() {
        let schema = Schema::v0().with_attribute("count", Attribute::optional_int64());

        // Valid with value
        let diagnostics = validate(&schema, &json!({"count": 42}));
        assert!(diagnostics.is_empty());

        // Valid without value
        let diagnostics = validate(&schema, &json!({}));
        assert!(diagnostics.is_empty());

        // Valid with null
        let diagnostics = validate(&schema, &json!({"count": null}));
        assert!(diagnostics.is_empty());

        // Wrong type
        let diagnostics = validate(&schema, &json!({"count": "not a number"}));
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_validate_computed_attribute_skipped() {
        let schema = Schema::v0().with_attribute("id", Attribute::computed_string());

        // Computed attributes should be skipped
        let diagnostics = validate(&schema, &json!({}));
        assert!(diagnostics.is_empty());

        // Even with wrong type, we don't validate computed-only attrs
        let diagnostics = validate(&schema, &json!({"id": 123}));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_validate_int64() {
        let schema = Schema::v0().with_attribute("count", Attribute::required_int64());

        // Integer
        let diagnostics = validate(&schema, &json!({"count": 42}));
        assert!(diagnostics.is_empty());

        // Float that's actually an integer
        let diagnostics = validate(&schema, &json!({"count": 42.0}));
        assert!(diagnostics.is_empty());

        // Float with fractional part
        let diagnostics = validate(&schema, &json!({"count": 42.5}));
        assert_eq!(diagnostics.len(), 1);

        // String
        let diagnostics = validate(&schema, &json!({"count": "42"}));
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_validate_bool() {
        let schema = Schema::v0().with_attribute("enabled", Attribute::required_bool());

        let diagnostics = validate(&schema, &json!({"enabled": true}));
        assert!(diagnostics.is_empty());

        let diagnostics = validate(&schema, &json!({"enabled": false}));
        assert!(diagnostics.is_empty());

        let diagnostics = validate(&schema, &json!({"enabled": "true"}));
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_validate_list() {
        let schema = Schema::v0().with_attribute(
            "tags",
            Attribute::new(
                AttributeType::list(AttributeType::String),
                AttributeFlags::required(),
            ),
        );

        // Valid list
        let diagnostics = validate(&schema, &json!({"tags": ["a", "b", "c"]}));
        assert!(diagnostics.is_empty());

        // Empty list
        let diagnostics = validate(&schema, &json!({"tags": []}));
        assert!(diagnostics.is_empty());

        // Wrong element type
        let diagnostics = validate(&schema, &json!({"tags": ["a", 123, "c"]}));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].attribute, Some("tags.1".to_string()));

        // Not a list
        let diagnostics = validate(&schema, &json!({"tags": "not a list"}));
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_validate_map() {
        let schema = Schema::v0().with_attribute(
            "labels",
            Attribute::new(
                AttributeType::map(AttributeType::String),
                AttributeFlags::required(),
            ),
        );

        // Valid map
        let diagnostics = validate(&schema, &json!({"labels": {"env": "prod", "app": "web"}}));
        assert!(diagnostics.is_empty());

        // Wrong value type
        let diagnostics = validate(&schema, &json!({"labels": {"env": "prod", "count": 42}}));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].attribute, Some("labels.count".to_string()));
    }

    #[test]
    fn test_validate_nested_block_single() {
        let schema = Schema::v0().with_block(
            "config",
            NestedBlock::single(Block::new().with_attribute("enabled", Attribute::required_bool())),
        );

        // Valid
        let diagnostics = validate(&schema, &json!({"config": {"enabled": true}}));
        assert!(diagnostics.is_empty());

        // Missing optional block is ok
        let diagnostics = validate(&schema, &json!({}));
        assert!(diagnostics.is_empty());

        // Invalid nested attribute
        let diagnostics = validate(&schema, &json!({"config": {"enabled": "yes"}}));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].attribute, Some("config.enabled".to_string()));
    }

    #[test]
    fn test_validate_nested_block_list() {
        let schema = Schema::v0().with_block(
            "ingress",
            NestedBlock::list(Block::new().with_attribute("port", Attribute::required_int64()))
                .with_min_items(1)
                .with_max_items(3),
        );

        // Valid
        let diagnostics = validate(&schema, &json!({"ingress": [{"port": 80}, {"port": 443}]}));
        assert!(diagnostics.is_empty());

        // Too few items
        let diagnostics = validate(&schema, &json!({"ingress": []}));
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].summary.contains("at least 1"));

        // Too many items
        let diagnostics = validate(
            &schema,
            &json!({"ingress": [{"port": 80}, {"port": 443}, {"port": 8080}, {"port": 9090}]}),
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].summary.contains("at most 3"));

        // Invalid nested attribute
        let diagnostics = validate(&schema, &json!({"ingress": [{"port": "eighty"}]}));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].attribute, Some("ingress.0.port".to_string()));
    }

    #[test]
    fn test_validate_nested_block_map() {
        let schema = Schema::v0().with_block(
            "volumes",
            NestedBlock::map(
                Block::new().with_attribute("mount_path", Attribute::required_string()),
            ),
        );

        // Valid
        let diagnostics = validate(
            &schema,
            &json!({"volumes": {"data": {"mount_path": "/data"}, "logs": {"mount_path": "/logs"}}}),
        );
        assert!(diagnostics.is_empty());

        // Invalid nested attribute
        let diagnostics = validate(&schema, &json!({"volumes": {"data": {"mount_path": 123}}}));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].attribute,
            Some("volumes.data.mount_path".to_string())
        );
    }

    #[test]
    fn test_validate_multiple_errors() {
        let schema = Schema::v0()
            .with_attribute("name", Attribute::required_string())
            .with_attribute("count", Attribute::required_int64())
            .with_attribute("enabled", Attribute::required_bool());

        // All wrong types
        let diagnostics = validate(
            &schema,
            &json!({"name": 123, "count": "not a number", "enabled": "yes"}),
        );
        assert_eq!(diagnostics.len(), 3);
    }

    #[test]
    fn test_validate_deeply_nested() {
        let schema = Schema::v0().with_block(
            "network",
            NestedBlock::list(
                Block::new()
                    .with_attribute("name", Attribute::required_string())
                    .with_block(
                        "subnet",
                        NestedBlock::list(
                            Block::new().with_attribute("cidr", Attribute::required_string()),
                        ),
                    ),
            ),
        );

        // Valid
        let diagnostics = validate(
            &schema,
            &json!({
                "network": [{
                    "name": "vpc-1",
                    "subnet": [{"cidr": "10.0.0.0/24"}]
                }]
            }),
        );
        assert!(diagnostics.is_empty());

        // Invalid deeply nested
        let diagnostics = validate(
            &schema,
            &json!({
                "network": [{
                    "name": "vpc-1",
                    "subnet": [{"cidr": 123}]
                }]
            }),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].attribute,
            Some("network.0.subnet.0.cidr".to_string())
        );
    }

    #[test]
    fn test_is_valid_helper() {
        let schema = Schema::v0().with_attribute("name", Attribute::required_string());

        assert!(is_valid(&schema, &json!({"name": "test"})));
        assert!(!is_valid(&schema, &json!({})));
    }

    #[test]
    fn test_validate_result_helper() {
        let schema = Schema::v0().with_attribute("name", Attribute::required_string());

        assert!(validate_result(&schema, &json!({"name": "test"})).is_ok());

        let result = validate_result(&schema, &json!({}));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().len(), 1);
    }

    #[test]
    fn test_validate_object_type() {
        let mut object_attrs = HashMap::new();
        object_attrs.insert("host".to_string(), AttributeType::String);
        object_attrs.insert("port".to_string(), AttributeType::Int64);

        let schema = Schema::v0().with_attribute(
            "endpoint",
            Attribute::new(
                AttributeType::Object(object_attrs),
                AttributeFlags::required(),
            ),
        );

        // Valid
        let diagnostics = validate(
            &schema,
            &json!({"endpoint": {"host": "localhost", "port": 8080}}),
        );
        assert!(diagnostics.is_empty());

        // Wrong nested type
        let diagnostics = validate(
            &schema,
            &json!({"endpoint": {"host": "localhost", "port": "8080"}}),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].attribute, Some("endpoint.port".to_string()));
    }

    #[test]
    fn test_validate_dynamic_type() {
        let schema = Schema::v0().with_attribute(
            "metadata",
            Attribute::new(AttributeType::Dynamic, AttributeFlags::required()),
        );

        // Any value should be valid
        assert!(validate(&schema, &json!({"metadata": "string"})).is_empty());
        assert!(validate(&schema, &json!({"metadata": 123})).is_empty());
        assert!(validate(&schema, &json!({"metadata": {"nested": "object"}})).is_empty());
        assert!(validate(&schema, &json!({"metadata": [1, 2, 3]})).is_empty());
    }

    #[test]
    fn test_validate_root_not_object() {
        let schema = Schema::v0().with_attribute("name", Attribute::required_string());

        let diagnostics = validate(&schema, &json!("not an object"));
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].summary.contains("Expected object"));
    }
}
