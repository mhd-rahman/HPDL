//! Basic usage of the HPDL semantic validator.
//!
//! Run with: `cargo run -p hpdl-sema --example basic_validation`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hpdl_model::{
    BodyId, CelestialBody, CelestialIdentity, PhysicalProperties, PlanetDefinition, TraitSet,
};
use hpdl_sema::{Validator, ValidatorConfig};

fn main() {
    // Build a minimal, valid planet definition.
    let body = CelestialBody::new(
        BodyId::new("kepler-186f").unwrap(),
        PhysicalProperties {
            mass_kg: Some(6_500_000_000_000_000_000_000_000),
            radius_m: Some(7_100_000),
            ..PhysicalProperties::default()
        },
        TraitSet::new(["terrestrial".to_string(), "habitable-zone".to_string()]).unwrap(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .with_identity(CelestialIdentity::Planet);

    let definition = PlanetDefinition::new("1.0", body).unwrap();

    // Validate with the default (strict, deterministic) configuration.
    let validator = Validator::new(ValidatorConfig::default());

    match validator.validate(definition) {
        Ok(validated) => {
            println!(
                "Validation succeeded for {}",
                validated.definition().body().id()
            );
            println!("Applied rules: {:?}", validated.metadata().applied_rules);
        }
        Err(diagnostics) => {
            eprintln!(
                "Validation failed with {} diagnostic(s):",
                diagnostics.len()
            );
            for d in diagnostics {
                eprintln!("  {d}");
            }
        }
    }
}
