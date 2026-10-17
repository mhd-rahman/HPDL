//! Security hardening tests for the RON frontend.
//!
//! The loader enforces a file-size limit on the parse path and exposes
//! `SecurityConfig` validators for nesting depth and collection length. These
//! tests verify the enforced limits fail closed with typed errors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use hpdl_ron::{Error, RonLoader, SecurityConfig, SourceDocument};

const MINIMAL: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("test"),
        properties: PhysicalProperties(
            mass_kg: None, radius_m: None, surface_gravity_mms2: None,
            orbital_period_s: None, rotation_period_s: None,
        ),
        traits: TraitSet([]),
        modules: [], statuses: [], actions: [], migrations: [],
    )
)
"#;

#[test]
fn oversized_file_is_rejected_on_parse() {
    let config = SecurityConfig::builder().max_file_size(64).build();
    let loader = RonLoader::with_config(config);
    let source = SourceDocument::from_string("big.ron", MINIMAL.to_string());

    let result = loader.parse(&source);
    assert!(matches!(result, Err(Error::FileSizeExceeded { .. })));
}

#[test]
fn within_size_limit_parses() {
    let config = SecurityConfig::builder().max_file_size(10_000).build();
    let loader = RonLoader::with_config(config);
    let source = SourceDocument::from_string("ok.ron", MINIMAL.to_string());
    assert!(loader.parse(&source).is_ok());
}

#[test]
fn nesting_depth_validator_fails_closed() {
    let config = SecurityConfig::new();
    assert!(config
        .validate_nesting_depth(config.max_nesting_depth())
        .is_ok());
    assert!(matches!(
        config.validate_nesting_depth(config.max_nesting_depth() + 1),
        Err(Error::NestingDepthExceeded { .. })
    ));
}

#[test]
fn collection_length_validator_fails_closed() {
    let config = SecurityConfig::new();
    assert!(config
        .validate_collection_length(config.max_collection_length())
        .is_ok());
    assert!(matches!(
        config.validate_collection_length(config.max_collection_length() + 1),
        Err(Error::CollectionLengthExceeded { .. })
    ));
}

#[test]
fn large_but_bounded_collection_parses_without_panic() {
    // A traits list with many (but under-limit) entries must parse without panic.
    let mut traits = String::new();
    for i in 0..500 {
        if i > 0 {
            traits.push_str(", ");
        }
        traits.push_str(&format!("\"trait-{i}\""));
    }
    let ron = format!(
        r#"PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("many-traits"),
                properties: PhysicalProperties(
                    mass_kg: None, radius_m: None, surface_gravity_mms2: None,
                    orbital_period_s: None, rotation_period_s: None,
                ),
                traits: TraitSet([{traits}]),
                modules: [], statuses: [], actions: [], migrations: [],
            )
        )"#
    );

    let loader = RonLoader::new();
    let source = SourceDocument::from_string("many.ron", ron);
    // Parsing must not panic; result may be Ok or a typed Err.
    let _ = loader.parse(&source);
}
