//! Testing utilities for provider implementations.
//!
//! This module provides utilities to test `ProviderService` implementations
//! without spinning up a full gRPC server.
//!
//! # Example
//!
//! ```ignore
//! use hemmer_provider_sdk::testing::ProviderTester;
//! use serde_json::json;
//!
//! #[tokio::test]
//! async fn test_create_resource() {
//!     let tester = ProviderTester::new(MyProvider::new());
//!
//!     // Configure the provider
//!     tester.configure(json!({"api_key": "test"})).await.unwrap();
//!
//!     // Test create
//!     let state = tester.create("my_resource", json!({
//!         "name": "test-resource"
//!     })).await.unwrap();
//!
//!     assert_eq!(state["name"], "test-resource");
//! }
//! ```

use crate::error::ProviderError;
use crate::schema::{Diagnostic, DiagnosticSeverity, ProviderSchema};
use crate::server::ProviderService;
use crate::types::{ImportedResource, PlanResult};
use serde_json::Value;

/// A test harness for provider implementations.
///
/// This wraps a `ProviderService` implementation and provides
/// simplified methods for testing without a gRPC server.
///
/// # Example
///
/// ```ignore
/// use hemmer_provider_sdk::testing::ProviderTester;
///
/// let tester = ProviderTester::new(MyProvider::new());
/// tester.configure(json!({})).await.unwrap();
/// let state = tester.create("my_resource", json!({"name": "test"})).await.unwrap();
/// ```
pub struct ProviderTester<P: ProviderService> {
    provider: P,
}

impl<P: ProviderService> ProviderTester<P> {
    /// Create a new tester for the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Get a reference to the underlying provider.
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Get a mutable reference to the underlying provider.
    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    // =========================================================================
    // Schema & Metadata
    // =========================================================================

    /// Get the provider's schema.
    pub fn schema(&self) -> ProviderSchema {
        self.provider.schema()
    }

    /// Get the list of resource type names.
    pub fn resource_types(&self) -> Vec<String> {
        self.provider.metadata().resources
    }

    /// Get the list of data source type names.
    pub fn data_source_types(&self) -> Vec<String> {
        self.provider.metadata().data_sources
    }

    // =========================================================================
    // Provider Lifecycle
    // =========================================================================

    /// Validate provider configuration.
    ///
    /// Returns `Ok(())` if validation passes (no error diagnostics).
    /// Returns `Err` with the diagnostics if there are errors.
    pub async fn validate_provider_config(&self, config: Value) -> Result<(), TestError> {
        let diagnostics = self.provider.validate_provider_config(config).await?;
        check_diagnostics(diagnostics)
    }

    /// Configure the provider.
    ///
    /// Returns `Ok(())` if configuration succeeds.
    /// Returns `Err` with the diagnostics if there are errors.
    pub async fn configure(&self, config: Value) -> Result<(), TestError> {
        let diagnostics = self.provider.configure(config).await?;
        check_diagnostics(diagnostics)
    }

    /// Stop the provider.
    pub async fn stop(&self) -> Result<(), ProviderError> {
        self.provider.stop().await
    }

    // =========================================================================
    // Resource Operations
    // =========================================================================

    /// Validate a resource configuration.
    pub async fn validate_resource_config(
        &self,
        resource_type: &str,
        config: Value,
    ) -> Result<(), TestError> {
        let diagnostics = self
            .provider
            .validate_resource_config(resource_type, config)
            .await?;
        check_diagnostics(diagnostics)
    }

    /// Plan a resource creation (no prior state).
    pub async fn plan_create(
        &self,
        resource_type: &str,
        proposed_state: Value,
    ) -> Result<PlanResult, ProviderError> {
        self.provider
            .plan(resource_type, None, proposed_state.clone(), proposed_state)
            .await
    }

    /// Plan a resource update.
    pub async fn plan_update(
        &self,
        resource_type: &str,
        prior_state: Value,
        proposed_state: Value,
    ) -> Result<PlanResult, ProviderError> {
        self.provider
            .plan(
                resource_type,
                Some(prior_state),
                proposed_state.clone(),
                proposed_state,
            )
            .await
    }

