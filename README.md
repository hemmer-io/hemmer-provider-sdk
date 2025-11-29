# Hemmer Provider SDK

[![CI](https://github.com/hemmer-io/hemmer-provider-sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/hemmer-io/hemmer-provider-sdk/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Rust SDK for building Hemmer providers**

The Hemmer Provider SDK provides gRPC protocol types and server helpers for building providers that integrate with [Hemmer](https://github.com/hemmer-io/hemmer), the next-generation Infrastructure as Code tool.

## Status

🚧 **Currently in active development**

## Features

- **Protocol Buffers**: Canonical provider protocol definition
- **Pre-compiled Types**: Committed Rust types (no build-time generation)
- **Provider Traits**: Simple traits for implementing providers
- **Server Helpers**: Easy provider startup with handshake protocol

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hemmer-provider-sdk = "0.1"
```

## Quick Start

```rust
use hemmer_provider_sdk::{serve, ProviderServer, ProviderError, CreateOutput, Plan};
use std::collections::HashMap;

pub struct MyProvider {
    // Provider state
}

#[tonic::async_trait]
impl ProviderServer for MyProvider {
    async fn configure(&self, config: HashMap<String, String>) -> Result<(), ProviderError> {
        // Initialize with credentials from config
        Ok(())
    }

    async fn create(
        &self,
        resource_type: &str,
        input: serde_json::Value,
    ) -> Result<CreateOutput, ProviderError> {
        // Create resource
        todo!()
    }

    // ... implement other methods
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = MyProvider::new();
    serve(provider).await
}
```

## Handshake Protocol

When a provider starts, it outputs to stdout:

```
HEMMER_PROVIDER|1|127.0.0.1:50051
```

Format: `HEMMER_PROVIDER|<protocol_version>|<address>`

This allows Hemmer to spawn the provider as a subprocess and connect via gRPC.

## Documentation

- [API Documentation](https://docs.rs/hemmer-provider-sdk)
- [Hemmer Documentation](https://github.com/hemmer-io/hemmer)

## Related Projects

- [hemmer](https://github.com/hemmer-io/hemmer) - The Hemmer IaC tool
- [hemmer-provider-generator](https://github.com/hemmer-io/hemmer-provider-generator) - Generate providers from OpenAPI specs

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
