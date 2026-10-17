//! Action declaration validation.

use crate::{Diagnostic, DiagnosticCode};
use hpdl_model::PlanetDefinition;

/// Validator for action declarations.
pub struct ActionValidator;

impl ActionValidator {
    /// Create a new action validator.
    pub fn new() -> Self {
        Self
    }

    /// Validate all action declarations.
    pub fn validate_actions(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        for action in body.actions() {
            // Validate action name
            if action.name().is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidActionReference,
                    "Action name cannot be empty".to_string(),
                ));
            }

            if action.name().len() > 128 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidActionReference,
                    format!(
                        "Action name '{}' exceeds maximum length of 128",
                        action.name()
                    ),
                ));
            }

            // Validate reads and writes are valid field references
            for read_field in action.reads() {
                if read_field.is_empty() {
                    diagnostics.push(Diagnostic::error(
                        DiagnosticCode::InvalidActionReference,
                        format!("Action '{}': read field cannot be empty", action.name()),
                    ));
                }
            }

            for write_field in action.writes() {
                if write_field.is_empty() {
                    diagnostics.push(Diagnostic::error(
                        DiagnosticCode::InvalidActionReference,
                        format!("Action '{}': write field cannot be empty", action.name()),
                    ));
                }
            }
        }

        diagnostics
    }

    /// Validate action capabilities and write→capability coverage (feature 10).
    ///
    /// For each action: assemble its [`ReadWriteSet`] (which validates field-path
    /// well-formedness), then report any mutating write not backed by a declared
    /// capability. A write targeting a `status.*` field is already rejected at
    /// `ActionDeclaration` construction (status writes are forbidden), so it never
    /// reaches here.
    pub fn validate_capabilities(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        for action in body.actions() {
            match action.read_write_set() {
                Ok(rws) => {
                    for write in rws.uncovered_writes() {
                        diagnostics.push(
                            Diagnostic::error(
                                DiagnosticCode::UncoveredWrite,
                                format!(
                                    "action '{}' writes to {:?} with no declared capability authorizing it",
                                    action.name(),
                                    write
                                ),
                            )
                            .with_note(
                                "declare a capability whose namespace covers the write path"
                                    .to_string(),
                            ),
                        );
                    }
                }
                Err(e) => {
                    diagnostics.push(Diagnostic::error(
                        DiagnosticCode::MalformedFieldPath,
                        format!("action '{}' has a malformed field path: {e}", action.name()),
                    ));
                }
            }
        }

        diagnostics
    }
}

impl Default for ActionValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{ActionDeclaration, BodyId, CelestialBody, PhysicalProperties, TraitSet};

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
    fn test_action_validator_creation() {
        let _validator = ActionValidator::new();
    }

    #[test]
    fn test_validate_actions_minimal() {
        let validator = ActionValidator::new();
        let definition = minimal_definition();
        let diagnostics = validator.validate_actions(&definition);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_action() {
        let validator = ActionValidator::new();

        let action = ActionDeclaration::new(
            "TerraformStep",
            ["state.atmosphere.co2", "state.temperature"],
            ["state.atmosphere.co2"],
        )
        .unwrap();

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![action],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = validator.validate_actions(&definition);
        assert!(diagnostics.is_empty());
    }

    fn def_with_action(action: ActionDeclaration) -> PlanetDefinition {
        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![action],
            vec![],
        )
        .unwrap();
        PlanetDefinition::new("1.0", body).unwrap()
    }

    #[test]
    fn test_capability_covered_write_is_clean() {
        use hpdl_model::Capability;
        let validator = ActionValidator::new();
        let action = ActionDeclaration::new("T", [] as [&str; 0], ["state.atmosphere.co2"])
            .unwrap()
            .with_capabilities([Capability::new("state.atmosphere.write").unwrap()]);
        let diagnostics = validator.validate_capabilities(&def_with_action(action));
        assert!(
            diagnostics.is_empty(),
            "covered write should be clean: {diagnostics:?}"
        );
    }

    #[test]
    fn test_uncovered_write_is_flagged() {
        use hpdl_model::Capability;
        let validator = ActionValidator::new();
        // Writes hydrosphere but only declares an atmosphere capability.
        let action = ActionDeclaration::new("T", [] as [&str; 0], ["state.hydrosphere.level"])
            .unwrap()
            .with_capabilities([Capability::new("state.atmosphere.write").unwrap()]);
        let diagnostics = validator.validate_capabilities(&def_with_action(action));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UncoveredWrite));
    }

    #[test]
    fn test_write_with_no_capability_is_flagged() {
        let validator = ActionValidator::new();
        let action =
            ActionDeclaration::new("T", [] as [&str; 0], ["state.atmosphere.co2"]).unwrap();
        let diagnostics = validator.validate_capabilities(&def_with_action(action));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UncoveredWrite));
    }

    #[test]
    fn test_malformed_field_path_is_flagged() {
        let validator = ActionValidator::new();
        // A write path with an empty segment is malformed.
        let action = ActionDeclaration::new("T", [] as [&str; 0], ["state..co2"]).unwrap();
        let diagnostics = validator.validate_capabilities(&def_with_action(action));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::MalformedFieldPath));
    }

    #[test]
    fn test_read_only_action_needs_no_capability() {
        let validator = ActionValidator::new();
        // Reads only, no writes → no capability required, no diagnostics.
        let action =
            ActionDeclaration::new("Observe", ["state.atmosphere.co2"], [] as [&str; 0]).unwrap();
        let diagnostics = validator.validate_capabilities(&def_with_action(action));
        assert!(diagnostics.is_empty());
    }
}
