//! Server helpers for running providers.
//!
//! This module provides the `ProviderService` trait that providers implement,
//! and the `serve` function to start a gRPC server with the handshake protocol.
//!
//! # Signal Handling
//!
//! The server automatically handles OS signals (SIGTERM, SIGINT) for graceful shutdown.
//! When a signal is received, the server:
//! 1. Stops accepting new connections
//! 2. Waits for in-flight requests to complete (with configurable timeout)
//! 3. Calls the provider's `stop()` method
//! 4. Exits cleanly

use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpListener;
use tonic::transport::Server;

use crate::error::ProviderError;
use crate::schema::{Diagnostic, DiagnosticSeverity, ProviderSchema};
use crate::types::{
    ImportedResource, PlanResult, ProviderMetadata, HANDSHAKE_PREFIX, PROTOCOL_VERSION,
};

/// Trait that provider implementations must implement.
///
/// This provides a higher-level API than the raw gRPC trait, using
/// ergonomic Rust types instead of protobuf types.
///
/// # Example
///
/// ```ignore
/// use hemmer_provider_sdk::{ProviderService, ProviderError, PlanResult, ProviderSchema};
/// use hemmer_provider_sdk::schema::{Schema, Attribute, Diagnostic};
///
/// struct MyProvider;
///
/// #[async_trait::async_trait]
/// impl ProviderService for MyProvider {
///     fn schema(&self) -> ProviderSchema {
///         ProviderSchema::new()
///             .with_resource("example_resource", Schema::v0()
///                 .with_attribute("name", Attribute::required_string()))
///     }
///
///     async fn configure(&self, config: serde_json::Value) -> Result<Vec<Diagnostic>, ProviderError> {
///         Ok(vec![])
///     }
///
///     // ... implement other methods
/// }
/// ```
#[async_trait::async_trait]
pub trait ProviderService: Send + Sync + 'static {
    // =========================================================================
    // Schema & Metadata
    // =========================================================================

    /// Return the provider's schema including all resources and data sources.
    fn schema(&self) -> ProviderSchema;

    /// Return provider metadata for performance optimization.
    /// By default, this is derived from the schema.
    fn metadata(&self) -> ProviderMetadata {
        let schema = self.schema();
        ProviderMetadata {
            resources: schema.resources.keys().cloned().collect(),
            data_sources: schema.data_sources.keys().cloned().collect(),
            capabilities: Default::default(),
        }
    }

    // =========================================================================
    // Provider Lifecycle
    // =========================================================================

    /// Validate the provider configuration before configuring.
    /// Returns diagnostics (errors and warnings).
    async fn validate_provider_config(
        &self,
        config: serde_json::Value,
    ) -> Result<Vec<Diagnostic>, ProviderError> {
        let _ = config;
        Ok(vec![])
    }

    /// Configure the provider with credentials and settings.
    /// Returns diagnostics (errors and warnings).
    async fn configure(&self, config: serde_json::Value) -> Result<Vec<Diagnostic>, ProviderError>;

    /// Stop the provider gracefully.
    async fn stop(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    // =========================================================================
    // Resource Operations
    // =========================================================================

    /// Validate a resource's configuration before planning.
    async fn validate_resource_config(
        &self,
        resource_type: &str,
        config: serde_json::Value,
    ) -> Result<Vec<Diagnostic>, ProviderError> {
        let _ = (resource_type, config);
        Ok(vec![])
    }

    /// Upgrade resource state from an older schema version.
    async fn upgrade_resource_state(
        &self,
        resource_type: &str,
        version: i64,
        state: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        let _ = (resource_type, version);
        // Default: no upgrade needed, return state as-is
        Ok(state)
    }

    /// Plan changes for a resource.
    async fn plan(
        &self,
        resource_type: &str,
        prior_state: Option<serde_json::Value>,
        proposed_state: serde_json::Value,
        config: serde_json::Value,
    ) -> Result<PlanResult, ProviderError>;

    /// Create a new resource.
    async fn create(
        &self,
        resource_type: &str,
        planned_state: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError>;

    /// Read the current state of a resource.
    async fn read(
        &self,
        resource_type: &str,
        current_state: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError>;

    /// Update an existing resource.
    async fn update(
        &self,
        resource_type: &str,
        prior_state: serde_json::Value,
        planned_state: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError>;

    /// Delete a resource.
    async fn delete(
        &self,
        resource_type: &str,
        current_state: serde_json::Value,
    ) -> Result<(), ProviderError>;

    /// Import existing infrastructure into management.
    async fn import_resource(
        &self,
        resource_type: &str,
        _id: &str,
    ) -> Result<Vec<ImportedResource>, ProviderError> {
        Err(ProviderError::Sdk(format!(
            "Import not supported for resource type: {}",
            resource_type
        )))
    }

    // =========================================================================
    // Data Source Operations
    // =========================================================================

    /// Validate a data source's configuration.
    async fn validate_data_source_config(
        &self,
        data_source_type: &str,
        config: serde_json::Value,
    ) -> Result<Vec<Diagnostic>, ProviderError> {
        let _ = (data_source_type, config);
        Ok(vec![])
    }

    /// Read data from an external source.
    async fn read_data_source(
        &self,
        data_source_type: &str,
        _config: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        Err(ProviderError::UnknownResource(format!(
            "Unknown data source type: {}",
            data_source_type
        )))
    }
}

/// Wrapper that implements the generated gRPC trait.
struct ProviderGrpcService<P: ProviderService> {
    provider: P,
}

impl<P: ProviderService> ProviderGrpcService<P> {
    fn diagnostics_to_proto(
        &self,
        diagnostics: Vec<Diagnostic>,
    ) -> Vec<crate::generated::Diagnostic> {
        diagnostics
            .into_iter()
            .map(|d| crate::generated::Diagnostic {
                severity: match d.severity {
                    DiagnosticSeverity::Error => {
                        crate::generated::diagnostic::Severity::Error as i32
                    }
                    DiagnosticSeverity::Warning => {
                        crate::generated::diagnostic::Severity::Warning as i32
                    }
                },
                summary: d.summary,
                detail: d.detail.unwrap_or_default(),
                attribute: d.attribute.unwrap_or_default(),
            })
            .collect()
    }

    fn error_to_diagnostics(&self, err: ProviderError) -> Vec<crate::generated::Diagnostic> {
        vec![crate::generated::Diagnostic {
            severity: crate::generated::diagnostic::Severity::Error as i32,
            summary: err.to_string(),
            detail: String::new(),
            attribute: String::new(),
        }]
    }

    fn schema_to_proto(&self, schema: &crate::schema::Schema) -> crate::generated::Schema {
        crate::generated::Schema {
            version: schema.version as i64,
            block: Some(block_to_proto(&schema.block)),
        }
    }
}

fn block_to_proto(block: &crate::schema::Block) -> crate::generated::Block {
    crate::generated::Block {
        attributes: block
            .attributes
            .iter()
            .map(|(name, attr)| crate::generated::Attribute {
                name: name.clone(),
                r#type: serde_json::to_vec(&attr.attr_type).unwrap_or_default(),
                required: attr.flags.required,
                optional: attr.flags.optional,
                computed: attr.flags.computed,
                sensitive: attr.flags.sensitive,
                description: attr.description.clone().unwrap_or_default(),
                force_new: attr.force_new,
                default_value: attr
                    .default
                    .as_ref()
                    .map(|v| serde_json::to_vec(v).unwrap_or_default())
                    .unwrap_or_default(),
            })
            .collect(),
        block_types: block
            .blocks
            .iter()
            .map(|(name, nested)| crate::generated::NestedBlock {
                type_name: name.clone(),
                block: Some(block_to_proto(&nested.block)),
                nesting_mode: match nested.nesting_mode {
                    crate::schema::BlockNestingMode::Single => {
                        crate::generated::nested_block::NestingMode::Single as i32
                    }
                    crate::schema::BlockNestingMode::List => {
                        crate::generated::nested_block::NestingMode::List as i32
                    }
                    crate::schema::BlockNestingMode::Set => {
                        crate::generated::nested_block::NestingMode::Set as i32
                    }
                    crate::schema::BlockNestingMode::Map => {
                        crate::generated::nested_block::NestingMode::Map as i32
                    }
                },
                min_items: nested.min_items as i32,
                max_items: nested.max_items as i32,
            })
            .collect(),
        description: block.description.clone().unwrap_or_default(),
    }
}

#[tonic::async_trait]
impl<P: ProviderService> crate::generated::provider_server::Provider for ProviderGrpcService<P> {
    async fn get_metadata(
        &self,
        _request: tonic::Request<crate::generated::GetMetadataRequest>,
    ) -> Result<tonic::Response<crate::generated::GetMetadataResponse>, tonic::Status> {
        let metadata = self.provider.metadata();
        Ok(tonic::Response::new(
            crate::generated::GetMetadataResponse {
                server_capabilities: Some(crate::generated::ServerCapabilities {
                    plan_destroy: metadata.capabilities.plan_destroy,
                }),
                resources: metadata.resources,
                data_sources: metadata.data_sources,
                diagnostics: vec![],
            },
        ))
    }

    async fn get_schema(
        &self,
        _request: tonic::Request<crate::generated::GetSchemaRequest>,
    ) -> Result<tonic::Response<crate::generated::GetSchemaResponse>, tonic::Status> {
        let schema = self.provider.schema();
        Ok(tonic::Response::new(crate::generated::GetSchemaResponse {
            provider: Some(self.schema_to_proto(&schema.provider)),
            resources: schema
                .resources
                .iter()
                .map(|(k, v)| (k.clone(), self.schema_to_proto(v)))
                .collect(),
            data_sources: schema
                .data_sources
                .iter()
                .map(|(k, v)| (k.clone(), self.schema_to_proto(v)))
                .collect(),
            diagnostics: vec![],
        }))
    }

    async fn validate_provider_config(
        &self,
        request: tonic::Request<crate::generated::ValidateProviderConfigRequest>,
    ) -> Result<tonic::Response<crate::generated::ValidateProviderConfigResponse>, tonic::Status>
    {
        let req = request.into_inner();
        let config = serde_json::from_slice(&req.config).unwrap_or(serde_json::Value::Null);

        match self.provider.validate_provider_config(config).await {
            Ok(diagnostics) => Ok(tonic::Response::new(
                crate::generated::ValidateProviderConfigResponse {
                    diagnostics: self.diagnostics_to_proto(diagnostics),
                },
            )),
            Err(e) => Ok(tonic::Response::new(
                crate::generated::ValidateProviderConfigResponse {
                    diagnostics: self.error_to_diagnostics(e),
                },
            )),
        }
    }

    async fn configure(
        &self,
        request: tonic::Request<crate::generated::ConfigureRequest>,
    ) -> Result<tonic::Response<crate::generated::ConfigureResponse>, tonic::Status> {
        let req = request.into_inner();
        let config = serde_json::from_slice(&req.config).unwrap_or(serde_json::Value::Null);

        match self.provider.configure(config).await {
            Ok(diagnostics) => Ok(tonic::Response::new(crate::generated::ConfigureResponse {
                diagnostics: self.diagnostics_to_proto(diagnostics),
            })),
            Err(e) => Ok(tonic::Response::new(crate::generated::ConfigureResponse {
                diagnostics: self.error_to_diagnostics(e),
            })),
        }
    }

    async fn stop(
        &self,
        _request: tonic::Request<crate::generated::StopRequest>,
    ) -> Result<tonic::Response<crate::generated::StopResponse>, tonic::Status> {
        match self.provider.stop().await {
            Ok(()) => Ok(tonic::Response::new(crate::generated::StopResponse {
                error: String::new(),
            })),
            Err(e) => Ok(tonic::Response::new(crate::generated::StopResponse {
                error: e.to_string(),
            })),
        }
    }

    async fn validate_resource_config(
        &self,
        request: tonic::Request<crate::generated::ValidateResourceConfigRequest>,
    ) -> Result<tonic::Response<crate::generated::ValidateResourceConfigResponse>, tonic::Status>
    {
        let req = request.into_inner();
        let config = serde_json::from_slice(&req.config).unwrap_or(serde_json::Value::Null);

        match self
            .provider
            .validate_resource_config(&req.resource_type, config)
            .await
        {
            Ok(diagnostics) => Ok(tonic::Response::new(
                crate::generated::ValidateResourceConfigResponse {
                    diagnostics: self.diagnostics_to_proto(diagnostics),
                },
            )),
            Err(e) => Ok(tonic::Response::new(
                crate::generated::ValidateResourceConfigResponse {
                    diagnostics: self.error_to_diagnostics(e),
                },
            )),
        }
    }

    async fn upgrade_resource_state(
        &self,
        request: tonic::Request<crate::generated::UpgradeResourceStateRequest>,
    ) -> Result<tonic::Response<crate::generated::UpgradeResourceStateResponse>, tonic::Status>
    {
        let req = request.into_inner();
        let state = serde_json::from_slice(&req.raw_state).unwrap_or(serde_json::Value::Null);

        match self
            .provider
            .upgrade_resource_state(&req.resource_type, req.version, state)
            .await
        {
            Ok(upgraded) => Ok(tonic::Response::new(
                crate::generated::UpgradeResourceStateResponse {
                    upgraded_state: serde_json::to_vec(&upgraded).unwrap_or_default(),
                    diagnostics: vec![],
                },
            )),
            Err(e) => Ok(tonic::Response::new(
                crate::generated::UpgradeResourceStateResponse {
                    upgraded_state: vec![],
                    diagnostics: self.error_to_diagnostics(e),
                },
            )),
        }
    }

    async fn plan(
        &self,
        request: tonic::Request<crate::generated::PlanRequest>,
    ) -> Result<tonic::Response<crate::generated::PlanResponse>, tonic::Status> {
        let req = request.into_inner();

        let prior_state = if req.prior_state.is_empty() {
            None
        } else {
            serde_json::from_slice(&req.prior_state).ok()
        };

        let proposed_state =
            serde_json::from_slice(&req.proposed_state).unwrap_or(serde_json::Value::Null);
        let config = serde_json::from_slice(&req.config).unwrap_or(serde_json::Value::Null);

        match self
            .provider
            .plan(&req.resource_type, prior_state, proposed_state, config)
            .await
        {
            Ok(result) => Ok(tonic::Response::new(crate::generated::PlanResponse {
                planned_state: serde_json::to_vec(&result.planned_state).unwrap_or_default(),
                changes: result.changes.into_iter().map(Into::into).collect(),
                requires_replace: result.requires_replace,
                diagnostics: vec![],
            })),
            Err(e) => Ok(tonic::Response::new(crate::generated::PlanResponse {
                planned_state: vec![],
                changes: vec![],
                requires_replace: false,
                diagnostics: self.error_to_diagnostics(e),
            })),
        }
    }

    async fn create(
        &self,
        request: tonic::Request<crate::generated::CreateRequest>,
    ) -> Result<tonic::Response<crate::generated::CreateResponse>, tonic::Status> {
        let req = request.into_inner();
        let planned_state =
            serde_json::from_slice(&req.planned_state).unwrap_or(serde_json::Value::Null);

        match self
            .provider
            .create(&req.resource_type, planned_state)
            .await
        {
            Ok(state) => Ok(tonic::Response::new(crate::generated::CreateResponse {
                state: serde_json::to_vec(&state).unwrap_or_default(),
                diagnostics: vec![],
            })),
            Err(e) => Ok(tonic::Response::new(crate::generated::CreateResponse {
                state: vec![],
                diagnostics: self.error_to_diagnostics(e),
            })),
        }
    }

    async fn read(
        &self,
        request: tonic::Request<crate::generated::ReadRequest>,
    ) -> Result<tonic::Response<crate::generated::ReadResponse>, tonic::Status> {
        let req = request.into_inner();
        let current_state =
            serde_json::from_slice(&req.current_state).unwrap_or(serde_json::Value::Null);

        match self.provider.read(&req.resource_type, current_state).await {
            Ok(state) => Ok(tonic::Response::new(crate::generated::ReadResponse {
                state: serde_json::to_vec(&state).unwrap_or_default(),
                diagnostics: vec![],
            })),
            Err(e) => Ok(tonic::Response::new(crate::generated::ReadResponse {
                state: vec![],
                diagnostics: self.error_to_diagnostics(e),
            })),
        }
    }

    async fn update(
        &self,
        request: tonic::Request<crate::generated::UpdateRequest>,
    ) -> Result<tonic::Response<crate::generated::UpdateResponse>, tonic::Status> {
        let req = request.into_inner();
        let prior_state =
            serde_json::from_slice(&req.prior_state).unwrap_or(serde_json::Value::Null);
        let planned_state =
            serde_json::from_slice(&req.planned_state).unwrap_or(serde_json::Value::Null);

        match self
            .provider
            .update(&req.resource_type, prior_state, planned_state)
            .await
        {
            Ok(state) => Ok(tonic::Response::new(crate::generated::UpdateResponse {
                state: serde_json::to_vec(&state).unwrap_or_default(),
                diagnostics: vec![],
            })),
            Err(e) => Ok(tonic::Response::new(crate::generated::UpdateResponse {
                state: vec![],
                diagnostics: self.error_to_diagnostics(e),
            })),
        }
    }

    async fn delete(
        &self,
        request: tonic::Request<crate::generated::DeleteRequest>,
    ) -> Result<tonic::Response<crate::generated::DeleteResponse>, tonic::Status> {
        let req = request.into_inner();
        let current_state =
            serde_json::from_slice(&req.current_state).unwrap_or(serde_json::Value::Null);

        match self
            .provider
            .delete(&req.resource_type, current_state)
            .await
        {
            Ok(()) => Ok(tonic::Response::new(crate::generated::DeleteResponse {
                diagnostics: vec![],
            })),
            Err(e) => Ok(tonic::Response::new(crate::generated::DeleteResponse {
                diagnostics: self.error_to_diagnostics(e),
            })),
        }
    }

    async fn import_resource_state(
        &self,
        request: tonic::Request<crate::generated::ImportResourceStateRequest>,
    ) -> Result<tonic::Response<crate::generated::ImportResourceStateResponse>, tonic::Status> {
        let req = request.into_inner();

        match self
            .provider
            .import_resource(&req.resource_type, &req.id)
            .await
        {
            Ok(imported) => Ok(tonic::Response::new(
                crate::generated::ImportResourceStateResponse {
                    imported: imported
                        .into_iter()
                        .map(|r| crate::generated::ImportedResource {
                            resource_type: r.resource_type,
                            state: serde_json::to_vec(&r.state).unwrap_or_default(),
                        })
                        .collect(),
                    diagnostics: vec![],
                },
            )),
            Err(e) => Ok(tonic::Response::new(
                crate::generated::ImportResourceStateResponse {
                    imported: vec![],
                    diagnostics: self.error_to_diagnostics(e),
                },
            )),
        }
    }

    async fn validate_data_source_config(
        &self,
        request: tonic::Request<crate::generated::ValidateDataSourceConfigRequest>,
    ) -> Result<tonic::Response<crate::generated::ValidateDataSourceConfigResponse>, tonic::Status>
    {
        let req = request.into_inner();
        let config = serde_json::from_slice(&req.config).unwrap_or(serde_json::Value::Null);

        match self
            .provider
            .validate_data_source_config(&req.data_source_type, config)
            .await
        {
            Ok(diagnostics) => Ok(tonic::Response::new(
                crate::generated::ValidateDataSourceConfigResponse {
                    diagnostics: self.diagnostics_to_proto(diagnostics),
                },
            )),
            Err(e) => Ok(tonic::Response::new(
                crate::generated::ValidateDataSourceConfigResponse {
                    diagnostics: self.error_to_diagnostics(e),
                },
            )),
        }
    }

    async fn read_data_source(
        &self,
        request: tonic::Request<crate::generated::ReadDataSourceRequest>,
    ) -> Result<tonic::Response<crate::generated::ReadDataSourceResponse>, tonic::Status> {
        let req = request.into_inner();
        let config = serde_json::from_slice(&req.config).unwrap_or(serde_json::Value::Null);

        match self
            .provider
            .read_data_source(&req.data_source_type, config)
            .await
        {
            Ok(state) => Ok(tonic::Response::new(
                crate::generated::ReadDataSourceResponse {
                    state: serde_json::to_vec(&state).unwrap_or_default(),
                    diagnostics: vec![],
                },
            )),
            Err(e) => Ok(tonic::Response::new(
                crate::generated::ReadDataSourceResponse {
                    state: vec![],
                    diagnostics: self.error_to_diagnostics(e),
                },
            )),
        }
    }
}

/// Options for configuring the provider server.
#[derive(Debug, Clone)]
pub struct ServeOptions {
    /// Timeout for graceful shutdown. After receiving a shutdown signal,
    /// the server will wait this long for in-flight requests to complete.
    /// Default: 30 seconds.
    pub shutdown_timeout: Duration,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

impl ServeOptions {
    /// Create new serve options with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the shutdown timeout.
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }
}

/// Wait for a shutdown signal (SIGTERM or SIGINT).
///
/// On Unix, this waits for SIGTERM or SIGINT.
/// On Windows, this waits for CTRL+C.
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt())
            .expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                eprintln!("Received SIGTERM, initiating graceful shutdown...");
            }
            _ = sigint.recv() => {
                eprintln!("Received SIGINT, initiating graceful shutdown...");
            }
        }
    }

    #[cfg(windows)]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        eprintln!("Received CTRL+C, initiating graceful shutdown...");
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Fallback: just wait forever (no signal handling)
        std::future::pending::<()>().await;
    }
}

