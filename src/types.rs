//! Convenience types for provider implementations.
//!
//! These types provide a more ergonomic API over the raw protobuf types.

use serde::{Deserialize, Serialize};

/// A change to a single attribute during a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeChange {
    /// The path to the attribute that changed.
    pub path: String,
    /// The value before the change (JSON-encoded, None if creating).
    pub before: Option<serde_json::Value>,
    /// The value after the change (JSON-encoded, None if deleting).
    pub after: Option<serde_json::Value>,
}

impl AttributeChange {
    /// Create a new attribute change.
    pub fn new(
        path: impl Into<String>,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
    ) -> Self {
        Self {
            path: path.into(),
            before,
            after,
        }
    }

    /// Create a change for a new attribute.
    pub fn added(path: impl Into<String>, value: serde_json::Value) -> Self {
        Self::new(path, None, Some(value))
    }

    /// Create a change for a removed attribute.
    pub fn removed(path: impl Into<String>, value: serde_json::Value) -> Self {
        Self::new(path, Some(value), None)
    }

    /// Create a change for a modified attribute.
    pub fn modified(
        path: impl Into<String>,
        before: serde_json::Value,
        after: serde_json::Value,
    ) -> Self {
        Self::new(path, Some(before), Some(after))
    }
}

impl From<crate::generated::AttributeChange> for AttributeChange {
    fn from(proto: crate::generated::AttributeChange) -> Self {
        Self {
            path: proto.path,
            before: if proto.before.is_empty() {
                None
            } else {
                serde_json::from_slice(&proto.before).ok()
            },
            after: if proto.after.is_empty() {
                None
            } else {
                serde_json::from_slice(&proto.after).ok()
            },
        }
    }
}

impl From<AttributeChange> for crate::generated::AttributeChange {
    fn from(change: AttributeChange) -> Self {
        Self {
            path: change.path,
            before: change
                .before
                .map(|v| serde_json::to_vec(&v).unwrap_or_default())
                .unwrap_or_default(),
            after: change
                .after
                .map(|v| serde_json::to_vec(&v).unwrap_or_default())
                .unwrap_or_default(),
        }
    }
}

/// The result of a plan operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanResult {
    /// The planned state after the operation.
    pub planned_state: serde_json::Value,
    /// The list of attribute changes.
    pub changes: Vec<AttributeChange>,
    /// Whether the resource requires replacement.
    pub requires_replace: bool,
}

impl PlanResult {
    /// Create a plan result with no changes.
    pub fn no_change(state: serde_json::Value) -> Self {
        Self {
            planned_state: state,
            changes: Vec::new(),
            requires_replace: false,
        }
    }

    /// Create a plan result with changes.
    pub fn with_changes(
        planned_state: serde_json::Value,
        changes: Vec<AttributeChange>,
        requires_replace: bool,
    ) -> Self {
        Self {
            planned_state,
            changes,
            requires_replace,
        }
    }

    /// Automatically compute attribute changes by comparing prior and proposed states.
    ///
    /// This method walks both JSON trees and emits an `AttributeChange` for each difference.
    /// Nested objects use dot-notation paths (e.g., `"metadata.labels.app"`).
    ///
    /// # Arguments
    ///
    /// * `prior` - The previous state (None if creating a new resource)
    /// * `proposed` - The desired new state
    ///
    /// # Examples
    ///
    /// ```
    /// use hemmer_provider_sdk::PlanResult;
    /// use serde_json::json;
    ///
    /// // Creating a new resource
    /// let result = PlanResult::from_diff(None, &json!({"name": "test", "count": 1}));
    /// assert_eq!(result.changes.len(), 2);
    ///
    /// // Updating a resource
    /// let prior = json!({"name": "old", "count": 1});
    /// let proposed = json!({"name": "new", "count": 1});
    /// let result = PlanResult::from_diff(Some(&prior), &proposed);
    /// assert_eq!(result.changes.len(), 1);
    /// assert_eq!(result.changes[0].path, "name");
    ///
    /// // No changes
    /// let state = json!({"name": "same"});
    /// let result = PlanResult::from_diff(Some(&state), &state);
    /// assert!(result.changes.is_empty());
    /// ```
    pub fn from_diff(prior: Option<&serde_json::Value>, proposed: &serde_json::Value) -> Self {
        match prior {
            None => {
                // Creating new resource - all fields are additions
                let changes = collect_all_fields("", proposed);
                Self {
                    planned_state: proposed.clone(),
                    changes,
                    requires_replace: false,
                }
            },
            Some(prior_state) => {
                let changes = compute_json_diff("", prior_state, proposed);
                Self {
                    planned_state: proposed.clone(),
                    changes,
                    requires_replace: false,
                }
            },
        }
    }
}

