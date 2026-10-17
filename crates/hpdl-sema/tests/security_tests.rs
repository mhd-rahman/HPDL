//! Security hardening tests: resource limits and no-panic guarantees.
//!
//! The validator must fail closed on hostile input (oversized collections,
//! over-long strings, deep nesting budgets) with typed diagnostics, never a
//! panic or unbounded work.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use hpdl_model::{
    ActionDeclaration, BodyId, CelestialBody, PhysicalProperties, PlanetDefinition,
    StatusDeclaration, TraitSet,
};
use hpdl_sema::{DiagnosticCode, Validator, ValidatorConfig};

fn def_with(statuses: Vec<StatusDeclaration>, actions: Vec<ActionDeclaration>) -> PlanetDefinition {
    let body = CelestialBody::new(
        BodyId::new("subject").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        statuses,
        actions,
        vec![],
    )
    .unwrap();
    PlanetDefinition::new("1.0", body).unwrap()
}

#[test]
fn oversized_status_collection_hits_limit() {
    let v = Validator::new(ValidatorConfig::default().with_max_collection_size(4));
    let statuses: Vec<_> = (0..10)
        .map(|i| StatusDeclaration::new(format!("s{i}"), 0, 100).unwrap())
        .collect();

    let diags = v
        .validate(def_with(statuses, vec![]))
        .expect_err("oversized collection must fail");
    assert!(diags
        .iter()
        .any(|d| d.code == DiagnosticCode::MaxCollectionSizeExceeded));
}

#[test]
fn oversized_action_collection_hits_limit() {
    let v = Validator::new(ValidatorConfig::default().with_max_collection_size(2));
    let actions: Vec<_> = (0..6)
        .map(|i| ActionDeclaration::new(format!("a{i}"), [] as [&str; 0], [] as [&str; 0]).unwrap())
        .collect();

    let diags = v
        .validate(def_with(vec![], actions))
        .expect_err("oversized action collection must fail");
    assert!(diags
        .iter()
        .any(|d| d.code == DiagnosticCode::MaxCollectionSizeExceeded));
}

#[test]
fn overlong_body_id_hits_string_limit() {
    let v = Validator::new(ValidatorConfig::default().with_max_string_length(8));
    // BodyId itself allows up to 128 chars; the validator's string limit is stricter.
    let long_id = "a".repeat(64);
    let body = CelestialBody::new(
        BodyId::new(long_id).unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    let diags = Validator::new(v_cfg(8))
        .validate(PlanetDefinition::new("1.0", body).unwrap())
        .expect_err("overlong id must fail");
    assert!(diags
        .iter()
        .any(|d| d.code == DiagnosticCode::MaxStringLengthExceeded));
    let _ = v; // config reused via helper
}

fn v_cfg(max_string: usize) -> ValidatorConfig {
    ValidatorConfig::default().with_max_string_length(max_string)
}

#[test]
fn large_but_within_limit_input_does_not_panic_and_passes() {
    // A large (but bounded) number of valid, uniquely-named statuses must be
    // handled without panicking and without tripping the default limit.
    let statuses: Vec<_> = (0..2_000)
        .map(|i| StatusDeclaration::new(format!("status_{i}"), 0, 100).unwrap())
        .collect();

    let result = Validator::default_config().validate(def_with(statuses, vec![]));
    assert!(
        result.is_ok(),
        "2000 valid statuses should validate cleanly"
    );
}

#[test]
fn validation_never_panics_on_extreme_physical_values() {
    // Maximum-magnitude integers must be handled with checked arithmetic, no panic.
    let props = PhysicalProperties {
        mass_kg: Some(u128::MAX),
        radius_m: Some(u128::MAX),
        surface_gravity_mms2: Some(u64::MAX),
        orbital_period_s: Some(u64::MAX),
        rotation_period_s: Some(i64::MIN),
    };
    let body = CelestialBody::new(
        BodyId::new("extreme").unwrap(),
        props,
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();

    // We only assert that validation returns (Ok or Err) without panicking.
    let _ = Validator::default_config().validate(PlanetDefinition::new("1.0", body).unwrap());
}
