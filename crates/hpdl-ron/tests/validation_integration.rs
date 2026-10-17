//! Integration tests: RON fixtures parse through `RonLoader` and validate through
//! the shared `hpdl-sema` pipeline via `ParsedDefinition::validate`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use hpdl_ron::{RonLoader, SourceDocument};

fn fixture(name: &str) -> SourceDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    SourceDocument::from_string(name, content)
}

#[test]
fn minimal_fixture_parses_and_validates() {
    let loader = RonLoader::new();
    let parsed = loader.parse(&fixture("minimal.ron")).expect("parse");
    assert_eq!(parsed.definition().body().id().as_str(), "proxima-b");
    assert!(parsed.validate().is_ok(), "minimal fixture should validate");
}

#[test]
fn earth_like_fixture_parses_and_validates() {
    let loader = RonLoader::new();
    let parsed = loader.parse(&fixture("earth_like.ron")).expect("parse");
    let body = parsed.definition().body();
    assert_eq!(body.id().as_str(), "terra-nova");
    assert_eq!(body.statuses().len(), 1);
    assert_eq!(body.actions().len(), 1);
    assert_eq!(body.modules().len(), 1);
    assert!(
        parsed.validate().is_ok(),
        "earth-like fixture should validate"
    );
}

#[test]
fn gas_giant_fixture_parses_and_validates() {
    let loader = RonLoader::new();
    let parsed = loader.parse(&fixture("gas_giant.ron")).expect("parse");
    assert_eq!(parsed.definition().body().id().as_str(), "jove-prime");
    assert!(parsed.validate().is_ok());
}

#[test]
fn dwarf_planet_fixture_parses_and_validates() {
    let loader = RonLoader::new();
    let parsed = loader.parse(&fixture("dwarf_planet.ron")).expect("parse");
    let body = parsed.definition().body();
    assert_eq!(body.id().as_str(), "ceres-analog");
    // Retrograde rotation is preserved as a negative sidereal period.
    assert_eq!(body.properties().rotation_period_s, Some(-32667));
    assert_eq!(body.migrations().len(), 1);
    assert!(parsed.validate().is_ok());
}

#[test]
fn zero_mass_fixture_parses_but_fails_validation() {
    let loader = RonLoader::new();
    // Structurally valid RON — parsing succeeds.
    let parsed = loader
        .parse(&fixture("invalid_zero_mass.ron"))
        .expect("zero-mass fixture should still parse");
    // But semantic validation rejects it.
    let result = parsed.validate();
    assert!(result.is_err(), "zero mass must fail semantic validation");
    let msg = format!("{}", result.err().unwrap());
    assert!(
        msg.contains("E2001") || msg.to_lowercase().contains("mass"),
        "error should mention the mass violation, got: {msg}"
    );
}