/// Recursively collect all fields from a JSON value as additions.
///
/// Used when creating a new resource to mark all fields as added.
fn collect_all_fields(prefix: &str, value: &serde_json::Value) -> Vec<AttributeChange> {
    use serde_json::Value;

    match value {
        Value::Object(map) => {
            let mut changes = Vec::new();
            for (key, val) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };

                match val {
                    Value::Object(_) | Value::Array(_) => {
                        // Recursively collect nested fields
                        changes.extend(collect_all_fields(&path, val));
                    },
                    _ => {
                        // Leaf value
                        changes.push(AttributeChange::added(path, val.clone()));
                    },
                }
            }
            changes
        },
        Value::Array(arr) => {
            let mut changes = Vec::new();
            for (idx, val) in arr.iter().enumerate() {
                let path = if prefix.is_empty() {
                    format!("[{}]", idx)
                } else {
                    format!("{}[{}]", prefix, idx)
                };

                match val {
                    Value::Object(_) | Value::Array(_) => {
                        changes.extend(collect_all_fields(&path, val));
                    },
                    _ => {
                        changes.push(AttributeChange::added(path, val.clone()));
                    },
                }
            }
            changes
        },
        _ => {
            // Scalar value at root
            if !prefix.is_empty() {
                vec![AttributeChange::added(prefix, value.clone())]
            } else {
                vec![]
            }
        },
    }
}

