//! Hemmer Provider SDK
//!
//! This crate provides the gRPC protocol types and server helpers for building
//! Hemmer providers. It follows the pattern established by
//! [terraform-plugin-go](https://github.com/hashicorp/terraform-plugin-go).
//!
//! # Overview
//!
//! The SDK provides:
//!
//! - **Protocol Buffers types**: Pre-compiled Rust types from the canonical provider protocol
//! - **Schema types**: Types for describing provider, resource, and data source schemas
//! - **ProviderService trait**: A high-level trait that providers implement
//! - **Server helpers**: Functions to start a gRPC server with the handshake protocol
//! - **Error types**: Common error types for provider implementations
//! - **Logging**: Integration with `tracing` for structured logging
//!
//! # Quick Start
//!
//! ```ignore
//! use hemmer_provider_sdk::{
//!     serve, ProviderService, ProviderError, PlanResult,
//!     schema::{ProviderSchema, Schema, Attribute, Diagnostic},
//! };
//!
//! struct MyProvider;
//!
//! #[async_trait::async_trait]
//! impl ProviderService for MyProvider {
//!     fn schema(&self) -> ProviderSchema {
//!         ProviderSchema::new()
//!             .with_resource("example_resource", Schema::v0()
//!                 .with_attribute("name", Attribute::required_string())
//!                 .with_attribute("id", Attribute::computed_string()))
//!     }
//!
//!     async fn configure(
//!         &self,
//!         config: serde_json::Value,
//!     ) -> Result<Vec<Diagnostic>, ProviderError> {
//!         Ok(vec![])
//!     }
//!
//!     async fn plan(
//!         &self,
//!         resource_type: &str,
//!         prior_state: Option<serde_json::Value>,
//!         proposed_state: serde_json::Value,
//!         config: serde_json::Value,
//!     ) -> Result<PlanResult, ProviderError> {
//!         Ok(PlanResult::no_change(proposed_state))
//!     }
//!
//!     async fn create(
//!         &self,
//!         resource_type: &str,
//!         planned_state: serde_json::Value,
//!     ) -> Result<serde_json::Value, ProviderError> {
//!         Ok(planned_state)
//!     }
//!
//!     async fn read(
//!         &self,
//!         resource_type: &str,
//!         current_state: serde_json::Value,
//!     ) -> Result<serde_json::Value, ProviderError> {
//!         Ok(current_state)
//!     }
//!
//!     async fn update(
//!         &self,
//!         resource_type: &str,
//!         prior_state: serde_json::Value,
//!         planned_state: serde_json::Value,
//!     ) -> Result<serde_json::Value, ProviderError> {
//!         Ok(planned_state)
//!     }
//!
//!     async fn delete(
//!         &self,
//!         resource_type: &str,
//!         current_state: serde_json::Value,
//!     ) -> Result<(), ProviderError> {
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let provider = MyProvider;
//!     serve(provider).await
//! }
//! ```
//!
//! # Handshake Protocol
//!
//! When a provider starts via [`serve`], it outputs a handshake string to stdout:
//!
//! ```text
//! HEMMER_PROVIDER|1|127.0.0.1:50051
//! ```
//!
//! Format: `HEMMER_PROVIDER|<protocol_version>|<address>`
//!
//! This allows Hemmer to spawn the provider as a subprocess and connect via gRPC.
//!
//! # Provider Protocol
//!
//! The SDK implements a complete provider protocol similar to Terraform's:
//!
//! - **GetMetadata**: Returns provider capabilities and resource/data source names
//! - **GetSchema**: Returns full schema for provider config, resources, and data sources
//! - **ValidateProviderConfig**: Validates provider configuration
//! - **Configure**: Configures the provider with credentials
//! - **Stop**: Gracefully shuts down the provider
//! - **ValidateResourceConfig**: Validates resource configuration
//! - **UpgradeResourceState**: Migrates state from older schema versions
//! - **Plan**: Calculates required changes
//! - **Create/Read/Update/Delete**: CRUD operations for resources
//! - **ImportResourceState**: Imports existing infrastructure
//! - **ValidateDataSourceConfig**: Validates data source configuration
//! - **ReadDataSource**: Reads data from external sources

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod logging;
pub mod schema;
pub mod server;
pub mod types;
pub mod validation;

#[allow(missing_docs)]
#[allow(clippy::all)]
pub mod generated;

// Re-export main types at crate root
pub use error::ProviderError;
pub use logging::{init_logging, init_logging_with_default, try_init_logging};
pub use schema::ProviderSchema;
pub use server::{
    serve, serve_on, serve_on_with_options, serve_with_options, ProviderService, ServeOptions,
};
pub use types::{
    AttributeChange, ImportedResource, PlanResult, ProviderMetadata, ServerCapabilities,
    HANDSHAKE_PREFIX, PROTOCOL_VERSION,
};
pub use validation::{is_valid, validate, validate_result};

// Re-export async_trait for convenience
pub use async_trait::async_trait;

// Re-export commonly used external types
pub use serde_json;
pub use tonic;
pub use tracing;
