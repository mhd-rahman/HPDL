//! Integration tests for the `hpdl` CLI via the testable `dispatch` entry point,
//! plus one smoke test spawning the built binary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use hpdl_cli::dispatch;
use hpdl_cli::exit::{FAILURE, OK, USAGE};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique scratch path in the OS temp dir (no third-party `tempfile` dep).
fn scratch_path(suffix: &str) -> std::path::PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("hpdl-cli-test-{pid}-{n}-{suffix}"))
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

const MINIMAL: &str = "minimal.ron";

#[test]
fn no_args_prints_help_ok() {
    let out = dispatch(&[]);
    assert_eq!(out.code, OK);
    assert!(out.stdout.contains("hpdl — HPDL developer CLI"));
}

#[test]
fn version_prints_crate_version() {
    let out = dispatch(&args(&["--version"]));
    assert_eq!(out.code, OK);
    assert!(out.stdout.starts_with("hpdl "));
}

#[test]
fn unknown_command_is_usage_error() {
    let out = dispatch(&args(&["frobnicate", "x.ron"]));
    assert_eq!(out.code, USAGE);
    assert!(out.stderr.contains("unknown command"));
}

#[test]
fn unknown_flag_is_usage_error() {
    let out = dispatch(&args(&["check", "x.ron", "--nope"]));
    assert_eq!(out.code, USAGE);
}

#[test]
fn missing_file_is_usage_error() {
    let out = dispatch(&args(&["check"]));
    assert_eq!(out.code, USAGE);
}

#[test]
fn unreadable_file_is_usage_error() {
    let out = dispatch(&args(&["check", "does-not-exist.ron"]));
    assert_eq!(out.code, USAGE);
    assert!(out.stderr.contains("cannot read"));
}

#[test]
fn check_minimal_succeeds() {
    let out = dispatch(&args(&["check", &fixture(MINIMAL)]));
    assert_eq!(out.code, OK, "stderr: {}", out.stderr);
    assert!(out.stdout.starts_with("ok: "));
}

#[test]
fn check_malformed_fails_with_diagnostics() {
    let path = scratch_path("bad.ron");
    fs::write(&path, "this is not ron {{{").unwrap();
    let out = dispatch(&args(&["check", path.to_str().unwrap()]));
    let _ = fs::remove_file(&path);
    assert_eq!(out.code, FAILURE);
    assert!(out.stderr.contains("error:"));
}

#[test]
fn build_text_prints_manifest() {
    let out = dispatch(&args(&["build", &fixture(MINIMAL)]));
    assert_eq!(out.code, OK, "stderr: {}", out.stderr);
    assert!(out.stdout.contains("source_name = minimal"));
    assert!(out.stdout.contains("pir_version = 2.0"));
    assert!(out.stdout.contains("content_hash = "));
}

#[test]
fn build_json_is_machine_readable() {
    let out = dispatch(&args(&["build", &fixture(MINIMAL), "--format", "json"]));
    assert_eq!(out.code, OK);
    assert!(out.stdout.contains("\"source_name\":\"minimal\""));
    assert!(out.stdout.contains("\"pir_version\":\"2.0\""));
    assert!(out.stdout.contains("\"content_hash\":\""));
}

#[test]
fn inspect_summary_has_expected_lines() {
    let out = dispatch(&args(&["inspect", &fixture(MINIMAL)]));
    assert_eq!(out.code, OK, "stderr: {}", out.stderr);
    assert!(out.stdout.contains("source:          minimal"));
    assert!(out.stdout.contains("pir_version:     2.0"));
    assert!(out.stdout.contains("content_hash:    "));
    assert!(out.stdout.contains("modules:         (none)"));
}

#[test]
fn export_to_file_writes_pir_bytes() {
    let out_path = scratch_path("planet.pir");
    let out = dispatch(&args(&[
        "export",
        &fixture(MINIMAL),
        "--out",
        out_path.to_str().unwrap(),
    ]));
    assert_eq!(out.code, OK, "stderr: {}", out.stderr);
    let bytes = fs::read(&out_path).unwrap();
    let _ = fs::remove_file(&out_path);
    assert!(!bytes.is_empty(), "PIR payload must be non-empty");
    assert!(out.stdout.contains("wrote"));
}

#[test]
fn export_without_out_prints_hex() {
    let out = dispatch(&args(&["export", &fixture(MINIMAL)]));
    assert_eq!(out.code, OK);
    let hex = out.stdout.trim();
    assert!(!hex.is_empty());
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn invalid_lock_is_usage_error() {
    let lock = scratch_path("bad.lock");
    fs::write(&lock, "not a lock").unwrap();
    let out = dispatch(&args(&[
        "build",
        &fixture(MINIMAL),
        "--lock",
        lock.to_str().unwrap(),
    ]));
    let _ = fs::remove_file(&lock);
    assert_eq!(out.code, USAGE);
    assert!(out.stderr.contains("invalid lockfile"));
}

#[test]
fn inspect_golden_matches_stable_lines() {
    // The golden fixture captures the stable (hash-independent) lines of the
    // inspect summary; the content hash line varies and is checked separately.
    let out = dispatch(&args(&["inspect", &fixture(MINIMAL)]));
    assert_eq!(out.code, OK);
    let golden = include_str!("golden/inspect_minimal.txt");
    let actual_stable: Vec<&str> = out
        .stdout
        .lines()
        .filter(|l| !l.starts_with("content_hash:"))
        .collect();
    let golden_stable: Vec<&str> = golden
        .lines()
        .filter(|l| !l.starts_with("content_hash:"))
        .collect();
    assert_eq!(actual_stable, golden_stable);
}

#[test]
fn binary_smoke_test_runs_and_exits_cleanly() {
    // Spawn the real built binary to confirm end-to-end wiring + exit code.
    let output = Command::new(env!("CARGO_BIN_EXE_hpdl"))
        .args(["check", &fixture(MINIMAL)])
        .output()
        .expect("spawn hpdl");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("ok: "));
}