/// Recursively compute differences between two JSON values.
///
/// Returns a list of `AttributeChange` objects representing all differences.
/// Nested objects use dot-notation paths (e.g., `"spec.replicas"`).
/// Array elements use bracket notation (e.g., `"items[0].name"`).
fn compute_json_diff(
    prefix: &str,
    prior: &serde_json::Value,
    proposed: &serde_json::Value,
) -> Vec<AttributeChange> {
    use serde_json::Value;

    // If values are identical, no changes
    if prior == proposed {
        return Vec::new();
    }

    match (prior, proposed) {
        // Both are objects - recursively compare fields
        (Value::Object(prior_map), Value::Object(proposed_map)) => {
            let mut changes = Vec::new();

            // Find all keys from both objects
            let mut all_keys = std::collections::HashSet::new();
            all_keys.extend(prior_map.keys());
            all_keys.extend(proposed_map.keys());

            for key in all_keys {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };

                match (prior_map.get(key), proposed_map.get(key)) {
                    (Some(prior_val), Some(proposed_val)) => {
                        // Field exists in both - check for differences
                        if prior_val != proposed_val {
                            match (prior_val, proposed_val) {
                                (Value::Object(_), Value::Object(_))
                                | (Value::Array(_), Value::Array(_)) => {
                                    // Recursively diff nested structures
                                    changes.extend(compute_json_diff(
                                        &path,
                                        prior_val,
                                        proposed_val,
                                    ));
                                },
                                _ => {
                                    // Different types or different leaf values
                                    changes.push(AttributeChange::modified(
                                        path,
                                        prior_val.clone(),
                                        proposed_val.clone(),
                                    ));
                                },
                            }
                        }
                    },
                    (Some(prior_val), None) => {
                        // Field was removed
                        match prior_val {
                            Value::Object(_) | Value::Array(_) => {
                                // Recursively mark all nested fields as removed
                                for change in collect_all_fields(&path, prior_val) {
                                    changes.push(AttributeChange::removed(
                                        change.path,
                                        change.after.unwrap(),
                                    ));
                                }
                            },
                            _ => {
                                changes.push(AttributeChange::removed(path, prior_val.clone()));
                            },
                        }
                    },
                    (None, Some(proposed_val)) => {
                        // Field was added
                        match proposed_val {
                            Value::Object(_) | Value::Array(_) => {
                                // Recursively mark all nested fields as added
                                changes.extend(collect_all_fields(&path, proposed_val));
                            },
                            _ => {
                                changes.push(AttributeChange::added(path, proposed_val.clone()));
                            },
                        }
                    },
                    (None, None) => {
                        // Should never happen
                    },
                }
            }

            changes
        },
        // Both are arrays - compare elements
        (Value::Array(prior_arr), Value::Array(proposed_arr)) => {
            let mut changes = Vec::new();
            let max_len = prior_arr.len().max(proposed_arr.len());

            for idx in 0..max_len {
                let path = if prefix.is_empty() {
                    format!("[{}]", idx)
                } else {
                    format!("{}[{}]", prefix, idx)
                };

                match (prior_arr.get(idx), proposed_arr.get(idx)) {
                    (Some(prior_val), Some(proposed_val)) => {
                        // Element exists in both arrays
                        if prior_val != proposed_val {
                            match (prior_val, proposed_val) {
                                (Value::Object(_), Value::Object(_))
                                | (Value::Array(_), Value::Array(_)) => {
                                    changes.extend(compute_json_diff(
                                        &path,
                                        prior_val,
                                        proposed_val,
                                    ));
                                },
                                _ => {
                                    changes.push(AttributeChange::modified(
                                        path,
                                        prior_val.clone(),
                                        proposed_val.clone(),
                                    ));
                                },
                            }
                        }
                    },
                    (Some(prior_val), None) => {
                        // Element was removed from array
                        match prior_val {
                            Value::Object(_) | Value::Array(_) => {
                                for change in collect_all_fields(&path, prior_val) {
                                    changes.push(AttributeChange::removed(
                                        change.path,
                                        change.after.unwrap(),
                                    ));
                                }
                            },
                            _ => {
                                changes.push(AttributeChange::removed(path, prior_val.clone()));
                            },
                        }
                    },
                    (None, Some(proposed_val)) => {
                        // Element was added to array
                        match proposed_val {
                            Value::Object(_) | Value::Array(_) => {
                                changes.extend(collect_all_fields(&path, proposed_val));
                            },
                            _ => {
                                changes.push(AttributeChange::added(path, proposed_val.clone()));
                            },
                        }
                    },
                    (None, None) => {
                        // Should never happen
                    },
                }
            }

            changes
        },
        // Different types or different scalar values
        _ => {
            if !prefix.is_empty() {
                vec![AttributeChange::modified(
                    prefix,
                    prior.clone(),
                    proposed.clone(),
                )]
            } else {
                // Root-level scalar change
                vec![AttributeChange::modified(
                    "(root)",
                    prior.clone(),
                    proposed.clone(),
                )]
            }
        },
    }
}

/// An imported resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedResource {
    /// The resource type.
    pub resource_type: String,
    /// The imported state.
    pub state: serde_json::Value,
}

impl ImportedResource {
    /// Create a new imported resource.
    pub fn new(resource_type: impl Into<String>, state: serde_json::Value) -> Self {
        Self {
            resource_type: resource_type.into(),
            state,
        }
    }
}

/// Provider metadata returned by GetMetadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderMetadata {
    /// List of resource type names.
    pub resources: Vec<String>,
    /// List of data source type names.
    pub data_sources: Vec<String>,
    /// Server capabilities.
    pub capabilities: ServerCapabilities,
}

/// Server capability flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    /// Whether the provider supports planning destroy operations.
    pub plan_destroy: bool,
}

