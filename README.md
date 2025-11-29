# Hemmer Provider SDK

[![Crates.io](https://img.shields.io/crates/v/hemmer-provider-sdk.svg)](https://crates.io/crates/hemmer-provider-sdk)
[![Documentation](https://docs.rs/hemmer-provider-sdk/badge.svg)](https://docs.rs/hemmer-provider-sdk)
[![CI](https://github.com/hemmer-io/hemmer-provider-sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/hemmer-io/hemmer-provider-sdk/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Rust SDK for building Hemmer providers**

The Hemmer Provider SDK provides gRPC protocol types and server helpers for building providers that integrate with [Hemmer](https://github.com/hemmer-io/hemmer), the next-generation Infrastructure as Code tool.

## Status

🚧 **Currently in active development**

## Features

- **Complete Provider Protocol**: Full gRPC protocol similar to Terraform's provider protocol
- **Schema Support**: Define resource and data source schemas with types, validation, and documentation
- **Pre-compiled Types**: Committed Rust types (no build-time proto generation required)
- **Server Helpers**: Easy provider startup with handshake protocol
- **Validation**: Built-in support for provider, resource, and data source config validation
- **State Management**: Support for state upgrades and resource imports

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hemmer-provider-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## Quick Start

```rust
use hemmer_provider_sdk::{
    serve, ProviderService, ProviderError, PlanResult,
    schema::{ProviderSchema, Schema, Attribute, Diagnostic},
};

struct MyProvider;

#[hemmer_provider_sdk::async_trait]
impl ProviderService for MyProvider {
    fn schema(&self) -> ProviderSchema {
        ProviderSchema::new()
            .with_provider_config(
                Schema::v0()
                    .with_attribute("api_key", Attribute::required_string().sensitive())
            )
            .with_resource("mycloud_instance", Schema::v0()
                .with_attribute("name", Attribute::required_string())
                .with_attribute("size", Attribute::optional_string())
                .with_attribute("id", Attribute::computed_string())
            )
    }

    async fn configure(
        &self,
        config: serde_json::Value,
    ) -> Result<Vec<Diagnostic>, ProviderError> {
        // Initialize provider with credentials
        Ok(vec![])
    }

    async fn plan(
        &self,
        resource_type: &str,
        prior_state: Option<serde_json::Value>,
        proposed_state: serde_json::Value,
        config: serde_json::Value,
    ) -> Result<PlanResult, ProviderError> {
        Ok(PlanResult::no_change(proposed_state))
    }

    async fn create(
        &self,
        resource_type: &str,
        planned_state: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        // Create the resource and return its state
        Ok(planned_state)
    }

    async fn read(
        &self,
        resource_type: &str,
        current_state: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        // Read current state from the API
        Ok(current_state)
    }

    async fn update(
        &self,
        resource_type: &str,
        prior_state: serde_json::Value,
        planned_state: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        // Update the resource
        Ok(planned_state)
    }

    async fn delete(
        &self,
        resource_type: &str,
        current_state: serde_json::Value,
    ) -> Result<(), ProviderError> {
        // Delete the resource
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    serve(MyProvider).await
}
```

## Provider Protocol

The SDK implements a complete provider protocol with the following RPCs:

| RPC | Purpose |
|-----|---------|
| `GetMetadata` | Returns provider capabilities and resource/data source names |
| `GetSchema` | Returns full schema for provider config, resources, and data sources |
| `ValidateProviderConfig` | Validates provider configuration before use |
| `Configure` | Configures provider with credentials and settings |
| `Stop` | Gracefully shuts down the provider |
| `ValidateResourceConfig` | Validates resource configuration before planning |
| `UpgradeResourceState` | Migrates state from older schema versions |
| `Plan` | Calculates required changes to reach desired state |
| `Create` | Creates a new resource |
| `Read` | Reads current state of a resource |
| `Update` | Updates an existing resource |
| `Delete` | Deletes a resource |
| `ImportResourceState` | Imports existing infrastructure |
| `ValidateDataSourceConfig` | Validates data source configuration |
| `ReadDataSource` | Reads data from external sources |

## Schema Types

Define schemas for your resources using the builder pattern:

```rust
use hemmer_provider_sdk::schema::*;

let schema = Schema::v0()
    // Required string attribute
    .with_attribute("name", Attribute::required_string()
        .with_description("The name of the resource"))

    // Optional with default
    .with_attribute("region", Attribute::optional_string()
        .with_default(serde_json::json!("us-east-1")))

    // Computed (read-only) attribute
    .with_attribute("id", Attribute::computed_string())

    // Sensitive attribute (hidden in logs)
    .with_attribute("password", Attribute::required_string().sensitive())

    // Force replacement when changed
    .with_attribute("ami", Attribute::required_string().with_force_new())

    // Nested blocks
    .with_block("network", NestedBlock::list(
        Block::new()
            .with_attribute("subnet_id", Attribute::required_string())
            .with_attribute("security_groups",
                Attribute::new(AttributeType::list(AttributeType::String),
                              AttributeFlags::optional()))
    ));
```

## Validation

The SDK provides built-in validation helpers to validate configuration values against schemas:

```rust
use hemmer_provider_sdk::{validate, is_valid, schema::Schema};
use serde_json::json;

let schema = Schema::v0()
    .with_attribute("name", Attribute::required_string())
    .with_attribute("count", Attribute::optional(AttributeType::Number));

let value = json!({
    "name": "my-resource",
    "count": 5
});

// Get detailed validation diagnostics
let diagnostics = validate(&schema, &value);
if diagnostics.is_empty() {
    println!("Configuration is valid!");
}

// Or use the simple boolean check
if is_valid(&schema, &value) {
    println!("Valid!");
}
```

## Testing

The SDK includes a test harness for provider implementations:

```rust
use hemmer_provider_sdk::{ProviderTester, ProviderService};
use serde_json::json;

#[tokio::test]
async fn test_resource_lifecycle() {
    let provider = MyProvider::new();
    let tester = ProviderTester::new(provider);

    // Test complete CRUD lifecycle
    let final_state = tester
        .lifecycle_crud(
            "mycloud_instance",
            json!({"name": "test"}),           // create config
            json!({"name": "test-updated"}),   // update config
        )
        .await
        .expect("lifecycle should succeed");

    // Or test individual operations with assertions
    let plan = tester
        .plan("mycloud_instance", None, json!({"name": "test"}))
        .await
        .unwrap();

    tester.assert_plan_creates(&plan);
}
```

## Handshake Protocol

When a provider starts via `serve()`, it outputs a handshake string to stdout:

```
HEMMER_PROVIDER|1|127.0.0.1:50051
```

Format: `HEMMER_PROVIDER|<protocol_version>|<address>`

This allows Hemmer to spawn the provider as a subprocess and connect via gRPC.

## Contributing

### Quick Setup

```bash
# Clone and setup development environment
git clone https://github.com/hemmer-io/hemmer-provider-sdk
cd hemmer-provider-sdk
./scripts/setup.sh
```

The setup script will:
- Install git pre-commit hooks
- Verify your Rust toolchain
- Run an initial build and test

### Development Workflow

```bash
# Build
cargo build

# Run tests
cargo test

# Run linter
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt --all

# View documentation
cargo doc --no-deps --open
```

### Regenerating Proto Types

Proto types are pre-compiled and committed. To regenerate after changing `proto/provider.proto`:

```bash
# Requires protoc to be installed
cargo build --features regenerate-proto
```

### Code Style

- Follow standard Rust conventions (rustfmt, clippy)
- Document public APIs with doc comments
- Write tests for new functionality
- Keep commits focused and atomic

### Pull Request Process

1. Create a feature branch from `main`
2. Make your changes with tests
3. Ensure all checks pass (`cargo test`, `cargo clippy`, `cargo fmt --check`)
4. Submit a PR using the template
5. Address review feedback

## Related Projects

- [hemmer](https://github.com/hemmer-io/hemmer) - The Hemmer IaC tool
- [hemmer-provider-generator](https://github.com/hemmer-io/hemmer-provider-generator) - Generate providers from OpenAPI specs

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
