//! Schema types for describing provider and resource structure.
//!
//! Schemas describe the shape of provider configuration, resources, and data sources.
//! They enable validation, documentation generation, and proper state management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The type of an attribute value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeType {
    /// A string value.
    String,
    /// A 64-bit integer.
    Int64,
    /// A 64-bit floating point number.
    Float64,
    /// A boolean value.
    Bool,
    /// A list of values of a single type.
    List(Box<AttributeType>),
    /// A set of unique values of a single type.
    Set(Box<AttributeType>),
    /// A map from string keys to values of a single type.
    Map(Box<AttributeType>),
    /// An object with a fixed set of attributes.
    Object(HashMap<String, AttributeType>),
    /// A dynamic type that can hold any value (use sparingly).
    Dynamic,
}

impl AttributeType {
    /// Create a list type.
    pub fn list(element_type: AttributeType) -> Self {
        Self::List(Box::new(element_type))
    }

    /// Create a set type.
    pub fn set(element_type: AttributeType) -> Self {
        Self::Set(Box::new(element_type))
    }

    /// Create a map type.
    pub fn map(element_type: AttributeType) -> Self {
        Self::Map(Box::new(element_type))
    }

    /// Create an object type.
    pub fn object(attributes: HashMap<String, AttributeType>) -> Self {
        Self::Object(attributes)
    }
}

/// Describes how an attribute can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AttributeFlags {
    /// The attribute is required in configuration.
    pub required: bool,
    /// The attribute is optional in configuration.
    pub optional: bool,
    /// The attribute is computed by the provider (read-only).
    pub computed: bool,
    /// The attribute is sensitive and should be hidden in logs/UI.
    pub sensitive: bool,
}

impl AttributeFlags {
    /// Create flags for a required attribute.
    pub fn required() -> Self {
        Self {
            required: true,
            ..Default::default()
        }
    }

    /// Create flags for an optional attribute.
    pub fn optional() -> Self {
        Self {
            optional: true,
            ..Default::default()
        }
    }

    /// Create flags for a computed attribute (read-only, set by provider).
    pub fn computed() -> Self {
        Self {
            computed: true,
            ..Default::default()
        }
    }

    /// Create flags for an optional+computed attribute (can be set, but has default from provider).
    pub fn optional_computed() -> Self {
        Self {
            optional: true,
            computed: true,
            ..Default::default()
        }
    }

    /// Mark the attribute as sensitive.
    pub fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }
}

/// Describes a single attribute in a schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// The type of the attribute.
    #[serde(rename = "type")]
    pub attr_type: AttributeType,
    /// Flags describing how the attribute can be used.
    #[serde(flatten)]
    pub flags: AttributeFlags,
    /// Human-readable description of the attribute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// If set, changing this attribute forces resource replacement.
    #[serde(default)]
    pub force_new: bool,
    /// Default value for the attribute (JSON-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

impl Attribute {
    /// Create a new attribute with the given type and flags.
    pub fn new(attr_type: AttributeType, flags: AttributeFlags) -> Self {
        Self {
            attr_type,
            flags,
            description: None,
            force_new: false,
            default: None,
        }
    }

    /// Create a required string attribute.
    pub fn required_string() -> Self {
        Self::new(AttributeType::String, AttributeFlags::required())
    }

    /// Create an optional string attribute.
    pub fn optional_string() -> Self {
        Self::new(AttributeType::String, AttributeFlags::optional())
    }

    /// Create a computed string attribute.
    pub fn computed_string() -> Self {
        Self::new(AttributeType::String, AttributeFlags::computed())
    }

    /// Create a required int64 attribute.
    pub fn required_int64() -> Self {
        Self::new(AttributeType::Int64, AttributeFlags::required())
    }

    /// Create an optional int64 attribute.
    pub fn optional_int64() -> Self {
        Self::new(AttributeType::Int64, AttributeFlags::optional())
    }

    /// Create a computed int64 attribute.
    pub fn computed_int64() -> Self {
        Self::new(AttributeType::Int64, AttributeFlags::computed())
    }

    /// Create a required bool attribute.
    pub fn required_bool() -> Self {
        Self::new(AttributeType::Bool, AttributeFlags::required())
    }

    /// Create an optional bool attribute.
    pub fn optional_bool() -> Self {
        Self::new(AttributeType::Bool, AttributeFlags::optional())
    }

    /// Create a computed bool attribute.
    pub fn computed_bool() -> Self {
        Self::new(AttributeType::Bool, AttributeFlags::computed())
    }

    /// Set the description for this attribute.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark this attribute as forcing resource replacement when changed.
    pub fn with_force_new(mut self) -> Self {
        self.force_new = true;
        self
    }

