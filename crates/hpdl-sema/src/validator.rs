//! Main validator orchestrating all validation passes.

use crate::action_validator::ActionValidator;
use crate::constraint_solver::ConstraintSolver;
use crate::environment_validator::EnvironmentValidator;
use crate::module_validator::ModuleValidator;
use crate::orbital_validator::OrbitalValidator;
use crate::resolver::ModuleResolver;
use crate::status_validator::StatusValidator;
use crate::type_checker::TypeChecker;
use crate::unit_registry::UnitRegistry;
use crate::validated::{ValidatedDefinition, ValidationMetadata};
use crate::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
use hpdl_model::PlanetDefinition;

/// Configuration for the validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorConfig {
    /// Maximum nesting depth for data structures.
    pub max_nesting_depth: usize,
    /// Maximum collection size (number of elements).
    pub max_collection_size: usize,
    /// Maximum string length in bytes.
    pub max_string_length: usize,
    /// Enable strict validation mode (treat warnings as errors).
    pub strict_mode: bool,
}

impl ValidatorConfig {
    /// Default maximum nesting depth.
    pub const DEFAULT_MAX_NESTING_DEPTH: usize = 32;
    /// Default maximum collection size.
    pub const DEFAULT_MAX_COLLECTION_SIZE: usize = 10_000;
    /// Default maximum string length.
    pub const DEFAULT_MAX_STRING_LENGTH: usize = 1_000_000;

    /// Create a new validator configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict validator configuration.
    pub fn strict() -> Self {
        Self {
            strict_mode: true,
            ..Self::default()
        }
    }

    /// Set the maximum nesting depth.
    #[must_use]
    pub fn with_max_nesting_depth(mut self, depth: usize) -> Self {
        self.max_nesting_depth = depth;
        self
    }

    /// Set the maximum collection size.
    #[must_use]
    pub fn with_max_collection_size(mut self, size: usize) -> Self {
        self.max_collection_size = size;
        self
    }

    /// Set the maximum string length.
    #[must_use]
    pub fn with_max_string_length(mut self, length: usize) -> Self {
        self.max_string_length = length;
        self
    }

    /// Enable strict mode.
    #[must_use]
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            max_nesting_depth: Self::DEFAULT_MAX_NESTING_DEPTH,
            max_collection_size: Self::DEFAULT_MAX_COLLECTION_SIZE,
            max_string_length: Self::DEFAULT_MAX_STRING_LENGTH,
            strict_mode: false,
        }
    }
}

/// Main validator coordinating all validation passes.
pub struct Validator {
    config: ValidatorConfig,
    type_checker: TypeChecker,
    constraint_solver: ConstraintSolver,
    unit_registry: UnitRegistry,
    status_validator: StatusValidator,
    action_validator: ActionValidator,
    module_validator: ModuleValidator,
    orbital_validator: OrbitalValidator,
    environment_validator: EnvironmentValidator,
    /// Optional module registry the module/status/environment passes consult to
    /// back declared module references (ADR 0011). `None` = no registry: every
    /// module reference is flagged non-executable, exactly as before the registry
    /// existed.
    resolver: Option<Box<dyn ModuleResolver>>,
}

impl Validator {
    /// Create a new validator with the given configuration and no module
    /// registry.
    pub fn new(config: ValidatorConfig) -> Self {
        Self {
            type_checker: TypeChecker::new(config.strict_mode),
            constraint_solver: ConstraintSolver::default(),
            unit_registry: UnitRegistry::default(),
            status_validator: StatusValidator,
            action_validator: ActionValidator,
            module_validator: ModuleValidator,
            orbital_validator: OrbitalValidator,
            environment_validator: EnvironmentValidator,
            resolver: None,
            config,
        }
    }

    /// Create a validator with default configuration and no module registry.
    pub fn default_config() -> Self {
        Self::new(ValidatorConfig::default())
    }

    /// Attach a module registry (a [`ModuleResolver`]) that the module, status,
    /// and environment passes consult to resolve declared module references
    /// against registered implementations (existence, kind, integrity — ADR
    /// 0011). Without this, module references are flagged non-executable.
    #[must_use]
    pub fn with_resolver(mut self, resolver: Box<dyn ModuleResolver>) -> Self {
        self.resolver = Some(resolver);
        self
    }

