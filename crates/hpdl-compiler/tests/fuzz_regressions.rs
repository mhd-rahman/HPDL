//! Fuzz crash-reproducer regression tests.
//!
//! The PIR decoder (`hpdl_pir::decode_planet_definition`) and the RON loader
//! (`hpdl_ron::RonLoader::parse`) must be **total** on untrusted input: they may
//! return `Err`, but must never panic, hang, or over-allocate (rust-standards +
//! determinism steering). The `fuzz/` crate searches for inputs that violate
//! that; when it finds one, `cargo fuzz` writes a reproducer under
//! `fuzz/artifacts/<target>/`.
//!
//! This test replays every committed reproducer through the SAME decoder entry
//! point the fuzz target drives, under the normal stable `cargo test --workspace`
//! gate (so the regression is enforced on Linux, macOS, and Windows — not only in
//! the nightly fuzz job). A reproducer that once panicked must now return without
//! panicking; the `Ok`/`Err` outcome is irrelevant, totality is the contract.
//!
//! The test passes vacuously when a target has no committed reproducers yet, so
//! it is green today and automatically covers any crasher added later. Adding a
//! crasher file under `fuzz/artifacts/<target>/` is all that is needed to turn it
//! into an enforced regression — no edit to this file required.

use std::fs;
use std::path::{Path, PathBuf};

use hpdl_ron::{RonLoader, SourceDocument};

/// Resolve `fuzz/artifacts/<target>` relative to the repo root.
///
/// `CARGO_MANIFEST_DIR` for this crate is `<repo>/crates/hpdl-compiler`, so the
/// repo root is two levels up.
fn artifacts_dir(target: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fuzz")
        .join("artifacts")
        .join(target)
}

/// Collect the regular files under `dir`, sorted by path for determinism.
///
/// Returns an empty vector if the directory does not exist (no crashers yet).
fn reproducers(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        // libFuzzer may drop a non-reproducer alongside crashers; skip obvious noise.
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.starts_with('.'))
        })
        .collect();
    files.sort();
    files
}

/// Every committed PIR reproducer must decode without panicking.
#[test]
fn decode_pir_reproducers_do_not_panic() {
    let dir = artifacts_dir("decode_pir");
    let files = reproducers(&dir);
    for path in &files {
        let bytes = fs::read(path).unwrap_or_else(|e| {
            panic!("failed to read PIR reproducer {}: {e}", path.display())
        });
        // Totality contract: returning Err is fine; panicking / hanging is the bug.
        let _ = hpdl_pir::decode_planet_definition(&bytes);
    }
    eprintln!(
        "decode_pir: replayed {} committed reproducer(s) from {}",
        files.len(),
        dir.display()
    );
}

/// Every committed RON reproducer must parse without panicking.
///
/// The fuzz target only feeds valid UTF-8 to the loader (non-UTF-8 inputs are
/// skipped), so a reproducer is always valid UTF-8; mirror that here.
#[test]
fn decode_ron_reproducers_do_not_panic() {
    let dir = artifacts_dir("decode_ron");
    let files = reproducers(&dir);
    for path in &files {
        let bytes = fs::read(path).unwrap_or_else(|e| {
            panic!("failed to read RON reproducer {}: {e}", path.display())
        });
        let Ok(text) = std::str::from_utf8(&bytes) else {
            // A non-UTF-8 artifact cannot be a decode_ron reproducer; skip it
            // rather than assert, matching the fuzz target's own guard.
            continue;
        };
        let source = SourceDocument::from_string("fuzz-regression".to_string(), text.to_string());
        let loader = RonLoader::new();
        // Totality contract: Err is fine; panicking / hanging is the bug.
        let _ = loader.parse(&source);
    }
    eprintln!(
        "decode_ron: replayed {} committed reproducer(s) from {}",
        files.len(),
        dir.display()
    );
}
