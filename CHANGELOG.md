# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-11-29

### Added

- Complete gRPC provider protocol implementation
- Schema types for defining provider, resource, and data source schemas
- Pre-compiled Protocol Buffer types (no build-time proto generation required)
- `ProviderService` trait for implementing providers
- `serve()` and `serve_on()` functions for starting the gRPC server
- Handshake protocol for Hemmer to connect to providers
- Validation helpers (`validate`, `is_valid`, `validate_result`)
- Testing utilities (`ProviderTester`) for provider implementations
- Logging integration with `tracing`
- Support for state upgrades and resource imports
- Data source support

[0.1.0]: https://github.com/hemmer-io/hemmer-provider-sdk/releases/tag/v0.1.0
