//! Physical constraint validation and cross-field consistency checking.

use crate::{Diagnostic, DiagnosticCode};
use hpdl_model::PlanetDefinition;

/// Constraint solver for validating physical constraints and consistency.
pub struct ConstraintSolver {
    tolerance: f64,
}

impl ConstraintSolver {
    /// Create a new constraint solver with the given tolerance for floating-point comparisons.
    pub fn new(tolerance: f64) -> Self {
        Self { tolerance }
    }

    /// Check all physical constraints in a planet definition.
    pub fn check_physical_constraints(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();
        let properties = body.properties();

        // Check mass constraints - mass_kg is Option<u128>
        if let Some(mass_kg) = properties.mass_kg {
            if mass_kg == 0 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::NegativeMass,
                    "Mass must be positive, got: 0".to_string(),
                ));
            }
        }

        // Check radius constraints - radius_m is Option<u128>
        if let Some(radius_m) = properties.radius_m {
            if radius_m == 0 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::ContradictoryConstraints,
                    "Radius must be positive, got: 0".to_string(),
                ));
            }
        }

        // Check temperature constraints (if present in future)
        // This will be expanded when temperature fields are added to PhysicalProperties

        diagnostics
    }

    /// Check cross-field consistency.
    pub fn check_cross_field_consistency(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();
        let properties = body.properties();

        // Check mass-radius relationship (basic sanity check)
        if let (Some(mass_kg), Some(radius_m)) = (properties.mass_kg, properties.radius_m) {
            // Convert u128 to f64 for density calculation
            let mass = mass_kg as f64;
            let radius = radius_m as f64;

            // Very basic density check: if density is impossibly high or low, warn
            // Volume of sphere: (4/3) * π * r³
            let volume = (4.0 / 3.0) * std::f64::consts::PI * radius.powi(3);
            let density = mass / volume;

            // Density checks (kg/m³):
            // - Less than hydrogen gas (0.1 kg/m³) is suspiciously low
            // - More than neutron star core (10^18 kg/m³) is impossibly high for planets
            if density < 0.1 {
                diagnostics.push(Diagnostic::warning(
                    DiagnosticCode::ContradictoryConstraints,
                    format!(
                        "Computed density ({:.2e} kg/m³) is extremely low. Check mass and radius values.",
                        density
                    ),
                ).with_note("Density is lower than hydrogen gas at STP".to_string()));
            }

            if density > 1.0e18 {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::ContradictoryConstraints,
                        format!(
                            "Computed density ({:.2e} kg/m³) exceeds physical limits for planets.",
                            density
                        ),
                    )
                    .with_note(
                        "This density is only found in neutron stars, not planets".to_string(),
                    ),
                );
            }
        }

        diagnostics
    }

    /// Check if two floating-point values are approximately equal.
    #[allow(dead_code)]
    fn approx_eq(&self, a: f64, b: f64) -> bool {
        (a - b).abs() < self.tolerance
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new(1e-10)
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
    fn test_constraint_solver_creation() {
        let solver = ConstraintSolver::new(1e-6);
        assert!((solver.tolerance - 1e-6).abs() < 1e-12);
    }

    #[test]
    fn test_minimal_definition_passes() {
        let solver = ConstraintSolver::default();
        let definition = minimal_definition();
        let diagnostics = solver.check_physical_constraints(&definition);
        assert!(
            diagnostics.is_empty(),
            "Minimal definition should pass constraints"
        );
    }

    #[test]
    fn test_negative_mass_rejected() {
        // Note: mass_kg is u128, so negative values are not possible
        // Testing zero mass instead
        let solver = ConstraintSolver::default();

        let props = PhysicalProperties {
            mass_kg: Some(0), // Zero mass is invalid
            ..PhysicalProperties::default()
        };

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = solver.check_physical_constraints(&definition);

        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::NegativeMass) }));
    }

    #[test]
    fn test_zero_mass_rejected() {
        let solver = ConstraintSolver::default();

        let props = PhysicalProperties {
            mass_kg: Some(0),
            ..PhysicalProperties::default()
        };

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = solver.check_physical_constraints(&definition);

        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::NegativeMass) }));
    }

    #[test]
    fn test_positive_mass_accepted() {
        let solver = ConstraintSolver::default();

        let props = PhysicalProperties {
            mass_kg: Some(5_972_000_000_000_000_000_000_000), // Earth mass in kg
            ..PhysicalProperties::default()
        };

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = solver.check_physical_constraints(&definition);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_negative_radius_rejected() {
        // Note: radius_m is u128, so negative values are not possible
        // Testing zero radius instead
        let solver = ConstraintSolver::default();

        let props = PhysicalProperties {
            radius_m: Some(0), // Zero radius is invalid
            ..PhysicalProperties::default()
        };

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = solver.check_physical_constraints(&definition);

        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::ContradictoryConstraints) }));
    }

    #[test]
    fn test_realistic_planet_passes() {
        let solver = ConstraintSolver::default();

        let props = PhysicalProperties {
            mass_kg: Some(5_972_000_000_000_000_000_000_000), // Earth mass in kg
            radius_m: Some(6_371_000),                        // Earth radius in meters
            ..PhysicalProperties::default()
        };

        let body = CelestialBody::new(
            BodyId::new("earth-like").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = solver.check_physical_constraints(&definition);
        assert!(diagnostics.is_empty());

        let cross_diagnostics = solver.check_cross_field_consistency(&definition);
        assert!(
            cross_diagnostics.is_empty(),
            "Earth-like planet should have realistic density"
        );
    }

    #[test]
    fn test_impossibly_dense_planet_rejected() {
        let solver = ConstraintSolver::default();

        let props = PhysicalProperties {
            mass_kg: Some(1_000_000_000_000_000_000_000_000_000_000), // Way too massive
            radius_m: Some(1000),                                     // 1 km radius
            ..PhysicalProperties::default()
        };

        let body = CelestialBody::new(
            BodyId::new("impossible").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = solver.check_cross_field_consistency(&definition);

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| {
            matches!(d.code, DiagnosticCode::ContradictoryConstraints)
                && d.message.contains("exceeds physical limits")
        }));
    }

    #[test]
    fn test_impossibly_light_planet_warned() {
        let solver = ConstraintSolver::default();

        let props = PhysicalProperties {
            mass_kg: Some(1),          // 1 kg
            radius_m: Some(1_000_000), // 1000 km radius
            ..PhysicalProperties::default()
        };

        let body = CelestialBody::new(
            BodyId::new("too-light").unwrap(),
            props,
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = solver.check_cross_field_consistency(&definition);

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| {
            matches!(d.code, DiagnosticCode::ContradictoryConstraints)
                && d.message.contains("extremely low")
        }));
    }
}
