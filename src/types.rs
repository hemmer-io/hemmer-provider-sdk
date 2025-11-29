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

/// The protocol version for the handshake.
pub const PROTOCOL_VERSION: u32 = 1;

/// The handshake prefix output by providers.
pub const HANDSHAKE_PREFIX: &str = "HEMMER_PROVIDER";

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
        assert_eq!(HANDSHAKE_PREFIX, "HEMMER_PROVIDER");
    }
}
