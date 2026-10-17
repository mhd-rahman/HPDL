//! Determinism tests: identical input must yield identical, stably-ordered output.
//!
//! The validator must not depend on `HashMap` iteration order, wall-clock,
//! locale, or any ambient state. Repeated runs over the same definition must
//! produce byte-identical diagnostic sequences.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use hpdl_model::{
    BodyId, CelestialBody, ParentRef, PhysicalProperties, PlanetDefinition, ReferenceFrame,
    StatusDeclaration, TraitSet,
};
use hpdl_sema::{Validator, ValidatorConfig};

/// Build a definition that produces several diagnostics of different kinds.
fn multi_error_definition() -> PlanetDefinition {
    let statuses: Vec<_> = (0..6)
        .map(|i| StatusDeclaration::new(format!("s{i}"), 0, 100).unwrap())
        .collect();

    let body = CelestialBody::new(
        BodyId::new("subject").unwrap(),
        PhysicalProperties {
            mass_kg: Some(0),
            ..PhysicalProperties::default()
        },
        TraitSet::empty(),
        vec![],
        statuses,
        vec![],
        vec![],
    )
    .unwrap()
    .with_parent(ParentRef::new(
        BodyId::new("subject").unwrap(),
        ReferenceFrame::EclipticJ2000,
    ));

    PlanetDefinition::new("1.0", body).unwrap()
}

fn render(diags: &[hpdl_sema::Diagnostic]) -> Vec<String> {
    diags.iter().map(|d| d.to_string()).collect()
}

#[test]
fn repeated_validation_is_identical() {
    let v = Validator::new(ValidatorConfig::default().with_max_collection_size(2));

    let first = v
        .validate(multi_error_definition())
        .expect_err("should fail");
    let second = v
        .validate(multi_error_definition())
        .expect_err("should fail");
    let third = v
        .validate(multi_error_definition())
        .expect_err("should fail");

    assert_eq!(render(&first), render(&second));
    assert_eq!(render(&second), render(&third));
}

#[test]
fn diagnostics_are_sorted() {
    let v = Validator::new(ValidatorConfig::default().with_max_collection_size(2));
    let diags = v
        .validate(multi_error_definition())
        .expect_err("should fail");

    for w in diags.windows(2) {
        assert!(w[0] <= w[1], "diagnostics must be in sorted order");
    }
}

#[test]
fn fresh_validators_agree() {
    // Two independently-constructed validators must agree on the same input.
    let a = Validator::default_config();
    let b = Validator::default_config();

    let ra = a.validate(multi_error_definition());
    let rb = b.validate(multi_error_definition());

    match (ra, rb) {
        (Err(da), Err(db)) => assert_eq!(render(&da), render(&db)),
        other => panic!("expected both to fail identically, got {other:?}"),
    }
}
