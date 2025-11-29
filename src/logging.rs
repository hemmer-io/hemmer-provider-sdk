//! Logging and tracing utilities for providers.
//!
//! This module provides helpers for setting up structured logging using the
//! `tracing` ecosystem. All logs are written to **stderr** to avoid interfering
//! with the handshake protocol on stdout.
//!
//! # Quick Start
//!
//! ```ignore
//! use hemmer_provider_sdk::{serve, init_logging};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize logging (reads RUST_LOG env var)
//!     init_logging();
//!
//!     tracing::info!("Starting provider");
//!     serve(MyProvider).await
//! }
//! ```
//!
//! # Environment Variables
//!
//! - `RUST_LOG`: Controls log levels (e.g., `info`, `debug`, `hemmer_provider_sdk=debug`)
//!
//! # Examples
//!
//! ```bash
//! # Show info logs (default)
//! RUST_LOG=info ./my-provider
//!
//! # Show debug logs for the SDK
//! RUST_LOG=hemmer_provider_sdk=debug ./my-provider
//!
//! # Show all debug logs
//! RUST_LOG=debug ./my-provider
//! ```

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize the default logging subscriber.
///
/// This sets up a `tracing` subscriber that:
/// - Writes to **stderr** (stdout is reserved for the handshake protocol)
/// - Respects the `RUST_LOG` environment variable for filtering
/// - Defaults to `info` level if `RUST_LOG` is not set
/// - Uses a compact, human-readable format
///
/// # Panics
///
/// Panics if a global subscriber has already been set.
///
/// # Example
///
/// ```ignore
/// use hemmer_provider_sdk::init_logging;
///
/// fn main() {
///     init_logging();
///     tracing::info!("Provider starting");
/// }
/// ```
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .init();
}

/// Initialize logging with a custom default level.
///
/// Like [`init_logging`], but allows specifying a default log level
/// that will be used if `RUST_LOG` is not set.
///
/// # Arguments
///
/// * `default_level` - The default log level (e.g., "debug", "info", "warn")
///
/// # Example
///
/// ```ignore
/// use hemmer_provider_sdk::init_logging_with_default;
///
/// fn main() {
///     // Default to debug level if RUST_LOG is not set
///     init_logging_with_default("debug");
/// }
/// ```
pub fn init_logging_with_default(default_level: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .init();
}

/// Try to initialize logging, returning false if already initialized.
///
/// Unlike [`init_logging`], this function does not panic if a subscriber
/// has already been set. This is useful in test scenarios or when
/// the provider might be initialized multiple times.
///
/// # Returns
///
/// - `true` if the subscriber was successfully set
/// - `false` if a subscriber was already set
///
/// # Example
///
/// ```ignore
/// use hemmer_provider_sdk::try_init_logging;
///
/// fn main() {
///     if !try_init_logging() {
///         eprintln!("Logging already initialized");
///     }
/// }
/// ```
pub fn try_init_logging() -> bool {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .try_init()
        .is_ok()
}

#[cfg(test)]
mod tests {
    // Note: We can't easily test logging initialization in unit tests
    // because the global subscriber can only be set once per process.
    // These tests would need to be run in separate processes.

    use super::*;

    #[test]
    fn test_env_filter_parsing() {
        // Test that EnvFilter can parse various formats
        assert!(EnvFilter::try_new("info").is_ok());
        assert!(EnvFilter::try_new("debug").is_ok());
        assert!(EnvFilter::try_new("hemmer_provider_sdk=debug").is_ok());
        assert!(EnvFilter::try_new("warn,hemmer_provider_sdk=debug").is_ok());
    }
}
