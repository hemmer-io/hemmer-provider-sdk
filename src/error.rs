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

    /// Resource already exists (create conflict).
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Permission denied (authentication/authorization failure).
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Quota or rate limit exceeded.
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Service temporarily unavailable.
    #[error("Service unavailable: {0}")]
    Unavailable(String),

    /// Operation timed out.
    #[error("Deadline exceeded: {0}")]
    DeadlineExceeded(String),

    /// Operation failed due to current state (precondition not met).
    #[error("Failed precondition: {0}")]
    FailedPrecondition(String),

    /// Operation not implemented.
    #[error("Unimplemented: {0}")]
    Unimplemented(String),
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
            ProviderError::AlreadyExists(msg) => tonic::Status::already_exists(msg),
            ProviderError::PermissionDenied(msg) => tonic::Status::permission_denied(msg),
            ProviderError::ResourceExhausted(msg) => tonic::Status::resource_exhausted(msg),
            ProviderError::Unavailable(msg) => tonic::Status::unavailable(msg),
            ProviderError::DeadlineExceeded(msg) => tonic::Status::deadline_exceeded(msg),
            ProviderError::FailedPrecondition(msg) => tonic::Status::failed_precondition(msg),
            ProviderError::Unimplemented(msg) => tonic::Status::unimplemented(msg),
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

    #[test]
    fn test_new_error_variants_display() {
        let err = ProviderError::AlreadyExists("bucket-123".to_string());
        assert_eq!(format!("{}", err), "Resource already exists: bucket-123");

        let err = ProviderError::PermissionDenied("access forbidden".to_string());
        assert_eq!(format!("{}", err), "Permission denied: access forbidden");

        let err = ProviderError::ResourceExhausted("quota exceeded".to_string());
        assert_eq!(format!("{}", err), "Resource exhausted: quota exceeded");

        let err = ProviderError::Unavailable("service down".to_string());
        assert_eq!(format!("{}", err), "Service unavailable: service down");

        let err = ProviderError::DeadlineExceeded("timeout".to_string());
        assert_eq!(format!("{}", err), "Deadline exceeded: timeout");

        let err = ProviderError::FailedPrecondition("state mismatch".to_string());
        assert_eq!(format!("{}", err), "Failed precondition: state mismatch");

        let err = ProviderError::Unimplemented("feature not available".to_string());
        assert_eq!(format!("{}", err), "Unimplemented: feature not available");
    }

    #[test]
    fn test_new_error_variants_to_status() {
        let err = ProviderError::AlreadyExists("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::AlreadyExists);

        let err = ProviderError::PermissionDenied("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::PermissionDenied);

        let err = ProviderError::ResourceExhausted("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::ResourceExhausted);

        let err = ProviderError::Unavailable("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);

        let err = ProviderError::DeadlineExceeded("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::DeadlineExceeded);

        let err = ProviderError::FailedPrecondition("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);

        let err = ProviderError::Unimplemented("test".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }
}