    /// Plan a resource deletion.
    pub async fn plan_delete(
        &self,
        resource_type: &str,
        prior_state: Value,
    ) -> Result<PlanResult, ProviderError> {
        self.provider
            .plan(resource_type, Some(prior_state), Value::Null, Value::Null)
            .await
    }

    /// Full plan operation with explicit config.
    pub async fn plan(
        &self,
        resource_type: &str,
        prior_state: Option<Value>,
        proposed_state: Value,
        config: Value,
    ) -> Result<PlanResult, ProviderError> {
        self.provider
            .plan(resource_type, prior_state, proposed_state, config)
            .await
    }

    /// Create a new resource.
    pub async fn create(
        &self,
        resource_type: &str,
        planned_state: Value,
    ) -> Result<Value, ProviderError> {
        self.provider.create(resource_type, planned_state).await
    }

    /// Read the current state of a resource.
    pub async fn read(
        &self,
        resource_type: &str,
        current_state: Value,
    ) -> Result<Value, ProviderError> {
        self.provider.read(resource_type, current_state).await
    }

    /// Update an existing resource.
    pub async fn update(
        &self,
        resource_type: &str,
        prior_state: Value,
        planned_state: Value,
    ) -> Result<Value, ProviderError> {
        self.provider
            .update(resource_type, prior_state, planned_state)
            .await
    }

    /// Delete a resource.
    pub async fn delete(
        &self,
        resource_type: &str,
        current_state: Value,
    ) -> Result<(), ProviderError> {
        self.provider.delete(resource_type, current_state).await
    }

    /// Import an existing resource.
    pub async fn import_resource(
        &self,
        resource_type: &str,
        id: &str,
    ) -> Result<Vec<ImportedResource>, ProviderError> {
        self.provider.import_resource(resource_type, id).await
    }

    /// Upgrade resource state from an older schema version.
    pub async fn upgrade_resource_state(
        &self,
        resource_type: &str,
        version: i64,
        state: Value,
    ) -> Result<Value, ProviderError> {
        self.provider
            .upgrade_resource_state(resource_type, version, state)
            .await
    }

    // =========================================================================
    // Data Source Operations
    // =========================================================================

    /// Validate a data source configuration.
    pub async fn validate_data_source_config(
        &self,
        data_source_type: &str,
        config: Value,
    ) -> Result<(), TestError> {
        let diagnostics = self
            .provider
            .validate_data_source_config(data_source_type, config)
            .await?;
        check_diagnostics(diagnostics)
    }

    /// Read data from a data source.
    pub async fn read_data_source(
        &self,
        data_source_type: &str,
        config: Value,
    ) -> Result<Value, ProviderError> {
        self.provider
            .read_data_source(data_source_type, config)
            .await
    }

    // =========================================================================
    // Lifecycle Helpers
    // =========================================================================

    /// Run a full create lifecycle: plan → create → read.
    ///
    /// Returns the final state after read.
    pub async fn lifecycle_create(
        &self,
        resource_type: &str,
        config: Value,
    ) -> Result<Value, ProviderError> {
        // Plan
        let plan_result = self.plan_create(resource_type, config).await?;

        // Create
        let created_state = self
            .create(resource_type, plan_result.planned_state)
            .await?;

        // Read to verify
        self.read(resource_type, created_state).await
    }

    /// Run a full update lifecycle: plan → update → read.
    ///
    /// Returns the final state after read.
    pub async fn lifecycle_update(
        &self,
        resource_type: &str,
        prior_state: Value,
        proposed_state: Value,
    ) -> Result<Value, ProviderError> {
        // Plan
        let plan_result = self
            .plan_update(resource_type, prior_state.clone(), proposed_state)
            .await?;

        // Update
        let updated_state = self
            .update(resource_type, prior_state, plan_result.planned_state)
            .await?;

        // Read to verify
        self.read(resource_type, updated_state).await
    }

    /// Run a full delete lifecycle: plan → delete.
    pub async fn lifecycle_delete(
        &self,
        resource_type: &str,
        current_state: Value,
    ) -> Result<(), ProviderError> {
        // Plan (optional, but good practice)
        let _ = self
            .plan_delete(resource_type, current_state.clone())
            .await?;

        // Delete
        self.delete(resource_type, current_state).await
    }

