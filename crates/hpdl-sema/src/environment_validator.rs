//! Planetary environment validation (HPDL feature 08).
//!
//! This pass is defensive and deterministic. The `hpdl-model` constructors
//! already enforce the numeric contracts, so these checks are defence in depth;
//! they never panic and never perform floating-point arithmetic.
//!
//! It enforces:
//!
//! 1. **Composition bounds** — every component `<= 1_000_000` ppm and the sum
//!    within the declared tolerance ([`DiagnosticCode::CompositionOutOfBounds`]).
//! 2. **Atmosphere / thermal / coverage values** — positive pressure, ordered
//!    temperatures, in-range coverage ([`DiagnosticCode::InvalidEnvironmentValue`]).
//! 3. **Unsupported climate model** — a declared `climate_model` is a contract
//!    reference with no registered implementation yet, so it is flagged
//!    ([`DiagnosticCode::UnsupportedEnvironmentModule`]); accepted as a
//!    declaration, never simulated.
//! 4. **Asset digests** — each digest is a 64-char lowercase hex string
//!    ([`DiagnosticCode::MissingAssetDigest`]).
//!
//! A biosphere of `Absent` or `Unknown` is a valid, first-class value and
//! produces **no** diagnostic — absence is never treated as an error or a zero.

use hpdl_model::{EnvironmentDefinition, PlanetDefinition, PPM_SCALE};

use crate::{Diagnostic, DiagnosticCode};

/// Validates a body's declared environment.
pub struct EnvironmentValidator;

impl EnvironmentValidator {
    /// Run all environment checks for a definition.
    pub fn validate(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if let Some(env) = definition.body().environment() {
            self.check_environment(env, &mut diagnostics);
        }
        diagnostics
    }

    fn check_environment(&self, env: &EnvironmentDefinition, diagnostics: &mut Vec<Diagnostic>) {
        if let Some(atm) = env.atmosphere() {
            // Composition bounds (defensive re-check).
            let composition = atm.composition();
            for (gas, ppm) in composition.components() {
                if ppm > PPM_SCALE {
                    diagnostics.push(Diagnostic::error(
                        DiagnosticCode::CompositionOutOfBounds,
                        format!(
                            "gas {:?} fraction {} ppm exceeds 1_000_000 (100%)",
                            gas.as_str(),
                            ppm
                        ),
                    ));
                }
            }
            let sum = composition.sum_ppm();
            let tol = u64::from(composition.tolerance_ppm());
            let target = u64::from(PPM_SCALE);
            let low = target.saturating_sub(tol);
            let high = target.saturating_add(tol);
            if sum < low || sum > high {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::CompositionOutOfBounds,
                        format!(
                            "composition sum {} ppm is outside 1_000_000 ± {} ppm",
                            sum,
                            composition.tolerance_ppm()
                        ),
                    )
                    .with_note(
                        "gas mole fractions must normalize to 100% within tolerance".to_string(),
                    ),
                );
            }

