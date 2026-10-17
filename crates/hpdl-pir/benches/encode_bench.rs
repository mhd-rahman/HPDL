//! Dependency-free canonical-encode + hash benchmark for `hpdl-pir`.
//!
//! `harness = false`: uses `std::time::Instant`, no external crates. This is the
//! determinism-critical path (bincode encode + domain-separated SHA-256), so a
//! regression here matters most.
//!
//! Run with: `cargo bench -p hpdl-pir`
//!
//! Indicative only; absolute numbers vary by machine.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use hpdl_model::{
    BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, StatusDeclaration, TraitSet,
};
use hpdl_pir::encode_planet_definition;

/// Build a definition with `n` status declarations to grow the payload size.
fn definition_with_n(n: usize) -> PlanetDefinition {
    let statuses: Vec<_> = (0..n)
        .map(|i| StatusDeclaration::new(format!("status_{i}"), 0, 100).unwrap())
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
        vec![],
        vec![],
    )
    .unwrap();

    PlanetDefinition::new("1.0", body).unwrap()
}

fn bench_size(n: usize, iters: usize) {
    let def = definition_with_n(n);
    let payload_bytes = encode_planet_definition(&def)
        .expect("benchmark definition must encode")
        .payload
        .len();

    let start = Instant::now();
    for _ in 0..iters {
        let env = encode_planet_definition(&def).expect("encode");
        std::hint::black_box(&env);
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iters as u32;

    println!(
        "encode+hash  n={n:>5}  payload={payload_bytes:>8}B  iters={iters:>5}  {per_iter:>12?}/iter"
    );
}

fn main() {
    println!("hpdl-pir encode+hash benchmark (dependency-free, indicative only)");
    bench_size(1, 50_000);
    bench_size(50, 10_000);
    bench_size(500, 1_000);
    bench_size(5_000, 100);
}
