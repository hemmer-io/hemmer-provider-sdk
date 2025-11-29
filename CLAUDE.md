# CLAUDE.md - Project Context for Claude Code

This file provides context for Claude Code when working on the Hemmer Provider SDK.

## Project Overview

The Hemmer Provider SDK is a Rust library for building providers that integrate with [Hemmer](https://github.com/hemmer-io/hemmer), a next-generation Infrastructure as Code tool. It provides:

- **gRPC Protocol**: Complete provider protocol similar to Terraform's tfprotov6
- **Schema System**: Types for defining resource and data source schemas
- **Server Helpers**: Functions to start a gRPC server with handshake protocol
- **Error Types**: Common error types for provider implementations

## Architecture

```text
hemmer-provider-sdk/
├── src/
│   ├── lib.rs          # Public API exports
│   ├── server.rs       # ProviderService trait and serve() functions
│   ├── schema.rs       # Schema types (Attribute, Block, NestedBlock, etc.)
│   ├── types.rs        # Convenience types (PlanResult, ImportedResource, etc.)
│   ├── error.rs        # ProviderError enum
│   └── generated.rs    # Pre-compiled protobuf types (do not edit manually)
├── proto/
│   └── provider.proto  # Protocol definition (source of truth)
├── scripts/
│   ├── pre-commit      # Pre-commit hook (fmt, clippy, tests)
│   └── setup.sh        # Developer setup script
└── .github/
    ├── workflows/ci.yml    # CI pipeline
    └── CODEOWNERS          # Code review assignments
```

## Key Files

| File | Purpose |
|------|---------|
| `src/server.rs` | Core `ProviderService` trait that providers implement, plus `serve()` functions |
| `src/schema.rs` | Schema builder types for defining resources and data sources |
| `src/types.rs` | Helper types like `PlanResult`, `AttributeChange`, `ImportedResource` |
| `src/error.rs` | `ProviderError` enum with tonic::Status conversion |
| `proto/provider.proto` | Protocol definition - regenerate with `cargo build --features regenerate-proto` |

## Common Development Tasks

### Initial Setup

```bash
# Clone and setup
git clone https://github.com/hemmer-io/hemmer-provider-sdk
cd hemmer-provider-sdk
./scripts/setup.sh
```

### Building

```bash
cargo build                           # Debug build
cargo build --release                 # Release build
cargo build --features regenerate-proto  # Regenerate proto types
```

### Testing

```bash
cargo test                  # Run all tests
cargo test --doc            # Run doc tests only
cargo test <test_name>      # Run specific test
```

### Linting and Formatting

```bash
cargo fmt --all             # Format code
cargo fmt --all -- --check  # Check formatting
cargo clippy --all-targets -- -D warnings  # Run clippy
```

### Documentation

```bash
cargo doc --no-deps --open  # Generate and open docs
```

## Code Style

- Follow standard Rust conventions (rustfmt, clippy)
- Use `thiserror` for error types
- Use builder pattern for schema types
- Prefer `async_trait` for async trait methods
- Document public APIs with doc comments
- Keep functions focused and small

## Testing Guidelines

- Unit tests go in the same file as the code being tested (in a `tests` module)
- Integration tests go in `tests/` directory
- Use descriptive test names: `test_<what>_<expected_behavior>`
- Test error cases, not just happy paths

## Protocol Overview

The SDK implements 14 gRPC RPCs:

| RPC | Purpose |
|-----|---------|
| `GetMetadata` | Provider capabilities and resource names |
| `GetSchema` | Full schema for provider, resources, data sources |
| `ValidateProviderConfig` | Validate provider configuration |
| `Configure` | Configure provider with credentials |
| `Stop` | Graceful shutdown |
| `ValidateResourceConfig` | Validate resource configuration |
| `UpgradeResourceState` | Migrate state from older schema versions |
| `Plan` | Calculate required changes |
| `Create` | Create a new resource |
| `Read` | Read current state of a resource |
| `Update` | Update an existing resource |
| `Delete` | Delete a resource |
| `ImportResourceState` | Import existing infrastructure |
| `ReadDataSource` | Read data from external sources |

## Handshake Protocol

When a provider starts, it outputs to stdout:

```text
HEMMER_PROVIDER|<protocol_version>|<address>
```

Example: `HEMMER_PROVIDER|1|127.0.0.1:50051`

This allows Hemmer to spawn providers as subprocesses and connect via gRPC.

---

## Issue Templates

### Bug Report

```markdown
## Description
[Clear description of the bug]

## Steps to Reproduce
1. [First step]
2. [Second step]
3. [...]

## Expected Behavior
[What should happen]

## Actual Behavior
[What actually happens]

## Environment
- Rust version: [e.g., 1.75.0]
- OS: [e.g., macOS 14.0]
- SDK version: [e.g., 0.1.0]

## Additional Context
[Any other relevant information]
```

### Feature Request

```markdown
## Description
[Clear description of the feature]

## Use Case
[Why is this feature needed? What problem does it solve?]

## Proposed Solution
[How should this feature work?]

## Alternatives Considered
[Other approaches you've considered]

## Additional Context
[Any other relevant information]
```

---

## Pull Request Template

```markdown
## Summary
[Brief description of the changes]

## Related Issue
Closes #[issue_number]

## Changes Made
- [Change 1]
- [Change 2]
- [...]

## Test Plan
- [ ] [Test case 1]
- [ ] [Test case 2]

## Checklist
- [ ] Code follows project style guidelines
- [ ] Tests pass locally (`cargo test`)
- [ ] Clippy passes (`cargo clippy --all-targets -- -D warnings`)
- [ ] Documentation updated if needed
- [ ] Breaking changes documented (if any)
```

---

## Labels

Use these labels for issues and PRs:

### Type

- `bug` - Something isn't working
- `enhancement` - New feature or request
- `documentation` - Improvements to documentation
- `testing` - Related to tests or testing utilities
- `security` - Security-related issue
- `performance` - Performance-related issue

### Priority

- `priority: high` - High priority issue
- `priority: low` - Low priority issue

### Status

- `needs-review` - Awaiting code review
- `blocked` - Blocked by another issue or external dependency
- `help wanted` - Extra attention is needed
- `good first issue` - Good for newcomers

### Resolution

- `duplicate` - This issue or pull request already exists
- `invalid` - This doesn't seem right
- `wontfix` - This will not be worked on

### Breaking Changes

- `breaking-change` - Introduces breaking API changes

### Dependencies (Dependabot)

- `dependencies` - Pull requests that update a dependency
- `rust` - Pull requests that update Rust code