    /// Validate a planet definition.
    ///
    /// Returns `Ok(ValidatedDefinition)` if validation succeeds,
    /// or `Err(Vec<Diagnostic>)` with all collected diagnostics if validation fails.
    ///
    /// # Errors
    ///
    /// Returns a vector of diagnostics if validation fails. At least one diagnostic
    /// will have `Severity::Error`.
    pub fn validate(
        &self,
        definition: PlanetDefinition,
    ) -> Result<ValidatedDefinition, Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();

        // Run all validation passes
        diagnostics.extend(self.validate_structure(&definition));
        diagnostics.extend(self.validate_types(&definition));
        diagnostics.extend(self.validate_constraints(&definition));
        diagnostics.extend(self.validate_units(&definition));
        diagnostics.extend(self.validate_statuses(&definition));
        diagnostics.extend(self.validate_actions(&definition));
        diagnostics.extend(self.validate_modules(&definition));
        diagnostics.extend(self.validate_orbital(&definition));
        diagnostics.extend(self.validate_environment(&definition));

        // Sort diagnostics for deterministic output
        diagnostics.sort();

        // Check if we have any errors (or warnings in strict mode)
        let has_errors = diagnostics.iter().any(|d| {
            matches!(d.severity, Severity::Error)
                || (self.config.strict_mode && matches!(d.severity, Severity::Warning))
        });

