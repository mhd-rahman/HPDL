//! Type checking for HPDL domain model validation.

use crate::{Diagnostic, DiagnosticCode};
use hpdl_model::{CelestialBody, PlanetDefinition};

/// Type checker for validating field types and structure.
pub struct TypeChecker {
    #[allow(dead_code)]
    strict: bool,
}

impl TypeChecker {
    /// Create a new type checker.
    pub fn new(strict: bool) -> Self {
        Self { strict }
    }

    /// Check all field types in a planet definition.
    pub fn check_field_types(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        diagnostics.extend(self.check_body_structure(body));
        diagnostics.extend(self.check_properties_structure(body));
        diagnostics.extend(self.check_traits_structure(body));
        diagnostics.extend(self.check_declarations_structure(body));

        diagnostics
    }

    /// Check required fields are present.
    pub fn check_required_fields(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Language version is required (enforced by PlanetDefinition constructor)
        if definition.language_version().is_empty() {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::MissingRequiredField,
                "Language version is required".to_string(),
            ));
        }

        // Body ID is required (enforced by CelestialBody constructor)
        let body = definition.body();
        if body.id().as_str().is_empty() {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::MissingRequiredField,
                "Body ID is required".to_string(),
            ));
        }

        diagnostics
    }

    /// Check enum variants are valid.
    pub fn check_enum_variants(&self, _definition: &PlanetDefinition) -> Vec<Diagnostic> {
        // Currently no enums to validate in the domain model
        // This will be expanded when we add classification enums, etc.
        Vec::new()
    }

    // Internal validation methods

    fn check_body_structure(&self, body: &CelestialBody) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Validate body ID is well-formed (already validated by BodyId::new)
        let id_str = body.id().as_str();
        if id_str.len() > 128 {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidFieldType,
                format!("Body ID '{}' exceeds maximum length of 128", id_str),
            ));
        }

        diagnostics
    }

    fn check_properties_structure(&self, body: &CelestialBody) -> Vec<Diagnostic> {
        let diagnostics = Vec::new();
        let _properties = body.properties();

        // Validate mass if present (mass_kg is u128, always non-negative)
        // No need to check for negative values since it's unsigned

        // Validate radius if present (radius_m is u128, always non-negative)
        // No need to check for negative values since it's unsigned

        // Physical properties use deterministic integer representation,
        // so no NaN/infinity checks needed

        diagnostics
    }

    fn check_traits_structure(&self, _body: &CelestialBody) -> Vec<Diagnostic> {
        // Traits are currently just a set of strings
        // Future: validate against known trait registry
        Vec::new()
    }

    fn check_declarations_structure(&self, body: &CelestialBody) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check status declarations
        for status in body.statuses() {
            if status.name().is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidFieldType,
                    "Status name cannot be empty".to_string(),
                ));
            }
            if status.name().len() > 128 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidFieldType,
                    format!(
                        "Status name '{}' exceeds maximum length of 128",
                        status.name()
                    ),
                ));
            }
        }

        // Check action declarations
        for action in body.actions() {
            if action.name().is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidFieldType,
                    "Action name cannot be empty".to_string(),
                ));
            }
            if action.name().len() > 128 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidFieldType,
                    format!(
                        "Action name '{}' exceeds maximum length of 128",
                        action.name()
                    ),
                ));
            }
        }

        // Check migration declarations
        for migration in body.migrations() {
            // MigrationDeclaration validates versions on construction,
            // so we just check name here (already validated by constructor)
            if migration.name().is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidFieldType,
                    "Migration name cannot be empty".to_string(),
                ));
            }
        }

        diagnostics
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, TraitSet};

    fn minimal_definition() -> PlanetDefinition {
        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        PlanetDefinition::new("1.0", body).unwrap()
    }

    #[test]
    fn test_type_checker_creation() {
        let checker = TypeChecker::new(false);
        assert!(!checker.strict);

        let strict_checker = TypeChecker::new(true);
        assert!(strict_checker.strict);
    }

    #[test]
    fn test_check_field_types_minimal() {
        let checker = TypeChecker::default();
        let definition = minimal_definition();
        let diagnostics = checker.check_field_types(&definition);
        assert!(
            diagnostics.is_empty(),
            "Minimal definition should have no type errors"
        );
    }

    #[test]
    fn test_check_required_fields_minimal() {
        let checker = TypeChecker::default();
        let definition = minimal_definition();
        let diagnostics = checker.check_required_fields(&definition);
        assert!(
            diagnostics.is_empty(),
            "Minimal definition should have all required fields"
        );
    }

    #[test]
    fn test_check_enum_variants() {
        let checker = TypeChecker::default();
        let definition = minimal_definition();
        let diagnostics = checker.check_enum_variants(&definition);
        assert!(diagnostics.is_empty(), "No enums to validate yet");
    }

    #[test]
    fn test_valid_properties() {
        let checker = TypeChecker::default();

        let props = PhysicalProperties {
            mass_kg: Some(5_972_000_000_000_000_000_000_000),
            radius_m: Some(6_371_000),
            ..PhysicalProperties::default()
        };

        let body = CelestialBody::new(
            BodyId::new("earth").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = checker.check_field_types(&definition);

        assert!(diagnostics.is_empty(), "Valid properties should pass");
    }
}
