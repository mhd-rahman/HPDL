//! Demonstrates how the validator reports typed, stable diagnostics for a
//! definition that violates several rules at once.
//!
//! Run with: `cargo run -p hpdl-sema --example sema_error_handling`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hpdl_model::{
    BodyId, CelestialBody, ParentRef, PhysicalProperties, PlanetDefinition, ReferenceFrame,
    TraitSet,
};
use hpdl_sema::{Severity, Validator};

fn main() {
    // This definition has two independent problems:
    //   1. zero mass (NegativeMass, E2001)
    //   2. a self-parent (ParentCycle, E8001)
    let body = CelestialBody::new(
        BodyId::new("broken").unwrap(),
        PhysicalProperties {
            mass_kg: Some(0),
            ..PhysicalProperties::default()
        },
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .with_parent(ParentRef::new(
        BodyId::new("broken").unwrap(),
        ReferenceFrame::EclipticJ2000,
    ));

    let definition = PlanetDefinition::new("1.0", body).unwrap();

    match Validator::default_config().validate(definition) {
        Ok(_) => println!("Unexpectedly validated."),
        Err(diagnostics) => {
            let errors = diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count();
            println!("Found {errors} error(s). Diagnostics (sorted, stable):");
            for d in &diagnostics {
                println!("  [{}] {}", d.code, d.message);
                for note in &d.notes {
                    println!("      note: {note}");
                }
            }
        }
    }
}
