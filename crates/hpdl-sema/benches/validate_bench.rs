//! Dependency-free semantic-validation benchmark for `hpdl-sema`.
//!
//! `harness = false`: uses `std::time::Instant`, no external crates. Reports
//! time-per-validation across input sizes so super-linear checks (e.g. an
//! accidental actions × writes blowup) would show up as a worsening shape.
//!
//! Run with: `cargo bench -p hpdl-sema`
//!
//! Indicative only; absolute numbers vary by machine.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use hpdl_model::{
    ActionDeclaration, BodyId, CelestialBody, PhysicalProperties, PlanetDefinition,
    StatusDeclaration, TraitSet,
};
use hpdl_sema::{Validator, ValidatorConfig};

/// Build a valid definition with `n` statuses and `n` actions.
fn definition_with_n(n: usize) -> PlanetDefinition {
    let statuses: Vec<_> = (0..n)
        .map(|i| StatusDeclaration::new(format!("status_{i}"), 0, 100).unwrap())
        .collect();
    let actions: Vec<_> = (0..n)
        .map(|i| {
            ActionDeclaration::new(format!("action_{i}"), ["state.a", "state.b"], ["state.a"])
                .unwrap()
        })
        .collect();

    let body = CelestialBody::new(
        BodyId::new("bench").unwrap(),
        PhysicalProperties {
            mass_kg: Some(5_972_000_000_000_000_000_000_000),
            radius_m: Some(6_371_000),
            ..PhysicalProperties::default()
        },
        TraitSet::empty(),
        vec![],
        statuses,
        actions,
        vec![],
    )
    .unwrap();

    PlanetDefinition::new("1.0", body).unwrap()
}

fn bench_size(n: usize, iters: usize) {
    // Use a config whose collection limit is high enough that the input is valid.
    let validator = Validator::new(ValidatorConfig::default().with_max_collection_size(100_000));
    let def = definition_with_n(n);

    // Warm up + assert it validates (benchmark the success path).
    validator
        .validate(def.clone())
        .expect("benchmark definition must validate");

    let start = Instant::now();
    for _ in 0..iters {
        let result = validator.validate(def.clone());
        std::hint::black_box(&result);
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iters as u32;

    println!("validate  n={n:>5} (statuses+actions)  iters={iters:>5}  {per_iter:>12?}/iter");
}

fn main() {
    println!("hpdl-sema validate benchmark (dependency-free, indicative only)");
    bench_size(1, 50_000);
    bench_size(50, 5_000);
    bench_size(500, 500);
    bench_size(5_000, 50);
}
