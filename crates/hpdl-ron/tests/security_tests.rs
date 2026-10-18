//! Security hardening tests for the RON frontend.
//!
//! The loader enforces file-size, nesting-depth and collection-length limits on
//! the parse path (the depth/length limits via a structural pre-scan before the
//! deserializer runs) and also exposes the underlying `SecurityConfig`
//! validators. These tests verify the limits fail closed with typed errors both
//! in isolation and through the real `RonLoader::parse` entry point.

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
fn deeply_nested_input_is_rejected_on_parse() {
    // Regression for the audit gap: the depth limit must now fire on the real
    // parse path, not only when the validator is called directly.
    let config = SecurityConfig::builder().max_nesting_depth(8).build();
    let loader = RonLoader::with_config(config);

    // Wrap the body value in many nested lists to blow past the depth limit.
    // The structural pre-scan rejects it before the deserializer sees it.
    let open = "[".repeat(64);
    let close = "]".repeat(64);
    let ron = format!(
        r#"PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("deep"),
                properties: {open}{close},
                traits: TraitSet([]),
                modules: [], statuses: [], actions: [], migrations: [],
            )
        )"#
    );
    let source = SourceDocument::from_string("deep.ron", ron);
    assert!(matches!(
        loader.parse(&source),
        Err(Error::NestingDepthExceeded { .. })
    ));
}

#[test]
fn oversized_collection_is_rejected_on_parse() {
    // A list longer than the configured collection limit must be rejected on the
    // parse path via the structural pre-scan.
    let config = SecurityConfig::builder().max_collection_length(16).build();
    let loader = RonLoader::with_config(config);

    let mut elements = String::new();
    for i in 0..64 {
        if i > 0 {
            elements.push_str(", ");
        }
        elements.push_str(&format!("\"t-{i}\""));
    }
    let ron = format!(
        r#"PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("wide"),
                properties: PhysicalProperties(
                    mass_kg: None, radius_m: None, surface_gravity_mms2: None,
                    orbital_period_s: None, rotation_period_s: None,
                ),
                traits: TraitSet([{elements}]),
                modules: [], statuses: [], actions: [], migrations: [],
            )
        )"#
    );
    let source = SourceDocument::from_string("wide.ron", ron);
    assert!(matches!(
        loader.parse(&source),
        Err(Error::CollectionLengthExceeded { .. })
    ));
}

#[test]
fn minimal_passes_structural_limits_on_parse() {
    // The minimal, well-formed document must still parse under default limits:
    // the structural pre-scan must not reject legitimate input.
    let loader = RonLoader::new();
    let source = SourceDocument::from_string("ok.ron", MINIMAL.to_string());
    assert!(loader.parse(&source).is_ok());
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