/// The current protocol version.
///
/// This should be incremented when making breaking changes to the gRPC service definition.
pub const PROTOCOL_VERSION: u32 = 1;

/// The minimum supported protocol version for backwards compatibility.
///
/// Providers should reject connections from clients using protocol versions
/// older than this value.
pub const MIN_PROTOCOL_VERSION: u32 = 1;

/// The handshake prefix output by providers.
pub const HANDSHAKE_PREFIX: &str = "HEMMER_PROVIDER";

/// Checks if a client protocol version is compatible with this provider.
///
/// Returns `Ok(())` if the client version is compatible, or an error message if not.
///
/// # Arguments
///
/// * `client_version` - The protocol version reported by the client
///
/// # Examples
///
/// ```
/// use hemmer_provider_sdk::{check_protocol_version, PROTOCOL_VERSION};
///
/// // Current version is always compatible
/// assert!(check_protocol_version(PROTOCOL_VERSION).is_ok());
///
/// // Older versions within MIN_PROTOCOL_VERSION range are compatible
/// assert!(check_protocol_version(1).is_ok());
///
/// // Versions below minimum are rejected
/// assert!(check_protocol_version(0).is_err());
/// ```
pub fn check_protocol_version(client_version: u32) -> Result<(), String> {
    if client_version < MIN_PROTOCOL_VERSION {
        return Err(format!(
            "Client protocol version {} is too old. Minimum supported version is {}",
            client_version, MIN_PROTOCOL_VERSION
        ));
    }

    if client_version > PROTOCOL_VERSION {
        // Client is newer than us - we'll try to be compatible
        // but warn that the client might expect features we don't support
        tracing::warn!(
            client_version = client_version,
            server_version = PROTOCOL_VERSION,
            "Client protocol version is newer than server version. Some features may not be available."
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_change_constructors() {
        let added = AttributeChange::added("name", serde_json::json!("test"));
        assert!(added.before.is_none());
        assert_eq!(added.after, Some(serde_json::json!("test")));

        let removed = AttributeChange::removed("name", serde_json::json!("old"));
        assert_eq!(removed.before, Some(serde_json::json!("old")));
        assert!(removed.after.is_none());

        let modified =
            AttributeChange::modified("count", serde_json::json!(1), serde_json::json!(2));
        assert_eq!(modified.before, Some(serde_json::json!(1)));
        assert_eq!(modified.after, Some(serde_json::json!(2)));
    }

    #[test]
    fn test_attribute_change_conversion() {
        let change =
            AttributeChange::modified("field", serde_json::json!("old"), serde_json::json!("new"));

        // Convert to proto
        let proto: crate::generated::AttributeChange = change.clone().into();
        assert_eq!(proto.path, "field");

        // Convert back
        let back: AttributeChange = proto.into();
        assert_eq!(back.path, change.path);
        assert_eq!(back.before, change.before);
        assert_eq!(back.after, change.after);
    }

    #[test]
    fn test_plan_result() {
        let no_change = PlanResult::no_change(serde_json::json!({"id": "123"}));
        assert!(no_change.changes.is_empty());
        assert!(!no_change.requires_replace);

        let with_changes = PlanResult::with_changes(
            serde_json::json!({"id": "123", "name": "new"}),
            vec![AttributeChange::modified(
                "name",
                serde_json::json!("old"),
                serde_json::json!("new"),
            )],
            false,
        );
        assert_eq!(with_changes.changes.len(), 1);
    }

    #[test]
    fn test_imported_resource() {
        let imported =
            ImportedResource::new("aws_s3_bucket", serde_json::json!({"id": "my-bucket"}));
        assert_eq!(imported.resource_type, "aws_s3_bucket");
        assert_eq!(imported.state["id"], "my-bucket");
    }

    #[test]
    fn test_protocol_constants() {
        assert_eq!(PROTOCOL_VERSION, 1);
        assert_eq!(MIN_PROTOCOL_VERSION, 1);
        assert_eq!(HANDSHAKE_PREFIX, "HEMMER_PROVIDER");
    }

    #[test]
    fn test_check_protocol_version_current() {
        // Current version should always be compatible
        assert!(check_protocol_version(PROTOCOL_VERSION).is_ok());
    }

    #[test]
    fn test_check_protocol_version_minimum() {
        // Minimum version should be compatible
        assert!(check_protocol_version(MIN_PROTOCOL_VERSION).is_ok());
    }

    #[test]
    fn test_check_protocol_version_too_old() {
        // Version below minimum should be rejected
        let result = check_protocol_version(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too old"));
    }

    #[test]
    fn test_check_protocol_version_newer() {
        // Newer version should be accepted (with warning)
        assert!(check_protocol_version(PROTOCOL_VERSION + 1).is_ok());
    }

    #[test]
    fn test_from_diff_create_simple() {
        // Creating a new resource with simple fields
        let proposed = serde_json::json!({
            "name": "test",
            "count": 42
        });

        let result = PlanResult::from_diff(None, &proposed);
        assert_eq!(result.planned_state, proposed);
        assert_eq!(result.changes.len(), 2);

        // Check that both fields are marked as added
        let paths: std::collections::HashSet<_> =
            result.changes.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains("name"));
        assert!(paths.contains("count"));

        // Verify they're all additions
        for change in &result.changes {
            assert!(change.before.is_none());
            assert!(change.after.is_some());
        }
    }

    #[test]
    fn test_from_diff_create_nested() {
        // Creating a resource with nested objects
        let proposed = serde_json::json!({
            "name": "test",
            "metadata": {
                "labels": {
                    "app": "myapp",
                    "env": "prod"
                }
            }
        });

        let result = PlanResult::from_diff(None, &proposed);

        // Should have changes for: name, metadata.labels.app, metadata.labels.env
        assert_eq!(result.changes.len(), 3);

        let paths: std::collections::HashSet<_> =
            result.changes.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains("name"));
        assert!(paths.contains("metadata.labels.app"));
        assert!(paths.contains("metadata.labels.env"));
    }

    #[test]
    fn test_from_diff_create_with_array() {
        // Creating a resource with arrays
        let proposed = serde_json::json!({
            "name": "test",
            "tags": ["web", "production"]
        });

        let result = PlanResult::from_diff(None, &proposed);

        // Should have changes for: name, tags[0], tags[1]
        assert_eq!(result.changes.len(), 3);

        let paths: std::collections::HashSet<_> =
            result.changes.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains("name"));
        assert!(paths.contains("tags[0]"));
        assert!(paths.contains("tags[1]"));
    }

    #[test]
    fn test_from_diff_no_changes() {
        // Updating with identical state
        let state = serde_json::json!({
            "name": "test",
            "count": 42
        });

        let result = PlanResult::from_diff(Some(&state), &state);
        assert_eq!(result.changes.len(), 0);
        assert_eq!(result.planned_state, state);
    }

    #[test]
    fn test_from_diff_simple_modification() {
        // Modifying a single field
        let prior = serde_json::json!({
            "name": "old",
            "count": 1
        });

        let proposed = serde_json::json!({
            "name": "new",
            "count": 1
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "name");
        assert_eq!(change.before, Some(serde_json::json!("old")));
        assert_eq!(change.after, Some(serde_json::json!("new")));
    }

    #[test]
    fn test_from_diff_nested_modification() {
        // Modifying a nested field
        let prior = serde_json::json!({
            "metadata": {
                "labels": {
                    "app": "oldapp",
                    "env": "prod"
                }
            }
        });

        let proposed = serde_json::json!({
            "metadata": {
                "labels": {
                    "app": "newapp",
                    "env": "prod"
                }
            }
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "metadata.labels.app");
        assert_eq!(change.before, Some(serde_json::json!("oldapp")));
        assert_eq!(change.after, Some(serde_json::json!("newapp")));
    }

    #[test]
    fn test_from_diff_add_field() {
        // Adding a new field to existing resource
        let prior = serde_json::json!({
            "name": "test"
        });

        let proposed = serde_json::json!({
            "name": "test",
            "count": 42
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "count");
        assert!(change.before.is_none());
        assert_eq!(change.after, Some(serde_json::json!(42)));
    }

    #[test]
    fn test_from_diff_remove_field() {
        // Removing a field from existing resource
        let prior = serde_json::json!({
            "name": "test",
            "count": 42
        });

        let proposed = serde_json::json!({
            "name": "test"
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "count");
        assert_eq!(change.before, Some(serde_json::json!(42)));
        assert!(change.after.is_none());
    }

    #[test]
    fn test_from_diff_add_nested_object() {
        // Adding a nested object
        let prior = serde_json::json!({
            "name": "test"
        });

        let proposed = serde_json::json!({
            "name": "test",
            "metadata": {
                "labels": {
                    "app": "myapp"
                }
            }
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "metadata.labels.app");
        assert!(change.before.is_none());
        assert_eq!(change.after, Some(serde_json::json!("myapp")));
    }

    #[test]
    fn test_from_diff_remove_nested_object() {
        // Removing a nested object
        let prior = serde_json::json!({
            "name": "test",
            "metadata": {
                "labels": {
                    "app": "myapp"
                }
            }
        });

        let proposed = serde_json::json!({
            "name": "test"
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "metadata.labels.app");
        assert_eq!(change.before, Some(serde_json::json!("myapp")));
        assert!(change.after.is_none());
    }

    #[test]
    fn test_from_diff_array_modification() {
        // Modifying array element
        let prior = serde_json::json!({
            "tags": ["web", "production"]
        });

        let proposed = serde_json::json!({
            "tags": ["web", "staging"]
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "tags[1]");
        assert_eq!(change.before, Some(serde_json::json!("production")));
        assert_eq!(change.after, Some(serde_json::json!("staging")));
    }

    #[test]
    fn test_from_diff_array_add_element() {
        // Adding element to array
        let prior = serde_json::json!({
            "tags": ["web"]
        });

        let proposed = serde_json::json!({
            "tags": ["web", "production"]
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "tags[1]");
        assert!(change.before.is_none());
        assert_eq!(change.after, Some(serde_json::json!("production")));
    }

    #[test]
    fn test_from_diff_array_remove_element() {
        // Removing element from array
        let prior = serde_json::json!({
            "tags": ["web", "production"]
        });

        let proposed = serde_json::json!({
            "tags": ["web"]
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "tags[1]");
        assert_eq!(change.before, Some(serde_json::json!("production")));
        assert!(change.after.is_none());
    }

    #[test]
    fn test_from_diff_complex_nested_structure() {
        // Complex nested structure with multiple changes
        let prior = serde_json::json!({
            "name": "test",
            "spec": {
                "replicas": 3,
                "template": {
                    "metadata": {
                        "labels": {
                            "app": "oldapp",
                            "version": "v1"
                        }
                    }
                }
            }
        });

        let proposed = serde_json::json!({
            "name": "test",
            "spec": {
                "replicas": 5,
                "template": {
                    "metadata": {
                        "labels": {
                            "app": "newapp",
                            "version": "v1"
                        }
                    }
                }
            }
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 2);

        let paths: std::collections::HashSet<_> =
            result.changes.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains("spec.replicas"));
        assert!(paths.contains("spec.template.metadata.labels.app"));
    }

    #[test]
    fn test_from_diff_type_change() {
        // Changing type of a field (number to string)
        let prior = serde_json::json!({
            "value": 42
        });

        let proposed = serde_json::json!({
            "value": "42"
        });

        let result = PlanResult::from_diff(Some(&prior), &proposed);
        assert_eq!(result.changes.len(), 1);

        let change = &result.changes[0];
        assert_eq!(change.path, "value");
        assert_eq!(change.before, Some(serde_json::json!(42)));
        assert_eq!(change.after, Some(serde_json::json!("42")));
    }
}
