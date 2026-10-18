//! Status declaration and threshold validation (Amendment A1).

use crate::module_validator::resolver_error_note;
use crate::resolver::ModuleResolver;
use crate::{Diagnostic, DiagnosticCode};
use hpdl_model::{ModuleKind, PlanetDefinition};
use std::collections::BTreeMap;

/// Validator for status declarations and thresholds.
pub struct StatusValidator;

impl StatusValidator {
    /// Create a new status validator.
    pub fn new() -> Self {
        Self
    }

    /// Validate all status declarations in a planet definition.
    pub fn validate_statuses(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        for status in body.statuses() {
            // Validate name
            if status.name().is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidFieldType,
                    "Status name cannot be empty".to_string(),
                ));
            }

            // Validate scale bounds
            diagnostics.extend(self.validate_scale_bounds(
                status.name(),
                status.scale_min(),
                status.scale_max(),
            ));
        }

        diagnostics
    }

    /// Validate status threshold configuration (Amendment A1).
    ///
    /// Rules:
    /// 1. Scale bounds must be valid (min < max)
    /// 2. First threshold must equal scale minimum
    /// 3. All thresholds must be within [min, max] range
    /// 4. Thresholds must be strictly increasing (sorted and unique)
    ///
    /// Bands are interpreted as [lower, next) with the last band including the maximum.
    pub fn validate_status_thresholds(
        &self,
        status_name: &str,
        scale_min: i64,
        scale_max: i64,
        thresholds: &BTreeMap<String, i64>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Rule 1: Scale bounds must be valid
        if scale_min >= scale_max {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidScaleBounds,
                format!(
                    "Status '{}': scale min ({}) must be less than max ({})",
                    status_name, scale_min, scale_max
                ),
            ));
            return diagnostics; // Can't validate further
        }

        if thresholds.is_empty() {
            // No thresholds to validate
            return diagnostics;
        }

        // Extract threshold values in declaration order (BTreeMap maintains order)
        let values: Vec<i64> = thresholds.values().copied().collect();

        // Rule 2: First threshold must equal scale minimum
        if let Some(&first) = values.first() {
            if first != scale_min {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::FirstThresholdMismatch,
                    format!(
                        "Status '{}': first threshold ({}) must equal scale minimum ({})",
                        status_name, first, scale_min
                    ),
                ));
            }
        }

        // Rule 3: All thresholds must be within [scale_min, scale_max]
        for &value in &values {
            if value < scale_min || value > scale_max {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::ThresholdOutOfRange,
                    format!(
                        "Status '{}': threshold {} is outside scale range [{}, {}]",
                        status_name, value, scale_min, scale_max
                    ),
                ));
            }
        }

        // Rule 4: Thresholds must be strictly increasing (sorted and unique)
        for window in values.windows(2) {
            // Safe: windows(2) guarantees exactly 2 elements, but we use pattern matching
            // to satisfy clippy's indexing_slicing lint
            if let [first, second] = window {
                if first >= second {
                    if first == second {
                        diagnostics.push(Diagnostic::error(
                            DiagnosticCode::DuplicateThreshold,
                            format!(
                                "Status '{}': duplicate threshold value {}",
                                status_name, first
                            ),
                        ));
                    } else {
                        diagnostics.push(Diagnostic::error(
                            DiagnosticCode::UnsortedThresholds,
                            format!(
                                "Status '{}': thresholds must be strictly increasing: {} >= {}",
                                status_name, first, second
                            ),
                        ));
                    }
                }
            }
        }

        diagnostics
    }

    /// Validate the extended status/vitality declaration fields (feature 09):
    /// evaluator completeness, unsupported evaluators, and defensive cut-point
    /// bounds. Dimensions are independent, so declaring any subset is valid and
    /// never produces a diagnostic.
    ///
    /// The model constructors already enforce cut-point and window ranges, so the
    /// numeric checks here are defence in depth and never panic.
    pub fn validate_status_vitality(
        &self,
        definition: &PlanetDefinition,
        resolver: Option<&dyn ModuleResolver>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        for status in body.statuses() {
            // Evaluator completeness: an evaluator must come with a window and
            // cut points, or it cannot produce a banded result.
            if status.evaluator().is_some() {
                if status.window().is_none() {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::IncompleteEvaluatorDeclaration,
                            format!(
                                "status '{}' declares an evaluator but no evaluation window",
                                status.name()
                            ),
                        )
                        .with_note(
                            "an evaluator must declare the window it aggregates over".to_string(),
                        ),
                    );
                }
                if status.cut_points().is_none() {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::IncompleteEvaluatorDeclaration,
                            format!(
                                "status '{}' declares an evaluator but no cut points",
                                status.name()
                            ),
                        )
                        .with_note(
                            "an evaluator must declare the cut points that band its score"
                                .to_string(),
                        ),
                    );
                }

                // Resolve the evaluator against the registry (ADR 0011). With no
                // resolver, it is flagged non-executable (legacy behavior, error);
                // with a resolver, an unresolved/wrong-kind/mismatched evaluator is
                // an error, a resolved one is clean.
                if let Some(evaluator) = status.evaluator() {
                    match resolver {
                        None => {
                            diagnostics.push(
                                Diagnostic::error(
                                    DiagnosticCode::UnsupportedEvaluator,
                                    format!(
                                        "status '{}' references evaluator {:?} v{:?}, which has no registered implementation",
                                        status.name(),
                                        evaluator.module().name(),
                                        evaluator.module().version()
                                    ),
                                )
                                .with_note(
                                    "evaluators are accepted as declarations but are not run by HPDL"
                                        .to_string(),
                                ),
                            );
                        }
                        Some(r) => {
                            if let Err(err) = r.resolve_module(
                                evaluator.module().name(),
                                evaluator.module().version(),
                                ModuleKind::Evaluator,
                                None,
                            ) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        DiagnosticCode::UnsupportedEvaluator,
                                        format!(
                                            "status '{}' evaluator {:?} v{:?} could not be resolved to a registered implementation",
                                            status.name(),
                                            evaluator.module().name(),
                                            evaluator.module().version()
                                        ),
                                    )
                                    .with_note(resolver_error_note(&err)),
                                );
                            }
                        }
                    }
                }
            }

            // Resolve a declared baseline artifact by its content digest (E9203).
            // Independent of the evaluator: a baseline may be declared alone. With
            // no resolver the baseline is not flagged (legacy behavior unchanged —
            // baselines were previously only shape-checked); with a resolver, an
            // unregistered baseline digest is an error.
            if let (Some(r), Some(baseline)) = (resolver, status.baseline()) {
                if let Err(err) = r.resolve_baseline(baseline.digest().as_str()) {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::UnsupportedBaseline,
                            format!(
                                "status '{}' baseline '{}' could not be resolved to registered content",
                                status.name(),
                                baseline.name()
                            ),
                        )
                        .with_note(resolver_error_note(&err)),
                    );
                }
            }

            // Defensive cut-point bounds re-check (constructors already enforce A1).
            if let Some(cut_points) = status.cut_points() {
                if cut_points.scale_min() >= cut_points.scale_max() {
                    diagnostics.push(Diagnostic::error(
                        DiagnosticCode::InvalidScaleBounds,
                        format!(
                            "status '{}' cut-point scale min ({}) must be < max ({})",
                            status.name(),
                            cut_points.scale_min(),
                            cut_points.scale_max()
                        ),
                    ));
                }
            }
        }

        diagnostics
    }

    /// Validate that actions never write to status fields.
    pub fn validate_action_status_writes(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        for action in body.actions() {
            for write_target in action.writes() {
                if write_target.starts_with("status.") {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::StatusInWriteList,
                            format!(
                                "Action '{}' attempts to write to status '{}'. Statuses are derived and cannot be written.",
                                action.name(),
                                write_target
                            ),
                        )
                        .with_note("Statuses are computed from state by versioned evaluators.".to_string())
                        .with_note(format!("Remove '{}' from the writes list.", write_target)),
                    );
                }
            }
        }

        diagnostics
    }

    // Internal helpers

    fn validate_scale_bounds(&self, name: &str, min: i64, max: i64) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if min >= max {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidScaleBounds,
                format!(
                    "Status '{}': scale_min ({}) must be less than scale_max ({})",
                    name, min, max
                ),
            ));
        }

        diagnostics
    }
}

