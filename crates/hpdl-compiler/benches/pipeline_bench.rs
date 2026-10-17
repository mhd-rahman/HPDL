//! Dependency-free end-to-end pipeline benchmark for `hpdl-compiler`.
//!
//! `harness = false`: uses `std::time::Instant`, no external crates. Measures
//! the full `compile_ron_str` path (decode → validate → resolve → lower → emit →
//! manifest), the integration number a user actually feels.
//!
//! Run with: `cargo bench -p hpdl-compiler`
//!
//! Indicative only; absolute numbers vary by machine.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Instant;

use hpdl_compiler::{CompileOptions, Compiler};

fn source_with_n_tags(n: usize) -> String {
    let mut tags = String::new();
    for i in 0..n {
        if i > 0 {
            tags.push_str(", ");
        }
        tags.push_str(&format!("\"trait-{i}\""));
    }
    format!(
        r#"PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("bench-planet"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),
            radius_m: Some(6371000),
            surface_gravity_mms2: None,
            orbital_period_s: None,
            rotation_period_s: None,
        ),
        traits: TraitSet([{tags}]),
        modules: [],
        statuses: [],
        actions: [],
        migrations: [],
    )
)"#
    )
}

fn bench_size(n: usize, iters: usize) {
    let src = source_with_n_tags(n);
    let bytes = src.len();

    // Warm up + assert the pipeline succeeds (benchmark the success path).
    {
        let compiler = Compiler::new(CompileOptions::default());
        compiler
            .compile_ron_str("bench", &src)
            .expect("benchmark source must compile");
    }

    let start = Instant::now();
    for _ in 0..iters {
        let compiler = Compiler::new(CompileOptions::default());
        let result = compiler.compile_ron_str("bench", &src).expect("compile");
        std::hint::black_box(&result);
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iters as u32;

    println!("compile  tags={n:>5}  src={bytes:>8}B  iters={iters:>5}  {per_iter:>12?}/iter");
}

fn main() {
    println!("hpdl-compiler full-pipeline benchmark (dependency-free, indicative only)");
    bench_size(1, 10_000);
    bench_size(50, 2_000);
    bench_size(500, 200);
    bench_size(5_000, 20);
}