            // Pressure.
            if atm.surface_pressure_pa() == 0 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidEnvironmentValue,
                    "atmosphere surface pressure must be strictly positive".to_string(),
                ));
            }

            // Climate model is a non-executable contract reference (no registry yet).
            if let Some(model) = atm.climate_model() {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::UnsupportedEnvironmentModule,
                        format!(
                            "climate model {:?} v{:?} has no registered implementation and is not executable",
                            model.name(),
                            model.version()
                        ),
                    )
                    .with_note(
                        "climate models are accepted as declarations but are not simulated"
                            .to_string(),
                    ),
                );
            }
        }

        if let Some(thermal) = env.thermal() {
            if !(thermal.min_mk() <= thermal.mean_mk() && thermal.mean_mk() <= thermal.max_mk()) {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidEnvironmentValue,
                    format!(
                        "thermal profile requires min ({}) <= mean ({}) <= max ({}) milli-kelvin",
                        thermal.min_mk(),
                        thermal.mean_mk(),
                        thermal.max_mk()
                    ),
                ));
            }
        }

        if let Some(hydro) = env.hydrosphere() {
            if hydro.surface_coverage_ppm() > PPM_SCALE {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidEnvironmentValue,
                    format!(
                        "hydrosphere coverage {} ppm exceeds 1_000_000",
                        hydro.surface_coverage_ppm()
                    ),
                ));
            }
        }

        if let Some(cryo) = env.cryosphere() {
            if cryo.surface_coverage_ppm() > PPM_SCALE {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::InvalidEnvironmentValue,
                    format!(
                        "cryosphere coverage {} ppm exceeds 1_000_000",
                        cryo.surface_coverage_ppm()
                    ),
                ));
            }
        }

        // Asset digests must be 64-char lowercase hex.
        for asset in env.assets() {
            let digest = asset.digest().as_str();
            let valid = digest.len() == 64
                && digest
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
            if !valid {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::MissingAssetDigest,
                    format!(
                        "asset {:?} has a missing or malformed content digest",
                        asset.name()
                    ),
                ));
            }
        }

        // Biosphere: Absent / Unknown / Present are all valid; no diagnostic.
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{
        AssetKind, AssetRef, AtmosphereProfile, Biosphere, BodyId, CelestialBody, ComplexityTier,
        Composition, Cryosphere, Digest, EnvironmentDefinition, GasId, Hydrosphere, ModuleRef,
        PhysicalProperties, PlanetDefinition, ThermalProfile, TraitSet,
    };

    fn body() -> CelestialBody {
        CelestialBody::new(
            BodyId::new("subject").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
    }

    fn def(env: EnvironmentDefinition) -> PlanetDefinition {
        PlanetDefinition::new("1.0", body().with_environment(env)).unwrap()
    }

    fn n2_atmosphere() -> AtmosphereProfile {
        let comp = Composition::new([(GasId::new("n2").unwrap(), 1_000_000)], 0).unwrap();
        AtmosphereProfile::new(101_325, comp, None).unwrap()
    }

    #[test]
    fn fully_valid_environment_passes() {
        let env = EnvironmentDefinition::new(Biosphere::Present(ComplexityTier::Microbial))
            .with_atmosphere(n2_atmosphere())
            .with_thermal(ThermalProfile::new(288_000, 184_000, 331_000).unwrap())
            .with_hydrosphere(Hydrosphere::new(708_000, None).unwrap())
            .with_cryosphere(Cryosphere::new(100_000).unwrap())
            .with_asset(
                AssetRef::new(
                    "albedo",
                    AssetKind::Texture,
                    Digest::new("a".repeat(64)).unwrap(),
                )
                .unwrap(),
            );
        let diagnostics = EnvironmentValidator.validate(&def(env));
        assert!(
            diagnostics.is_empty(),
            "expected clean, got {diagnostics:?}"
        );
    }

    #[test]
    fn absent_biosphere_produces_no_diagnostic() {
        let env = EnvironmentDefinition::new(Biosphere::Absent);
        assert!(EnvironmentValidator.validate(&def(env)).is_empty());
        let env2 = EnvironmentDefinition::new(Biosphere::Unknown);
        assert!(EnvironmentValidator.validate(&def(env2)).is_empty());
    }

    #[test]
    fn no_environment_produces_no_diagnostic() {
        let d = PlanetDefinition::new("1.0", body()).unwrap();
        assert!(EnvironmentValidator.validate(&d).is_empty());
    }

    #[test]
    fn declared_climate_model_is_unsupported() {
        let comp = Composition::new([(GasId::new("co2").unwrap(), 1_000_000)], 0).unwrap();
        let atm = AtmosphereProfile::new(
            610,
            comp,
            Some(ModuleRef::new("hpdl.climate.gcm", "1.0").unwrap()),
        )
        .unwrap();
        let env = EnvironmentDefinition::new(Biosphere::Absent).with_atmosphere(atm);
        let diagnostics = EnvironmentValidator.validate(&def(env));
        assert!(diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedEnvironmentModule));
    }
}
