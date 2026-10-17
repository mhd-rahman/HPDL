//! Dependency-free RON deserialization benchmark for `hpdl-ron`.
//!
//! `harness = false`: uses `std::time::Instant`, no external crates. Reports
//! throughput at several input sizes.
//!
//! Run with: `cargo bench -p hpdl-ron`
//!
//! Indicative only; absolute numbers vary by machine.

use std::time::Instant;

use hpdl_ron::{RonLoader, SourceDocument};

/// Build a RON planet whose `traits` list has `n` entries.
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
    let loader = RonLoader::new();

    // Warm up + assert it parses.
    {
        let source = SourceDocument::from_string("bench.ron", src.clone());
        loader.parse(&source).expect("benchmark source must parse");
    }

    let start = Instant::now();
    for _ in 0..iters {
        let source = SourceDocument::from_string("bench.ron", src.clone());
        let parsed = loader.parse(&source).expect("parse");
        std::hint::black_box(&parsed);
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iters as u32;
    let mib_per_s = if elapsed.as_secs_f64() > 0.0 {
        (bytes as f64 * iters as f64) / elapsed.as_secs_f64() / (1024.0 * 1024.0)
    } else {
        f64::INFINITY
    };

    println!(
        "ron-parse  tags={n:>5}  src={bytes:>8}B  iters={iters:>5}  {per_iter:>12?}/iter  ~{mib_per_s:>8.1} MiB/s"
    );
}

fn main() {
    println!("hpdl-ron parse benchmark (dependency-free, indicative only)");
    bench_size(1, 20_000);
    bench_size(50, 5_000);
    bench_size(500, 500);
    bench_size(5_000, 50);
}