        if has_errors {
            Err(diagnostics)
        } else {
            Ok(ValidatedDefinition::new(
                definition,
                ValidationMetadata::new(),
            ))
        }
    }

    /// Validate basic structure and security limits.
    fn validate_structure(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        // Check body ID
        if let Some(diag) = self.check_body_id(body.id()) {
            diagnostics.push(diag);
        }

        // Check collection sizes
        if let Some(diag) = self.check_collection_limit(body.migrations().len(), "migrations", None)
        {
            diagnostics.push(diag);
        }

        if let Some(diag) = self.check_collection_limit(body.statuses().len(), "statuses", None) {
            diagnostics.push(diag);
        }

        if let Some(diag) = self.check_collection_limit(body.actions().len(), "actions", None) {
            diagnostics.push(diag);
        }

        diagnostics
    }

    /// Validate field types.
    fn validate_types(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        diagnostics.extend(self.type_checker.check_field_types(definition));
        diagnostics.extend(self.type_checker.check_required_fields(definition));
        diagnostics.extend(self.type_checker.check_enum_variants(definition));
        diagnostics
    }

    /// Validate physical constraints.
    fn validate_constraints(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        diagnostics.extend(
            self.constraint_solver
                .check_physical_constraints(definition),
        );
        diagnostics.extend(
            self.constraint_solver
                .check_cross_field_consistency(definition),
        );
        diagnostics
    }

    /// Validate units and dimensions.
    fn validate_units(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();
        let properties = body.properties();

        // Validate numeric values in physical properties (fields are Option<u128>)
        if let Some(mass_kg) = properties.mass_kg {
            // Convert to f64 for numeric validation
            if let Some(diag) = self.unit_registry.validate_numeric(mass_kg as f64) {
                diagnostics.push(diag);
            }
        }

        if let Some(radius_m) = properties.radius_m {
            // Convert to f64 for numeric validation
            if let Some(diag) = self.unit_registry.validate_numeric(radius_m as f64) {
                diagnostics.push(diag);
            }
        }

        // Future: validate explicit units when unit system is added to model

        diagnostics
    }

    /// Validate status declarations.
    fn validate_statuses(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        diagnostics.extend(self.status_validator.validate_statuses(definition));
        diagnostics.extend(
            self.status_validator
                .validate_action_status_writes(definition),
        );
        diagnostics.extend(
            self.status_validator
                .validate_status_vitality(definition, self.resolver.as_deref()),
        );
        diagnostics
    }

    /// Validate action declarations (including capability/write-coverage checks).
    fn validate_actions(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = self.action_validator.validate_actions(definition);
        diagnostics.extend(self.action_validator.validate_capabilities(definition));
        diagnostics
    }

    /// Validate module references and world laws.
    fn validate_modules(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        diagnostics.extend(self.module_validator.validate_modules(definition));
        diagnostics.extend(
            self.module_validator
                .validate_world_laws(definition, self.resolver.as_deref()),
        );
        diagnostics
    }

    /// Validate orbital declarations, reference frames, and the parent graph.
    fn validate_orbital(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        self.orbital_validator.validate(definition)
    }

    /// Validate the declared planetary environment.
    fn validate_environment(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        self.environment_validator
            .validate(definition, self.resolver.as_deref())
    }

    // Helper methods

    /// Check if a body ID is valid.
    fn check_body_id(&self, id: &hpdl_model::BodyId) -> Option<Diagnostic> {
        // The BodyId type already validates on construction, so just check string length
        if let Some(diag) = self.check_string_limit(id.as_str(), "body ID", None) {
            return Some(diag);
        }

        None
    }

    /// Check if a collection exceeds the size limit.
    fn check_collection_limit(
        &self,
        size: usize,
        name: &str,
        span: Option<SourceSpan>,
    ) -> Option<Diagnostic> {
        if size > self.config.max_collection_size {
            Some(
                Diagnostic::error(
                    DiagnosticCode::MaxCollectionSizeExceeded,
                    format!(
                        "Collection '{}' size {} exceeds limit {}",
                        name, size, self.config.max_collection_size
                    ),
                )
                .with_note(format!(
                    "Consider reducing the size or increasing the limit (current: {})",
                    self.config.max_collection_size
                ))
                .with_span(span.unwrap_or_else(|| SourceSpan::new(0, 0))),
            )
        } else {
            None
        }
    }

    /// Check if a string exceeds the length limit.
    fn check_string_limit(
        &self,
        value: &str,
        name: &str,
        span: Option<SourceSpan>,
    ) -> Option<Diagnostic> {
        if value.len() > self.config.max_string_length {
            Some(
                Diagnostic::error(
                    DiagnosticCode::MaxStringLengthExceeded,
                    format!(
                        "String '{}' length {} exceeds limit {}",
                        name,
                        value.len(),
                        self.config.max_string_length
                    ),
                )
                .with_span(span.unwrap_or_else(|| SourceSpan::new(0, 0))),
            )
        } else {
            None
        }
    }

    /// Check if nesting depth exceeds the limit.
    #[allow(dead_code)]
    fn check_depth_limit(&self, depth: usize, span: Option<SourceSpan>) -> Option<Diagnostic> {
        if depth > self.config.max_nesting_depth {
            Some(
                Diagnostic::error(
                    DiagnosticCode::MaxDepthExceeded,
                    format!(
                        "Nesting depth {} exceeds limit {}",
                        depth, self.config.max_nesting_depth
                    ),
                )
                .with_span(span.unwrap_or_else(|| SourceSpan::new(0, 0))),
            )
        } else {
            None
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::BodyId;

    fn minimal_definition() -> PlanetDefinition {
        use hpdl_model::{CelestialBody, PhysicalProperties, TraitSet};

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![], // modules
            vec![], // statuses
            vec![], // actions
            vec![], // migrations
        )
        .unwrap();

        PlanetDefinition::new("1.0", body).unwrap()
    }

    #[test]
    fn test_validator_config_default() {
        let config = ValidatorConfig::default();
        assert_eq!(
            config.max_nesting_depth,
            ValidatorConfig::DEFAULT_MAX_NESTING_DEPTH
        );
        assert_eq!(
            config.max_collection_size,
            ValidatorConfig::DEFAULT_MAX_COLLECTION_SIZE
        );
        assert_eq!(
            config.max_string_length,
            ValidatorConfig::DEFAULT_MAX_STRING_LENGTH
        );
        assert!(!config.strict_mode);
    }

    #[test]
    fn test_validator_config_strict() {
        let config = ValidatorConfig::strict();
        assert!(config.strict_mode);
    }

    #[test]
    fn test_validator_config_builder() {
        let config = ValidatorConfig::new()
            .with_max_nesting_depth(16)
            .with_max_collection_size(5000)
            .with_max_string_length(500_000)
            .with_strict_mode(true);

        assert_eq!(config.max_nesting_depth, 16);
        assert_eq!(config.max_collection_size, 5000);
        assert_eq!(config.max_string_length, 500_000);
        assert!(config.strict_mode);
    }

    #[test]
    fn test_validator_creation() {
        let config = ValidatorConfig::default();
        let _validator = Validator::new(config);
    }

    #[test]
    fn test_validate_minimal_definition() {
        let validator = Validator::default_config();
        let definition = minimal_definition();

        let result = validator.validate(definition);
        assert!(result.is_ok(), "Minimal definition should validate");
    }

    #[test]
    fn test_validate_too_many_statuses() {
        use hpdl_model::{CelestialBody, PhysicalProperties, StatusDeclaration, TraitSet};

        let validator = Validator::new(ValidatorConfig::default().with_max_collection_size(5));

        // Create more statuses than allowed
        let mut statuses = Vec::new();
        for i in 0..10 {
            statuses.push(StatusDeclaration::new(format!("status_{}", i), 0, 100).unwrap());
        }

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![], // modules
            statuses,
            vec![], // actions
            vec![], // migrations
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let result = validator.validate(definition);
        assert!(result.is_err(), "Should fail with too many statuses");

        if let Err(diagnostics) = result {
            assert!(!diagnostics.is_empty());
            assert!(diagnostics
                .iter()
                .any(|d| { d.code == DiagnosticCode::MaxCollectionSizeExceeded }));
        }
    }

    #[test]
    fn test_validate_string_too_long() {
        use hpdl_model::{CelestialBody, PhysicalProperties, TraitSet};

        let validator = Validator::new(ValidatorConfig::default().with_max_string_length(10));

        // Create a body ID that's too long
        let long_id = "a".repeat(100);
        let body = CelestialBody::new(
            BodyId::new(long_id).unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![], // modules
            vec![], // statuses
            vec![], // actions
            vec![], // migrations
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let result = validator.validate(definition);
        assert!(result.is_err(), "Should fail with string too long");

        if let Err(diagnostics) = result {
            assert!(!diagnostics.is_empty());
            assert!(diagnostics
                .iter()
                .any(|d| { d.code == DiagnosticCode::MaxStringLengthExceeded }));
        }
    }

    #[test]
    fn test_validate_parent_cycle_fails() {
        use hpdl_model::{CelestialBody, ParentRef, PhysicalProperties, ReferenceFrame, TraitSet};

        let body = CelestialBody::new(
            BodyId::new("sol").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
        .with_parent(ParentRef::new(
            BodyId::new("sol").unwrap(),
            ReferenceFrame::EclipticJ2000,
        ));

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let result = Validator::default_config().validate(definition);
        assert!(result.is_err(), "self-parent should fail validation");
        if let Err(diagnostics) = result {
            assert!(diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::ParentCycle));
        }
    }

    #[test]
    fn test_diagnostics_are_sorted() {
        use hpdl_model::{
            ActionDeclaration, CelestialBody, PhysicalProperties, StatusDeclaration, TraitSet,
        };

        let validator = Validator::new(ValidatorConfig::default().with_max_collection_size(2));

        // Create multiple violations
        let mut statuses = Vec::new();
        let mut actions = Vec::new();

        for i in 0..5 {
            statuses.push(StatusDeclaration::new(format!("status_{}", i), 0, 100).unwrap());
            actions.push(
                ActionDeclaration::new(format!("action_{}", i), [] as [&str; 0], [] as [&str; 0])
                    .unwrap(),
            );
        }

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![], // modules
            statuses,
            actions,
            vec![], // migrations
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let result = validator.validate(definition);
        assert!(result.is_err());

        if let Err(diagnostics) = result {
            // Check that diagnostics are sorted
            for window in diagnostics.windows(2) {
                assert!(window[0] <= window[1], "Diagnostics should be sorted");
            }
        }
    }
}