    /// Run a full CRUD lifecycle: create → read → update → read → delete.
    ///
    /// Returns the state after the update (before delete).
    pub async fn lifecycle_crud(
        &self,
        resource_type: &str,
        initial_config: Value,
        updated_config: Value,
    ) -> Result<Value, ProviderError> {
        // Create
        let created_state = self.lifecycle_create(resource_type, initial_config).await?;

        // Update
        let updated_state = self
            .lifecycle_update(resource_type, created_state.clone(), updated_config)
            .await?;

        // Delete
        self.lifecycle_delete(resource_type, updated_state.clone())
            .await?;

        Ok(updated_state)
    }
}

/// Error type for test operations that may fail with diagnostics.
#[derive(Debug)]
pub enum TestError {
    /// The operation failed with diagnostics.
    Diagnostics(Vec<Diagnostic>),
    /// The operation failed with a provider error.
    Provider(ProviderError),
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::Diagnostics(diags) => {
                writeln!(f, "Operation failed with {} diagnostic(s):", diags.len())?;
                for diag in diags {
                    write!(f, "  [{:?}] {}", diag.severity, diag.summary)?;
                    if let Some(detail) = &diag.detail {
                        write!(f, ": {}", detail)?;
                    }
                    if let Some(attr) = &diag.attribute {
                        write!(f, " (at {})", attr)?;
                    }
                    writeln!(f)?;
                }
                Ok(())
            }
            TestError::Provider(e) => write!(f, "Provider error: {}", e),
        }
    }
}

impl std::error::Error for TestError {}

impl From<ProviderError> for TestError {
    fn from(e: ProviderError) -> Self {
        TestError::Provider(e)
    }
}

/// Check diagnostics and return an error if there are any errors.
fn check_diagnostics(diagnostics: Vec<Diagnostic>) -> Result<(), TestError> {
    let errors: Vec<_> = diagnostics
        .into_iter()
        .filter(|d| matches!(d.severity, DiagnosticSeverity::Error))
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(TestError::Diagnostics(errors))
    }
}

// =========================================================================
// Assertion Helpers
// =========================================================================

/// Assert that a plan result indicates the resource will be created.
///
/// # Panics
///
/// Panics if the plan has no changes or requires replacement.
pub fn assert_plan_creates(plan: &PlanResult) {
    assert!(
        !plan.changes.is_empty(),
        "Expected plan to have changes for create, but got no changes"
    );
    assert!(
        !plan.requires_replace,
        "Expected plan to create, not replace"
    );
}

/// Assert that a plan result indicates no changes.
///
/// # Panics
///
/// Panics if the plan has any changes.
pub fn assert_plan_no_changes(plan: &PlanResult) {
    assert!(
        plan.changes.is_empty(),
        "Expected no changes, but got {} change(s): {:?}",
        plan.changes.len(),
        plan.changes.iter().map(|c| &c.path).collect::<Vec<_>>()
    );
}

/// Assert that a plan result indicates changes are needed.
///
/// # Panics
///
/// Panics if the plan has no changes.
pub fn assert_plan_has_changes(plan: &PlanResult) {
    assert!(
        !plan.changes.is_empty(),
        "Expected plan to have changes, but got no changes"
    );
}

/// Assert that a plan requires resource replacement.
///
/// # Panics
///
/// Panics if the plan does not require replacement.
pub fn assert_plan_replaces(plan: &PlanResult) {
    assert!(
        plan.requires_replace,
        "Expected plan to require replacement, but it does not"
    );
}

/// Assert that a plan does not require resource replacement.
///
/// # Panics
///
/// Panics if the plan requires replacement.
pub fn assert_plan_updates_in_place(plan: &PlanResult) {
    assert!(
        !plan.requires_replace,
        "Expected plan to update in place, but it requires replacement"
    );
}

/// Assert that a plan has a change for a specific attribute path.
///
/// # Panics
///
/// Panics if the plan does not have a change for the given path.
pub fn assert_plan_changes_attribute(plan: &PlanResult, path: &str) {
    let has_change = plan.changes.iter().any(|c| c.path == path);
    assert!(
        has_change,
        "Expected plan to change attribute '{}', but it was not changed. Changed attributes: {:?}",
        path,
        plan.changes.iter().map(|c| &c.path).collect::<Vec<_>>()
    );
}