    /// Set a default value for this attribute.
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self
    }

    /// Mark this attribute as sensitive.
    pub fn sensitive(mut self) -> Self {
        self.flags.sensitive = true;
        self
    }
}

/// The nesting mode for a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BlockNestingMode {
    /// A single nested block (at most one).
    #[default]
    Single,
    /// A list of nested blocks (zero or more, ordered).
    List,
    /// A set of nested blocks (zero or more, unordered, unique).
    Set,
    /// A map of nested blocks keyed by string.
    Map,
}

/// Type alias for [`BlockNestingMode`] for generator compatibility.
///
/// This is an alias to maintain compatibility with hemmer-provider-generator v0.3.5.
pub type NestingMode = BlockNestingMode;

/// Type alias for [`BlockNestingMode`] for generator compatibility.
///
/// This is an alias to maintain compatibility with hemmer-provider-generator v0.3.5.
pub type BlockType = BlockNestingMode;

/// A nested block within a schema.
///
/// Blocks are used for complex nested structures that have their own
/// set of attributes (e.g., `ingress` blocks in a security group).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// The attributes within this block.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, Attribute>,
    /// Nested blocks within this block.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub blocks: HashMap<String, NestedBlock>,
    /// Human-readable description of the block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Block {
    /// Create a new empty block.
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
            blocks: HashMap::new(),
            description: None,
        }
    }

    /// Add an attribute to this block.
    pub fn with_attribute(mut self, name: impl Into<String>, attr: Attribute) -> Self {
        self.attributes.insert(name.into(), attr);
        self
    }

    /// Add a nested block to this block.
    pub fn with_block(mut self, name: impl Into<String>, block: NestedBlock) -> Self {
        self.blocks.insert(name.into(), block);
        self
    }

    /// Set the description for this block.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

/// A nested block with its nesting mode and constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NestedBlock {
    /// The block definition.
    #[serde(flatten)]
    pub block: Block,
    /// How the block is nested (single, list, set, map).
    #[serde(default)]
    pub nesting_mode: BlockNestingMode,
    /// Minimum number of blocks required.
    #[serde(default)]
    pub min_items: u32,
    /// Maximum number of blocks allowed (0 = unlimited).
    #[serde(default)]
    pub max_items: u32,
}

impl NestedBlock {
    /// Create a single nested block (0 or 1 allowed).
    pub fn single(block: Block) -> Self {
        Self {
            block,
            nesting_mode: BlockNestingMode::Single,
            min_items: 0,
            max_items: 1,
        }
    }

    /// Create a list of nested blocks.
    pub fn list(block: Block) -> Self {
        Self {
            block,
            nesting_mode: BlockNestingMode::List,
            min_items: 0,
            max_items: 0,
        }
    }

    /// Create a set of nested blocks.
    pub fn set(block: Block) -> Self {
        Self {
            block,
            nesting_mode: BlockNestingMode::Set,
            min_items: 0,
            max_items: 0,
        }
    }

    /// Create a map of nested blocks.
    pub fn map(block: Block) -> Self {
        Self {
            block,
            nesting_mode: BlockNestingMode::Map,
            min_items: 0,
            max_items: 0,
        }
    }

    /// Set the minimum number of blocks required.
    pub fn with_min_items(mut self, min: u32) -> Self {
        self.min_items = min;
        self
    }

    /// Set the maximum number of blocks allowed.
    pub fn with_max_items(mut self, max: u32) -> Self {
        self.max_items = max;
        self
    }
}

/// Schema for a resource or data source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    /// The version of this schema (for state upgrades).
    #[serde(default)]
    pub version: u64,
    /// The root block containing all attributes and nested blocks.
    #[serde(flatten)]
    pub block: Block,
}

impl Schema {
    /// Create a new schema with the given version.
    pub fn new(version: u64) -> Self {
        Self {
            version,
            block: Block::new(),
        }
    }

    /// Create a schema at version 0.
    pub fn v0() -> Self {
        Self::new(0)
    }

    /// Add an attribute to the schema.
    pub fn with_attribute(mut self, name: impl Into<String>, attr: Attribute) -> Self {
        self.block.attributes.insert(name.into(), attr);
        self
    }

    /// Add a nested block to the schema.
    pub fn with_block(mut self, name: impl Into<String>, block: NestedBlock) -> Self {
        self.block.blocks.insert(name.into(), block);
        self
    }
}

/// Schema for the provider configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProviderSchema {
    /// Schema for provider configuration.
    #[serde(default)]
    pub provider: Schema,
    /// Schemas for each resource type.
    #[serde(default)]
    pub resources: HashMap<String, Schema>,
    /// Schemas for each data source type.
    #[serde(default)]
    pub data_sources: HashMap<String, Schema>,
}

impl ProviderSchema {
    /// Create a new empty provider schema.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the provider configuration schema.
    pub fn with_provider_config(mut self, schema: Schema) -> Self {
        self.provider = schema;
        self
    }

