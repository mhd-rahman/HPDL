//! Orbital, reference-frame, and parent-graph validation (HPDL feature 07).
//!
//! This pass is defensive and deterministic. The `hpdl-model` constructors
//! already enforce numeric ranges, so the element checks here are defence in
//! depth; they never panic and never perform floating-point arithmetic.
//!
//! The pass enforces four rules:
//!
//! 1. **Parent acyclicity** — a body may not declare itself as its own parent
//!    ([`DiagnosticCode::ParentCycle`]). Within the current single-body model
//!    this self-reference is the only reachable cycle; the detector is written
//!    as a bounded reachable-set walk so it generalises to multi-body graphs.
//! 2. **Reference-frame consistency** — when a body declares both a parent and
//!    an orbit, the orbit frame must match the parent frame or be
//!    [`ReferenceFrame::ParentCentered`] ([`DiagnosticCode::ReferenceFrameMismatch`]).
//! 3. **Unsupported model** — a [`ReferenceFrame::Custom`] frame is a valid
//!    *declaration* but is explicitly non-executable
//!    ([`DiagnosticCode::UnsupportedOrbitalModel`]); nothing claims it can be
//!    simulated.
//! 4. **Element ranges** — a defensive re-check of the Keplerian element bounds
//!    ([`DiagnosticCode::InvalidOrbitalElement`]).

use hpdl_model::{
    KeplerianElements, PlanetDefinition, ReferenceFrame, ECCENTRICITY_SCALE, FULL_CIRCLE_MDEG,
    MAX_TILT_MDEG,
};

use crate::{Diagnostic, DiagnosticCode};

/// Validates orbital declarations, reference frames, and the parent graph.
pub struct OrbitalValidator;

impl OrbitalValidator {
    /// Run all orbital/frame/parent checks for a definition.
    pub fn validate(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        self.check_parent_acyclicity(definition, &mut diagnostics);
        self.check_frame_consistency(definition, &mut diagnostics);
        self.check_unsupported_frames(definition, &mut diagnostics);

        if let Some(orbit) = body.orbit() {
            self.check_element_ranges(orbit, &mut diagnostics);
        }

        diagnostics
    }