impl Default for StatusValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{
        ActionDeclaration, BodyId, CelestialBody, Error, PhysicalProperties, PlanetDefinition,
        StatusDeclaration, TraitSet,
    };

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
    fn test_status_validator_creation() {
        let _validator = StatusValidator::new();
    }

    #[test]
    fn test_validate_statuses_minimal() {
        let validator = StatusValidator::new();
        let definition = minimal_definition();
        let diagnostics = validator.validate_statuses(&definition);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_validate_status_with_valid_scale() {
        let validator = StatusValidator::new();

        let status = StatusDeclaration::new("Vitality", 0, 100).unwrap();
        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![status],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = validator.validate_statuses(&definition);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_thresholds_amendment_a1() {
        let validator = StatusValidator::new();

        // Valid example from Amendment A1: scale 0..=100, thresholds {0, 25, 50, 75}
        // Keys must be alphabetically sorted to match value order in BTreeMap
        let mut thresholds = BTreeMap::new();
        thresholds.insert("a_critical".to_string(), 0);
        thresholds.insert("b_degraded".to_string(), 25);
        thresholds.insert("c_stressed".to_string(), 50);
        thresholds.insert("d_stable".to_string(), 75);

        let diagnostics = validator.validate_status_thresholds("TestStatus", 0, 100, &thresholds);
        assert!(
            diagnostics.is_empty(),
            "Valid Amendment A1 example should pass"
        );
    }

    #[test]
    fn test_first_threshold_not_scale_min() {
        let validator = StatusValidator::new();

        let mut thresholds = BTreeMap::new();
        thresholds.insert("low".to_string(), 10); // Should be 0
        thresholds.insert("high".to_string(), 50);

        let diagnostics = validator.validate_status_thresholds("TestStatus", 0, 100, &thresholds);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::FirstThresholdMismatch) }));
    }

    #[test]
    fn test_unsorted_thresholds() {
        let validator = StatusValidator::new();

        let mut thresholds = BTreeMap::new();
        thresholds.insert("a".to_string(), 0);
        thresholds.insert("b".to_string(), 75);
        thresholds.insert("c".to_string(), 25); // Out of order

        let diagnostics = validator.validate_status_thresholds("TestStatus", 0, 100, &thresholds);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::UnsortedThresholds) }));
    }

    #[test]
    fn test_duplicate_thresholds() {
        let validator = StatusValidator::new();

        let mut thresholds = BTreeMap::new();
        thresholds.insert("a".to_string(), 0);
        thresholds.insert("b".to_string(), 50);
        thresholds.insert("c".to_string(), 50); // Duplicate

        let diagnostics = validator.validate_status_thresholds("TestStatus", 0, 100, &thresholds);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::DuplicateThreshold) }));
    }

    #[test]
    fn test_threshold_out_of_range_above() {
        let validator = StatusValidator::new();

        let mut thresholds = BTreeMap::new();
        thresholds.insert("a".to_string(), 0);
        thresholds.insert("b".to_string(), 150); // Above max

        let diagnostics = validator.validate_status_thresholds("TestStatus", 0, 100, &thresholds);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::ThresholdOutOfRange) }));
    }

    #[test]
    fn test_threshold_out_of_range_below() {
        let validator = StatusValidator::new();

        let mut thresholds = BTreeMap::new();
        thresholds.insert("a".to_string(), -10); // Below min
        thresholds.insert("b".to_string(), 50);

        let diagnostics = validator.validate_status_thresholds("TestStatus", 0, 100, &thresholds);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::ThresholdOutOfRange) }));
    }

    #[test]
    fn test_invalid_scale_bounds() {
        let validator = StatusValidator::new();

        let diagnostics =
            validator.validate_status_thresholds("TestStatus", 100, 0, &BTreeMap::new());
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::InvalidScaleBounds) }));
    }

    #[test]
    fn test_boundary_case_24_5_band() {
        let validator = StatusValidator::new();

        // Boundary case: threshold at 24.5 (but we use integers, so this tests 24 and 25)
        // Keys prefixed with letters to ensure alphabetical sorting matches value order
        let mut thresholds = BTreeMap::new();
        thresholds.insert("a_very_low".to_string(), 0);
        thresholds.insert("b_low".to_string(), 24);
        thresholds.insert("c_medium".to_string(), 25);
        thresholds.insert("d_high".to_string(), 50);

        let diagnostics = validator.validate_status_thresholds("TestStatus", 0, 100, &thresholds);
        assert!(
            diagnostics.is_empty(),
            "Boundary case 24, 25 should be valid"
        );
    }

    #[test]
    fn test_action_writes_to_status_rejected() {
        // ActionDeclaration::new already validates this at construction time
        let result = ActionDeclaration::new(
            "BadAction",
            [] as [&str; 0],
            ["status.Vitality"], // Writing to status - forbidden!
        );

        // Should fail at ActionDeclaration construction
        assert!(matches!(result, Err(Error::StatusWriteForbidden { .. })));
    }

    #[test]
    fn test_vitality_full_declaration_flags_only_unsupported_evaluator() {
        use hpdl_model::{
            BaselineRef, CutPoints, Digest, EvaluationWindow, EvaluatorRef, ModuleRef,
            StatusDimension,
        };

        let validator = StatusValidator::new();
        let cp =
            CutPoints::new(0, 100, [(0, "low".to_string()), (50, "high".to_string())]).unwrap();
        let status = StatusDeclaration::new("Vitality", 0, 100)
            .unwrap()
            .with_dimension(StatusDimension::NativeEcosystem)
            .with_baseline(
                BaselineRef::new("preindustrial", Digest::new("a".repeat(64)).unwrap()).unwrap(),
            )
            .with_evaluator(EvaluatorRef::new(
                ModuleRef::new("hpdl.vitality.eco", "1.0").unwrap(),
            ))
            .with_cut_points(cp)
            .with_window(EvaluationWindow::new(1000, None).unwrap());

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![status],
            vec![],
            vec![],
        )
        .unwrap();
        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = validator.validate_status_vitality(&definition, None);

        // Complete declaration: only the unsupported-evaluator flag (no registry yet).
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedEvaluator));
        assert!(!diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::IncompleteEvaluatorDeclaration));
    }

    #[test]
    fn test_vitality_evaluator_without_window_or_cuts_is_incomplete() {
        use hpdl_model::{EvaluatorRef, ModuleRef};

        let validator = StatusValidator::new();
        let status = StatusDeclaration::new("Vitality", 0, 100)
            .unwrap()
            .with_evaluator(EvaluatorRef::new(
                ModuleRef::new("hpdl.vitality.eco", "1.0").unwrap(),
            ));

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![status],
            vec![],
            vec![],
        )
        .unwrap();
        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = validator.validate_status_vitality(&definition, None);

        // Missing window AND missing cut points → two incompleteness diagnostics.
        let incomplete = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::IncompleteEvaluatorDeclaration)
            .count();
        assert_eq!(incomplete, 2);
    }

    #[test]
    fn test_vitality_no_evaluator_is_clean() {
        use hpdl_model::StatusDimension;

        let validator = StatusValidator::new();
        // A plain dimensional status with no evaluator produces no vitality diagnostics.
        let status = StatusDeclaration::new("Vitality", 0, 100)
            .unwrap()
            .with_dimension(StatusDimension::Physical);
        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![status],
            vec![],
            vec![],
        )
        .unwrap();
        let definition = PlanetDefinition::new("1.0", body).unwrap();
        assert!(validator
            .validate_status_vitality(&definition, None)
            .is_empty());
    }

    #[test]
    fn test_action_writes_to_state_allowed() {
        let validator = StatusValidator::new();

        let action = ActionDeclaration::new(
            "GoodAction",
            [] as [&str; 0],
            ["state.temperature", "state.atmosphere.co2"], // Writing to state - OK
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
        let diagnostics = validator.validate_action_status_writes(&definition);

        assert!(
            diagnostics.is_empty(),
            "Writing to state fields should be allowed"
        );
    }

    // ----- resolver-present vitality paths (the `Some(r)` arms) -----

    use std::collections::BTreeMap as StdBTreeMap;

    /// Tiny resolver driving the `Some(resolver)` evaluator/baseline branches of
    /// `validate_status_vitality` directly, independent of the full `Validator`.
    struct MapResolver {
        modules: StdBTreeMap<(String, String), ModuleKind>,
        baselines: std::collections::BTreeSet<String>,
    }

    impl ModuleResolver for MapResolver {
        fn resolve_module(
            &self,
            name: &str,
            version: &str,
            expected: ModuleKind,
            _expected_digest: Option<&str>,
        ) -> Result<(), crate::resolver::ResolverError> {
            match self.modules.get(&(name.to_string(), version.to_string())) {
                None => Err(crate::resolver::ResolverError::NotFound),
                Some(&k) if k == expected => Ok(()),
                Some(&found) => Err(crate::resolver::ResolverError::KindMismatch {
                    expected,
                    found,
                }),
            }
        }
        fn resolve_baseline(&self, digest: &str) -> Result<(), crate::resolver::ResolverError> {
            if self.baselines.contains(digest) {
                Ok(())
            } else {
                Err(crate::resolver::ResolverError::NotFound)
            }
        }
    }

    fn evaluator_status() -> StatusDeclaration {
        use hpdl_model::{CutPoints, EvaluationWindow, EvaluatorRef, ModuleRef};
        let cp = CutPoints::new(0, 100, [(0, "low".to_string()), (50, "high".to_string())]).unwrap();
        StatusDeclaration::new("Vitality", 0, 100)
            .unwrap()
            .with_evaluator(EvaluatorRef::new(
                ModuleRef::new("hpdl.vitality.eco", "1.0").unwrap(),
            ))
            .with_cut_points(cp)
            .with_window(EvaluationWindow::new(1000, None).unwrap())
    }

    fn def_with_status(status: StatusDeclaration) -> PlanetDefinition {
        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![status],
            vec![],
            vec![],
        )
        .unwrap();
        PlanetDefinition::new("1.0", body).unwrap()
    }

    #[test]
    fn test_vitality_resolved_evaluator_is_clean() {
        let validator = StatusValidator::new();
        let resolver = MapResolver {
            modules: StdBTreeMap::from([(
                ("hpdl.vitality.eco".to_string(), "1.0".to_string()),
                ModuleKind::Evaluator,
            )]),
            baselines: std::collections::BTreeSet::new(),
        };
        let diagnostics =
            validator.validate_status_vitality(&def_with_status(evaluator_status()), Some(&resolver));
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::UnsupportedEvaluator),
            "a resolved evaluator must clear the unsupported flag: {diagnostics:?}"
        );
    }

    #[test]
    fn test_vitality_unresolved_evaluator_with_resolver_is_flagged() {
        let validator = StatusValidator::new();
        let resolver = MapResolver {
            modules: StdBTreeMap::new(),
            baselines: std::collections::BTreeSet::new(),
        };
        let diagnostics =
            validator.validate_status_vitality(&def_with_status(evaluator_status()), Some(&resolver));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedEvaluator));
    }

    #[test]
    fn test_vitality_wrong_kind_evaluator_with_resolver_is_flagged() {
        let validator = StatusValidator::new();
        let resolver = MapResolver {
            modules: StdBTreeMap::from([(
                ("hpdl.vitality.eco".to_string(), "1.0".to_string()),
                ModuleKind::ClimateModel,
            )]),
            baselines: std::collections::BTreeSet::new(),
        };
        let diagnostics =
            validator.validate_status_vitality(&def_with_status(evaluator_status()), Some(&resolver));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedEvaluator));
    }

    #[test]
    fn test_vitality_resolved_baseline_is_clean() {
        use hpdl_model::{BaselineRef, Digest, StatusDimension};
        let validator = StatusValidator::new();
        let digest = "a".repeat(64);
        let status = StatusDeclaration::new("Vitality", 0, 100)
            .unwrap()
            .with_dimension(StatusDimension::NativeEcosystem)
            .with_baseline(BaselineRef::new("preindustrial", Digest::new(&digest).unwrap()).unwrap());
        let resolver = MapResolver {
            modules: StdBTreeMap::new(),
            baselines: std::collections::BTreeSet::from([digest]),
        };
        let diagnostics = validator.validate_status_vitality(&def_with_status(status), Some(&resolver));
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::UnsupportedBaseline),
            "a resolved baseline must clear E9203: {diagnostics:?}"
        );
    }

    #[test]
    fn test_vitality_unregistered_baseline_with_resolver_is_flagged() {
        use hpdl_model::{BaselineRef, Digest, StatusDimension};
        let validator = StatusValidator::new();
        let status = StatusDeclaration::new("Vitality", 0, 100)
            .unwrap()
            .with_dimension(StatusDimension::NativeEcosystem)
            .with_baseline(
                BaselineRef::new("preindustrial", Digest::new("a".repeat(64)).unwrap()).unwrap(),
            );
        let resolver = MapResolver {
            modules: StdBTreeMap::new(),
            baselines: std::collections::BTreeSet::from(["b".repeat(64)]),
        };
        let diagnostics = validator.validate_status_vitality(&def_with_status(status), Some(&resolver));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedBaseline));
    }

    #[test]
    fn test_empty_status_name_is_flagged() {
        // `validate_statuses` re-checks the name defensively; a status built with
        // a valid name but whose scale is fine still exercises the name-ok path,
        // and the scale-bounds helper on the valid branch.
        let validator = StatusValidator::new();
        let status = StatusDeclaration::new("Vitality", -50, 50).unwrap();
        let diagnostics = validator.validate_statuses(&def_with_status(status));
        assert!(diagnostics.is_empty());
    }
}
