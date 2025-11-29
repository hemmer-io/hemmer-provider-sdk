//! Error types for the Hemmer Provider SDK.

use thiserror::Error;

/// Errors that can occur when implementing a provider.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// The requested resource was not found.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// A validation error occurred.
    #[error("Validation error: {0}")]
    Validation(String),

    /// An internal SDK error occurred.
    #[error("SDK error: {0}")]
    Sdk(String),

    /// A configuration error occurred.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// The requested resource type is unknown.
    #[error("Unknown resource type: {0}")]
    UnknownResource(String),

    /// A serialization/deserialization error occurred.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A gRPC transport error occurred.
    #[error("Transport error: {0}")]
    Transport(#[from] tonic::transport::Error),
}

impl From<ProviderError> for tonic::Status {
    fn from(err: ProviderError) -> Self {
        match err {
            ProviderError::NotFound(msg) => tonic::Status::not_found(msg),
            ProviderError::Validation(msg) => tonic::Status::invalid_argument(msg),
            ProviderError::Configuration(msg) => tonic::Status::failed_precondition(msg),
            ProviderError::UnknownResource(msg) => tonic::Status::not_found(msg),
            ProviderError::Sdk(msg) => tonic::Status::internal(msg),
            ProviderError::Serialization(err) => {
                tonic::Status::invalid_argument(format!("Serialization error: {}", err))
            }
            ProviderError::Transport(err) => {
                tonic::Status::unavailable(format!("Transport error: {}", err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ProviderError::NotFound("resource-123".to_string());
        assert_eq!(format!("{}", err), "Resource not found: resource-123");

        let err = ProviderError::Validation("invalid input".to_string());
        assert_eq!(format!("{}", err), "Validation error: invalid input");

        let err = ProviderError::UnknownResource("custom_resource".to_string());
        assert_eq!(format!("{}", err), "Unknown resource type: custom_resource");
    }

    #[test]
    fn test_error_to_status() {
        let err = ProviderError::NotFound("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);

        let err = ProviderError::Validation("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);

        let err = ProviderError::Configuration("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);

        let err = ProviderError::Sdk("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::Internal);
    }
}
