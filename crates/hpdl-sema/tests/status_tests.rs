//! Status threshold validation tests (Amendment A1), including golden fixtures.
//!
//! These tests drive the `StatusValidator::validate_status_thresholds` API from
//! frozen JSON fixtures under `tests/golden/` so the Amendment A1 behaviour is
//! documented as data, not just code.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::path::Path;

use hpdl_sema::status_validator::StatusValidator;
use hpdl_sema::DiagnosticCode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ValidFixture {
    status_name: String,
    scale_min: i64,
    scale_max: i64,
    thresholds: BTreeMap<String, i64>,
}

#[derive(Debug, Deserialize)]
struct InvalidFixture {
    scale_min: i64,
    scale_max: i64,
    cases: Vec<InvalidCase>,
}

#[derive(Debug, Deserialize)]
struct InvalidCase {
    name: String,
    thresholds: BTreeMap<String, i64>,
    expected_code: String,
}

fn golden(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", full.display()))
}

#[test]
fn golden_valid_thresholds_pass() {
    let fixture: ValidFixture =
        serde_json::from_str(&golden("status_thresholds_valid.json")).expect("valid fixture JSON");

    let validator = StatusValidator::new();
    let diagnostics = validator.validate_status_thresholds(
        &fixture.status_name,
        fixture.scale_min,
        fixture.scale_max,
        &fixture.thresholds,
    );

    assert!(
        diagnostics.is_empty(),
        "golden valid fixture should produce no diagnostics, got {diagnostics:?}"
    );
}

#[test]
fn golden_invalid_thresholds_rejected_with_expected_codes() {
    let fixture: InvalidFixture = serde_json::from_str(&golden("status_thresholds_invalid.json"))
        .expect("invalid fixture JSON");

    let validator = StatusValidator::new();

    for case in &fixture.cases {
        let diagnostics = validator.validate_status_thresholds(
            &case.name,
            fixture.scale_min,
            fixture.scale_max,
            &case.thresholds,
        );

        assert!(
            !diagnostics.is_empty(),
            "case {:?} should produce diagnostics but did not",
            case.name
        );

        let codes: Vec<&str> = diagnostics.iter().map(|d| d.code.code()).collect();
        assert!(
            codes.contains(&case.expected_code.as_str()),
            "case {:?} expected code {} but got {:?}",
            case.name,
            case.expected_code,
            codes
        );
    }
}

/// The last band includes the maximum: a cut point exactly at the scale maximum
/// is still inside the inclusive `[min, max]` range and therefore accepted.
#[test]
fn threshold_at_scale_maximum_is_in_range() {
    let validator = StatusValidator::new();
    let mut thresholds = BTreeMap::new();
    thresholds.insert("a".to_string(), 0);
    thresholds.insert("b".to_string(), 100);

    let diagnostics = validator.validate_status_thresholds("Edge", 0, 100, &thresholds);
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == DiagnosticCode::ThresholdOutOfRange),
        "a cut point at the scale maximum must be in range"
    );
}

/// A single cut point at the scale minimum is a degenerate but valid single band.
#[test]
fn single_threshold_at_min_is_valid() {
    let validator = StatusValidator::new();
    let mut thresholds = BTreeMap::new();
    thresholds.insert("only".to_string(), 0);

    let diagnostics = validator.validate_status_thresholds("Single", 0, 100, &thresholds);
    assert!(
        diagnostics.is_empty(),
        "single band starting at scale min should be valid, got {diagnostics:?}"
    );
}