/// Assert that a plan does not have a change for a specific attribute path.
///
/// # Panics
///
/// Panics if the plan has a change for the given path.
pub fn assert_plan_does_not_change_attribute(plan: &PlanResult, path: &str) {
    let has_change = plan.changes.iter().any(|c| c.path == path);
    assert!(
        !has_change,
        "Expected plan to not change attribute '{}', but it was changed",
        path
    );
}

/// Assert that diagnostics contain no errors.
///
/// # Panics
///
/// Panics if there are any error diagnostics.
pub fn assert_no_errors(diagnostics: &[Diagnostic]) {
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, DiagnosticSeverity::Error))
        .collect();

    assert!(
        errors.is_empty(),
        "Expected no errors, but got {} error(s): {:?}",
        errors.len(),
        errors.iter().map(|d| &d.summary).collect::<Vec<_>>()
    );
}

/// Assert that diagnostics contain at least one error.
///
/// # Panics
///
/// Panics if there are no error diagnostics.
pub fn assert_has_errors(diagnostics: &[Diagnostic]) {
    let has_errors = diagnostics
        .iter()
        .any(|d| matches!(d.severity, DiagnosticSeverity::Error));

    assert!(has_errors, "Expected at least one error, but got none");
}

/// Assert that diagnostics contain an error with the given summary substring.
///
/// # Panics
///
/// Panics if no error diagnostic contains the given substring.
pub fn assert_error_contains(diagnostics: &[Diagnostic], substring: &str) {
    let has_matching_error = diagnostics
        .iter()
        .any(|d| matches!(d.severity, DiagnosticSeverity::Error) && d.summary.contains(substring));

    assert!(
        has_matching_error,
        "Expected an error containing '{}', but no matching error found. Errors: {:?}",
        substring,
        diagnostics
            .iter()
            .filter(|d| matches!(d.severity, DiagnosticSeverity::Error))
            .map(|d| &d.summary)
            .collect::<Vec<_>>()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Attribute, Schema};
    use crate::types::AttributeChange;
    use serde_json::json;

    // A simple test provider for testing the tester
    struct TestProvider;

    #[async_trait::async_trait]
    impl ProviderService for TestProvider {
        fn schema(&self) -> ProviderSchema {
            ProviderSchema::new()
                .with_provider_config(
                    Schema::v0().with_attribute("api_key", Attribute::optional_string()),
                )
                .with_resource(
                    "test_resource",
                    Schema::v0()
                        .with_attribute("name", Attribute::required_string())
                        .with_attribute("id", Attribute::computed_string()),
                )
        }

        async fn configure(&self, _config: Value) -> Result<Vec<Diagnostic>, ProviderError> {
            Ok(vec![])
        }

        async fn plan(
            &self,
            _resource_type: &str,
            prior_state: Option<Value>,
            proposed_state: Value,
            _config: Value,
        ) -> Result<PlanResult, ProviderError> {
            match prior_state {
                None => {
                    // Create
                    let mut planned = proposed_state.clone();
                    if let Value::Object(ref mut map) = planned {
                        map.insert("id".to_string(), json!("generated-id"));
                    }
                    Ok(PlanResult::with_changes(
                        planned,
                        vec![AttributeChange::added("id", json!("generated-id"))],
                        false,
                    ))
                }
                Some(prior) => {
                    // Update - check if name changed
                    if prior.get("name") != proposed_state.get("name") {
                        let mut planned = proposed_state.clone();
                        if let Value::Object(ref mut map) = planned {
                            map.insert("id".to_string(), prior["id"].clone());
                        }
                        Ok(PlanResult::with_changes(
                            planned,
                            vec![AttributeChange::modified(
                                "name",
                                prior["name"].clone(),
                                proposed_state["name"].clone(),
                            )],
                            false,
                        ))
                    } else {
                        Ok(PlanResult::no_change(prior))
                    }
                }
            }
        }

        async fn create(
            &self,
            _resource_type: &str,
            planned_state: Value,
        ) -> Result<Value, ProviderError> {
            Ok(planned_state)
        }

        async fn read(
            &self,
            _resource_type: &str,
            current_state: Value,
        ) -> Result<Value, ProviderError> {
            Ok(current_state)
        }

        async fn update(
            &self,
            _resource_type: &str,
            _prior_state: Value,
            planned_state: Value,
        ) -> Result<Value, ProviderError> {
            Ok(planned_state)
        }

        async fn delete(
            &self,
            _resource_type: &str,
            _current_state: Value,
        ) -> Result<(), ProviderError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_tester_configure() {
        let tester = ProviderTester::new(TestProvider);
        let result = tester.configure(json!({"api_key": "test"})).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tester_schema() {
        let tester = ProviderTester::new(TestProvider);
        let schema = tester.schema();
        assert!(schema.resources.contains_key("test_resource"));
    }

    #[tokio::test]
    async fn test_tester_resource_types() {
        let tester = ProviderTester::new(TestProvider);
        let types = tester.resource_types();
        assert!(types.contains(&"test_resource".to_string()));
    }

    #[tokio::test]
    async fn test_tester_plan_create() {
        let tester = ProviderTester::new(TestProvider);
        let plan = tester
            .plan_create("test_resource", json!({"name": "test"}))
            .await
            .unwrap();

        assert_plan_creates(&plan);
        assert_eq!(plan.planned_state["id"], "generated-id");
    }

    #[tokio::test]
    async fn test_tester_plan_update_with_changes() {
        let tester = ProviderTester::new(TestProvider);
        let plan = tester
            .plan_update(
                "test_resource",
                json!({"name": "old", "id": "123"}),
                json!({"name": "new", "id": "123"}),
            )
            .await
            .unwrap();

        assert_plan_has_changes(&plan);
        assert_plan_changes_attribute(&plan, "name");
        assert_plan_updates_in_place(&plan);
    }

    #[tokio::test]
    async fn test_tester_plan_update_no_changes() {
        let tester = ProviderTester::new(TestProvider);
        let state = json!({"name": "same", "id": "123"});
        let plan = tester
            .plan_update("test_resource", state.clone(), state)
            .await
            .unwrap();

        assert_plan_no_changes(&plan);
    }

    #[tokio::test]
    async fn test_tester_lifecycle_create() {
        let tester = ProviderTester::new(TestProvider);
        let state = tester
            .lifecycle_create("test_resource", json!({"name": "test"}))
            .await
            .unwrap();

        assert_eq!(state["name"], "test");
        assert_eq!(state["id"], "generated-id");
    }

    #[tokio::test]
    async fn test_tester_lifecycle_crud() {
        let tester = ProviderTester::new(TestProvider);
        let final_state = tester
            .lifecycle_crud(
                "test_resource",
                json!({"name": "initial"}),
                json!({"name": "updated"}),
            )
            .await
            .unwrap();

        assert_eq!(final_state["name"], "updated");
    }

    #[test]
    fn test_assert_no_errors() {
        let diagnostics = vec![Diagnostic::warning("Just a warning")];
        assert_no_errors(&diagnostics);
    }

    #[test]
    #[should_panic(expected = "Expected no errors")]
    fn test_assert_no_errors_fails() {
        let diagnostics = vec![Diagnostic::error("An error")];
        assert_no_errors(&diagnostics);
    }

    #[test]
    fn test_assert_has_errors() {
        let diagnostics = vec![Diagnostic::error("An error")];
        assert_has_errors(&diagnostics);
    }

    #[test]
    fn test_assert_error_contains() {
        let diagnostics = vec![Diagnostic::error("Invalid configuration value")];
        assert_error_contains(&diagnostics, "Invalid");
        assert_error_contains(&diagnostics, "configuration");
    }

    #[test]
    fn test_test_error_display() {
        let err = TestError::Diagnostics(vec![
            Diagnostic::error("First error").with_attribute("field1"),
            Diagnostic::error("Second error").with_detail("More info"),
        ]);

        let display = format!("{}", err);
        assert!(display.contains("First error"));
        assert!(display.contains("Second error"));
        assert!(display.contains("field1"));
        assert!(display.contains("More info"));
    }
}
