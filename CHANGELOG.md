# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2025-11-30

### Added

- Additional `ProviderError` variants for comprehensive gRPC status mapping (#35)
  - `AlreadyExists` - Resource already exists (create conflict)
  - `PermissionDenied` - Permission denied (authentication/authorization failure)
  - `ResourceExhausted` - Quota or rate limit exceeded
  - `Unavailable` - Service temporarily unavailable
  - `DeadlineExceeded` - Operation timed out
  - `FailedPrecondition` - Operation failed due to current state (precondition not met)
  - `Unimplemented` - Operation not implemented
  - Updated `From<ProviderError> for tonic::Status` with new variant mappings
  - Comprehensive test coverage for all new error variants
- Error Handling section in README with usage examples for all error variants

## [0.2.1] - 2025-11-29

### Added

- `PlanResult::from_diff()` method for automatic plan diff computation (#32)
  - Automatically computes attribute changes by comparing prior and proposed states
  - Eliminates need for manual `AttributeChange` construction in providers
  - Supports nested objects with dot-notation paths (e.g., `"metadata.labels.app"`)
  - Supports arrays with bracket notation (e.g., `"tags[0]"`)
  - Comprehensive test coverage for various diff scenarios

## [0.2.0] - 2025-11-29

### Added

- Protocol version negotiation for provider compatibility (#29, #30)
  - `PROTOCOL_VERSION` constant (current version: 1)
  - `MIN_PROTOCOL_VERSION` constant (minimum supported version: 1)
  - `check_protocol_version()` helper function for version validation
  - Version fields in `GetSchemaRequest` (`client_protocol_version`) and `GetSchemaResponse` (`server_protocol_version`)
  - Comprehensive test coverage for version negotiation scenarios
  - Documentation of versioning strategy in README

### Changed

- **BREAKING**: `GetSchemaRequest` now requires `client_protocol_version` field
- **BREAKING**: `GetSchemaResponse` now includes `server_protocol_version` field
- Updated gRPC protocol to validate client versions during `GetSchema` RPC
- Removed unnecessary Terraform references from documentation

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

[0.3.0]: https://github.com/hemmer-io/hemmer-provider-sdk/releases/tag/v0.3.0
[0.2.1]: https://github.com/hemmer-io/hemmer-provider-sdk/releases/tag/v0.2.1
[0.2.0]: https://github.com/hemmer-io/hemmer-provider-sdk/releases/tag/v0.2.0
[0.1.0]: https://github.com/hemmer-io/hemmer-provider-sdk/releases/tag/v0.1.0