/// Serve a provider implementation as a gRPC server.
///
/// This function:
/// 1. Finds an available port
/// 2. Starts the gRPC server
/// 3. Outputs the handshake string to stdout
/// 4. Handles shutdown signals (SIGTERM/SIGINT) gracefully
///
/// The handshake format is: `HEMMER_PROVIDER|<version>|<address>`
///
/// For custom configuration, use [`serve_with_options`].
pub async fn serve<P: ProviderService>(provider: P) -> Result<(), Box<dyn std::error::Error>> {
    serve_with_options(provider, ServeOptions::default()).await
}

/// Serve a provider with custom options.
///
/// See [`serve`] for details. This function allows configuring
/// shutdown behavior via [`ServeOptions`].
pub async fn serve_with_options<P: ProviderService>(
    provider: P,
    options: ServeOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find an available port by binding to port 0
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    serve_on_listener(provider, listener, addr, options).await
}

/// Serve a provider on a specific address.
///
/// Unlike [`serve`], this function binds to the specified address rather than
/// finding an available port.
pub async fn serve_on<P: ProviderService>(
    provider: P,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    serve_on_with_options(provider, addr, ServeOptions::default()).await
}

/// Serve a provider on a specific address with custom options.
pub async fn serve_on_with_options<P: ProviderService>(
    provider: P,
    addr: SocketAddr,
    options: ServeOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    let actual_addr = listener.local_addr()?;

    serve_on_listener(provider, listener, actual_addr, options).await
}

/// Internal function to serve on an already-bound listener.
async fn serve_on_listener<P: ProviderService>(
    provider: P,
    listener: TcpListener,
    addr: SocketAddr,
    _options: ServeOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    // Output the handshake
    println!("{}|{}|{}", HANDSHAKE_PREFIX, PROTOCOL_VERSION, addr);

    // Create the gRPC service
    let grpc_service = ProviderGrpcService { provider };
    let server = crate::generated::provider_server::ProviderServer::new(grpc_service);

    // Run the server with graceful shutdown
    // Note: tonic's serve_with_incoming_shutdown handles graceful shutdown internally.
    // The shutdown_timeout in options is reserved for future use when we need
    // more control over the shutdown process.
    Server::builder()
        .add_service(server)
        .serve_with_incoming_shutdown(
            tokio_stream::wrappers::TcpListenerStream::new(listener),
            async {
                wait_for_shutdown_signal().await;
            },
        )
        .await?;

    eprintln!("Server shutdown complete");
    Ok(())
}
