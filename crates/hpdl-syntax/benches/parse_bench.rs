//! Dependency-free parsing benchmark for `hpdl-syntax`.
//!
//! This is a `harness = false` benchmark: it uses `std::time::Instant` directly
//! so it adds no external crates (no criterion). It reports **throughput at
//! several input sizes** so a reader can see the scaling shape (linear vs.
//! super-linear), which is more meaningful than a single machine-specific
//! millisecond number.
//!
//! Run with: `cargo bench -p hpdl-syntax`
//!
//! Note: numbers are indicative only and vary by machine; treat regressions in
//! the *shape* across sizes as the signal, not absolute values.

use std::time::Instant;

use hpdl_syntax::{parse_file, SourceFile};

/// Build an HPDL source whose `traits.tags` list has `n` entries, so input size
/// scales roughly linearly with `n`.
fn source_with_n_tags(n: usize) -> String {
    let mut tags = String::new();
    for i in 0..n {
        if i > 0 {
            tags.push_str(", ");
        }
        tags.push_str(&format!("\"trait-{i}\""));
    }
    format!(
        r#"language hpdl "1.0";
planet bench {{
    id: "bench-planet",
    physical: {{
        mass_kg: 5972000000000000000000000,
        radius_m: 6371000,
    }},
    traits: {{ tags: [{tags}] }},
}}"#
    )
}

fn bench_size(n: usize, iters: usize) {
    let src = source_with_n_tags(n);
    let bytes = src.len();
    let source = SourceFile::from_string("bench.hpdl", src);

    // Warm up (and assert it actually parses, so we don't benchmark an error path).
    parse_file(&source).expect("benchmark source must parse");

    let start = Instant::now();
    for _ in 0..iters {
        let def = parse_file(&source).expect("parse");
        // Prevent the optimizer from eliding the work.
        std::hint::black_box(&def);
    }
    let elapsed = start.elapsed();

    let per_iter = elapsed / iters as u32;
    let mib_per_s = if elapsed.as_secs_f64() > 0.0 {
        (bytes as f64 * iters as f64) / elapsed.as_secs_f64() / (1024.0 * 1024.0)
    } else {
        f64::INFINITY
    };

    println!(
        "parse  tags={n:>5}  src={bytes:>8}B  iters={iters:>5}  {per_iter:>12?}/iter  ~{mib_per_s:>8.1} MiB/s"
    );
}

fn main() {
    println!("hpdl-syntax parse benchmark (dependency-free, indicative only)");
    bench_size(1, 20_000);
    bench_size(50, 5_000);
    bench_size(500, 500);
    bench_size(5_000, 50);
}