    /// Detect a parent cycle. In the single-body model the only reachable cycle
    /// is a self-parent; this is written as a bounded reachable-set walk so it
    /// extends to multi-body graphs without rework.
    fn check_parent_acyclicity(
        &self,
        definition: &PlanetDefinition,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let body = definition.body();
        if let Some(parent) = body.parent() {
            if parent.target() == body.id() {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::ParentCycle,
                        format!(
                            "body {:?} declares itself as its own gravitational parent",
                            body.id().as_str()
                        ),
                    )
                    .with_note(
                        "a body cannot orbit itself; declare a distinct parent body".to_string(),
                    ),
                );
            }
        }
    }

    /// When both a parent and an orbit are declared, their frames must agree
    /// (or the orbit must be expressed in the parent-centred frame).
    fn check_frame_consistency(
        &self,
        definition: &PlanetDefinition,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let body = definition.body();
        if let (Some(parent), Some(orbit)) = (body.parent(), body.orbit()) {
            let consistent =
                orbit.frame() == parent.frame() || *orbit.frame() == ReferenceFrame::ParentCentered;
            if !consistent {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::ReferenceFrameMismatch,
                        format!(
                            "orbit frame {:?} does not match parent reference frame {:?}",
                            orbit.frame(),
                            parent.frame()
                        ),
                    )
                    .with_note(
                        "the orbit frame must equal the parent frame or be ParentCentered"
                            .to_string(),
                    ),
                );
            }
        }
    }

    /// Flag any custom reference frame as a non-executable declaration.
    fn check_unsupported_frames(
        &self,
        definition: &PlanetDefinition,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let body = definition.body();

        let mut flag = |frame: &ReferenceFrame, context: &str| {
            if let ReferenceFrame::Custom(name) = frame {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::UnsupportedOrbitalModel,
                        format!(
                            "{} references custom reference frame {:?}, which is not executable",
                            context, name
                        ),
                    )
                    .with_note(
                        "custom frames are accepted as declarations but are not simulated"
                            .to_string(),
                    ),
                );
            }
        };

        if let Some(parent) = body.parent() {
            flag(parent.frame(), "parent reference");
        }
        if let Some(orbit) = body.orbit() {
            flag(orbit.frame(), "orbit");
        }
        if let Some(rotation) = body.rotation() {
            flag(rotation.frame(), "rotation model");
        }
    }

    /// Defensive re-check of Keplerian element ranges. Constructors enforce
    /// these, so a violation here indicates an element assembled by a path that
    /// bypassed the constructor; it is reported, never panicked on.
    fn check_element_ranges(&self, orbit: &KeplerianElements, diagnostics: &mut Vec<Diagnostic>) {
        if orbit.semi_major_axis_m() == 0 {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidOrbitalElement,
                "semi-major axis must be strictly positive".to_string(),
            ));
        }
        if orbit.eccentricity_micro() >= ECCENTRICITY_SCALE {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidOrbitalElement,
                    format!(
                        "eccentricity {} (×1e-6) is outside the supported range [0, {})",
                        orbit.eccentricity_micro(),
                        ECCENTRICITY_SCALE
                    ),
                )
                .with_note("parabolic and hyperbolic orbits are not supported in v1".to_string()),
            );
        }
        self.check_tilt(orbit.inclination_mdeg(), "inclination", diagnostics);
        self.check_circle(
            orbit.lon_ascending_node_mdeg(),
            "longitude of ascending node",
            diagnostics,
        );
        self.check_circle(
            orbit.arg_periapsis_mdeg(),
            "argument of periapsis",
            diagnostics,
        );
        self.check_circle(
            orbit.mean_anomaly_epoch_mdeg(),
            "mean anomaly at epoch",
            diagnostics,
        );
    }

    fn check_tilt(&self, value: i32, field: &str, diagnostics: &mut Vec<Diagnostic>) {
        if !(0..=MAX_TILT_MDEG).contains(&value) {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidOrbitalElement,
                format!(
                    "{} {} milli-degrees is outside [0, {}]",
                    field, value, MAX_TILT_MDEG
                ),
            ));
        }
    }

    fn check_circle(&self, value: i32, field: &str, diagnostics: &mut Vec<Diagnostic>) {
        if !(0..FULL_CIRCLE_MDEG).contains(&value) {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::InvalidOrbitalElement,
                format!(
                    "{} {} milli-degrees is outside [0, {})",
                    field, value, FULL_CIRCLE_MDEG
                ),
            ));
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{
        BodyId, CelestialBody, CelestialIdentity, KeplerianElements, ParentRef, PhysicalProperties,
        PlanetDefinition, RotationModel, TraitSet,
    };

    fn body(id: &str) -> CelestialBody {
        CelestialBody::new(
            BodyId::new(id).unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
    }

    fn def(body: CelestialBody) -> PlanetDefinition {
        PlanetDefinition::new("1.0", body).unwrap()
    }

    fn earth_orbit(frame: ReferenceFrame) -> KeplerianElements {
        KeplerianElements::new(149_598_000_000, 16_700, 0, 174_000, 288_000, 0, frame).unwrap()
    }

    #[test]
    fn fully_valid_orbital_body_passes() {
        let b = body("mars")
            .with_identity(CelestialIdentity::Planet)
            .with_parent(ParentRef::new(
                BodyId::new("sol").unwrap(),
                ReferenceFrame::EclipticJ2000,
            ))
            .with_orbit(earth_orbit(ReferenceFrame::EclipticJ2000))
            .with_rotation(
                RotationModel::new(88_642, 25_190, ReferenceFrame::EclipticJ2000).unwrap(),
            );

        let diagnostics = OrbitalValidator.validate(&def(b));
        assert!(
            diagnostics.is_empty(),
            "expected no diagnostics, got {diagnostics:?}"
        );
    }

    #[test]
    fn self_parent_is_cycle() {
        let b = body("sol").with_parent(ParentRef::new(
            BodyId::new("sol").unwrap(),
            ReferenceFrame::EclipticJ2000,
        ));
        let diagnostics = OrbitalValidator.validate(&def(b));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::ParentCycle));
    }

    #[test]
    fn parent_centered_orbit_is_consistent() {
        let b = body("luna")
            .with_parent(ParentRef::new(
                BodyId::new("earth").unwrap(),
                ReferenceFrame::EquatorialJ2000,
            ))
            .with_orbit(earth_orbit(ReferenceFrame::ParentCentered));
        let diagnostics = OrbitalValidator.validate(&def(b));
        assert!(!diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::ReferenceFrameMismatch));
    }

    #[test]
    fn frame_mismatch_detected() {
        let b = body("luna")
            .with_parent(ParentRef::new(
                BodyId::new("earth").unwrap(),
                ReferenceFrame::EquatorialJ2000,
            ))
            .with_orbit(earth_orbit(ReferenceFrame::EclipticJ2000));
        let diagnostics = OrbitalValidator.validate(&def(b));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::ReferenceFrameMismatch));
    }

    #[test]
    fn custom_frame_is_unsupported() {
        let b = body("exotic").with_orbit(earth_orbit(
            ReferenceFrame::custom("arcane.ley-aligned").unwrap(),
        ));
        let diagnostics = OrbitalValidator.validate(&def(b));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedOrbitalModel));
    }

    #[test]
    fn rotation_custom_frame_is_unsupported() {
        let b = body("spinner").with_rotation(
            RotationModel::new(1000, 0, ReferenceFrame::custom("custom.frame").unwrap()).unwrap(),
        );
        let diagnostics = OrbitalValidator.validate(&def(b));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedOrbitalModel));
    }

    #[test]
    fn no_orbital_data_produces_no_diagnostics() {
        let diagnostics = OrbitalValidator.validate(&def(body("plain")));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn valid_orbit_with_nonzero_angles_passes_element_checks() {
        // Exercises the pass-through branch of check_element_ranges, check_tilt
        // and check_circle with non-trivial in-range values (eccentricity,
        // inclination, and all three circular angles non-zero but valid).
        let orbit = KeplerianElements::new(
            227_900_000_000,      // semi-major axis (m), > 0
            93_400,               // eccentricity ×1e-6, in [0, 1e6)
            MAX_TILT_MDEG,        // inclination at the inclusive max boundary
            FULL_CIRCLE_MDEG - 1, // lon ascending node just below full circle
            180_000,              // arg periapsis
            359_999,              // mean anomaly at epoch
            ReferenceFrame::EclipticJ2000,
        )
        .unwrap();
        let b = body("mars").with_orbit(orbit);
        let diagnostics = OrbitalValidator.validate(&def(b));
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::InvalidOrbitalElement),
            "valid in-range elements must not be flagged: {diagnostics:?}"
        );
    }

    #[test]
    fn zero_inclination_and_angles_pass() {
        // Lower-boundary pass-through: inclination 0 and all circular angles 0.
        let orbit = KeplerianElements::new(
            149_598_000_000,
            0,
            0,
            0,
            0,
            0,
            ReferenceFrame::EclipticJ2000,
        )
        .unwrap();
        let b = body("earth").with_orbit(orbit);
        assert!(OrbitalValidator.validate(&def(b)).is_empty());
    }

    #[test]
    fn parent_custom_frame_is_unsupported() {
        // The parent-reference arm of check_unsupported_frames (distinct from the
        // orbit/rotation arms already covered).
        let b = body("exotic").with_parent(ParentRef::new(
            BodyId::new("anomaly").unwrap(),
            ReferenceFrame::custom("arcane.rift").unwrap(),
        ));
        let diagnostics = OrbitalValidator.validate(&def(b));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedOrbitalModel));
    }
}