    /// Add a resource schema.
    pub fn with_resource(mut self, name: impl Into<String>, schema: Schema) -> Self {
        self.resources.insert(name.into(), schema);
        self
    }

    /// Add a data source schema.
    pub fn with_data_source(mut self, name: impl Into<String>, schema: Schema) -> Self {
        self.data_sources.insert(name.into(), schema);
        self
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::v0()
    }
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    /// An error that prevents the operation from completing.
    Error,
    /// A warning that doesn't prevent the operation but should be addressed.
    Warning,
}

/// A diagnostic message from the provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The severity of the diagnostic.
    pub severity: DiagnosticSeverity,
    /// A short summary of the issue.
    pub summary: String,
    /// A detailed description of the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The attribute path where the issue occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
}

impl Diagnostic {
    /// Create an error diagnostic.
    pub fn error(summary: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            summary: summary.into(),
            detail: None,
            attribute: None,
        }
    }

    /// Create a warning diagnostic.
    pub fn warning(summary: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            summary: summary.into(),
            detail: None,
            attribute: None,
        }
    }

    /// Add detail to this diagnostic.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the attribute path for this diagnostic.
    pub fn with_attribute(mut self, attribute: impl Into<String>) -> Self {
        self.attribute = Some(attribute.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_type_constructors() {
        let list = AttributeType::list(AttributeType::String);
        assert!(matches!(list, AttributeType::List(_)));

        let map = AttributeType::map(AttributeType::Int64);
        assert!(matches!(map, AttributeType::Map(_)));
    }

    #[test]
    fn test_attribute_flags() {
        let required = AttributeFlags::required();
        assert!(required.required);
        assert!(!required.optional);
        assert!(!required.computed);

        let computed = AttributeFlags::computed();
        assert!(!computed.required);
        assert!(!computed.optional);
        assert!(computed.computed);

        let optional_computed = AttributeFlags::optional_computed();
        assert!(!optional_computed.required);
        assert!(optional_computed.optional);
        assert!(optional_computed.computed);

        let sensitive = AttributeFlags::required().sensitive();
        assert!(sensitive.sensitive);
    }

    #[test]
    fn test_attribute_builders() {
        let attr = Attribute::required_string()
            .with_description("A test attribute")
            .with_force_new();

        assert_eq!(attr.attr_type, AttributeType::String);
        assert!(attr.flags.required);
        assert_eq!(attr.description, Some("A test attribute".to_string()));
        assert!(attr.force_new);
    }

    #[test]
    fn test_schema_builder() {
        let schema = Schema::v0()
            .with_attribute("name", Attribute::required_string())
            .with_attribute("id", Attribute::computed_string())
            .with_block(
                "config",
                NestedBlock::single(
                    Block::new().with_attribute("enabled", Attribute::optional_bool()),
                ),
            );

        assert_eq!(schema.version, 0);
        assert!(schema.block.attributes.contains_key("name"));
        assert!(schema.block.attributes.contains_key("id"));
        assert!(schema.block.blocks.contains_key("config"));
    }

    #[test]
    fn test_provider_schema() {
        let provider_schema = ProviderSchema::new()
            .with_provider_config(
                Schema::v0().with_attribute("api_key", Attribute::required_string().sensitive()),
            )
            .with_resource(
                "example_resource",
                Schema::v0()
                    .with_attribute("name", Attribute::required_string())
                    .with_attribute("id", Attribute::computed_string()),
            )
            .with_data_source(
                "example_data",
                Schema::v0().with_attribute("filter", Attribute::optional_string()),
            );

        assert!(provider_schema
            .provider
            .block
            .attributes
            .contains_key("api_key"));
        assert!(provider_schema.resources.contains_key("example_resource"));
        assert!(provider_schema.data_sources.contains_key("example_data"));
    }

    #[test]
    fn test_diagnostic() {
        let err = Diagnostic::error("Invalid configuration")
            .with_detail("The value must be positive")
            .with_attribute("count");

        assert_eq!(err.severity, DiagnosticSeverity::Error);
        assert_eq!(err.summary, "Invalid configuration");
        assert_eq!(err.detail, Some("The value must be positive".to_string()));
        assert_eq!(err.attribute, Some("count".to_string()));
    }

    #[test]
    fn test_nested_block_modes() {
        let single = NestedBlock::single(Block::new());
        assert_eq!(single.nesting_mode, BlockNestingMode::Single);
        assert_eq!(single.max_items, 1);

        let list = NestedBlock::list(Block::new())
            .with_min_items(1)
            .with_max_items(5);
        assert_eq!(list.nesting_mode, BlockNestingMode::List);
        assert_eq!(list.min_items, 1);
        assert_eq!(list.max_items, 5);
    }
}
