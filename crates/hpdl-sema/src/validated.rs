//! Validated definition wrapper and metadata.

use hpdl_model::PlanetDefinition;

/// Metadata about the validation process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMetadata {
    /// Version of the validator that produced this validation.
    pub validator_version: String,
    /// List of validation rules that were applied.
    pub applied_rules: Vec<String>,
}

impl ValidationMetadata {
    /// Create new validation metadata with the current validator version.
    pub fn new() -> Self {
        Self {
            validator_version: env!("CARGO_PKG_VERSION").to_string(),
            applied_rules: vec![
                "type_checking".to_string(),
                "constraint_validation".to_string(),
                "unit_validation".to_string(),
                "status_validation".to_string(),
                "action_validation".to_string(),
                "module_validation".to_string(),
            ],
        }
    }

    /// Create validation metadata with a specific validator version.
    pub fn with_version(version: String) -> Self {
        let mut metadata = Self::new();
        metadata.validator_version = version;
        metadata
    }
}

impl Default for ValidationMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// A planet definition that has been successfully validated.
///
/// This wrapper ensures that consumers can distinguish between validated
/// and unvalidated definitions. Once created, it provides immutable access
/// to the validated definition and its metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDefinition {
    /// The validated planet definition.
    definition: PlanetDefinition,
    /// Metadata about the validation process.
    metadata: ValidationMetadata,
}

impl ValidatedDefinition {
    /// Create a new validated definition.
    ///
    /// This should only be called by the validator after successful validation.
    pub(crate) fn new(definition: PlanetDefinition, metadata: ValidationMetadata) -> Self {
        Self {
            definition,
            metadata,
        }
    }

    /// Get a reference to the validated planet definition.
    pub fn definition(&self) -> &PlanetDefinition {
        &self.definition
    }

    /// Get a reference to the validation metadata.
    pub fn metadata(&self) -> &ValidationMetadata {
        &self.metadata
    }

    /// Consume this wrapper and return the inner planet definition.
    ///
    /// Use this when you need ownership of the definition for further processing.
    pub fn into_inner(self) -> PlanetDefinition {
        self.definition
    }

    /// Consume this wrapper and return both the definition and metadata.
    pub fn into_parts(self) -> (PlanetDefinition, ValidationMetadata) {
        (self.definition, self.metadata)
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
    fn test_validation_metadata_new() {
        let metadata = ValidationMetadata::new();
        assert_eq!(metadata.validator_version, env!("CARGO_PKG_VERSION"));
        assert!(!metadata.applied_rules.is_empty());
        assert!(metadata
            .applied_rules
            .contains(&"type_checking".to_string()));
    }

    #[test]
    fn test_validation_metadata_with_version() {
        let metadata = ValidationMetadata::with_version("1.0.0".to_string());
        assert_eq!(metadata.validator_version, "1.0.0");
    }

    #[test]
    fn test_validated_definition_creation() {
        let definition = minimal_definition();
        let metadata = ValidationMetadata::new();
        let validated = ValidatedDefinition::new(definition.clone(), metadata.clone());

        assert_eq!(validated.definition(), &definition);
        assert_eq!(validated.metadata(), &metadata);
    }

    #[test]
    fn test_validated_definition_into_inner() {
        let definition = minimal_definition();
        let metadata = ValidationMetadata::new();
        let validated = ValidatedDefinition::new(definition.clone(), metadata);

        let inner = validated.into_inner();
        assert_eq!(inner, definition);
    }

    #[test]
    fn test_validated_definition_into_parts() {
        let definition = minimal_definition();
        let metadata = ValidationMetadata::new();
        let validated = ValidatedDefinition::new(definition.clone(), metadata.clone());

        let (def, meta) = validated.into_parts();
        assert_eq!(def, definition);
        assert_eq!(meta, metadata);
    }
}
