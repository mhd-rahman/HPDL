//! Malformed-input tests: the RON frontend must never panic on garbage and must
//! return typed errors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use hpdl_ron::{Error, RonLoader, SourceDocument};

fn try_parse(content: &str) -> Result<(), Error> {
    let loader = RonLoader::new();
    let source = SourceDocument::from_string("malformed.ron", content.to_string());
    loader.parse(&source).map(|_| ())
}

#[test]
fn unbalanced_braces_is_parse_error() {
    let result = try_parse("PlanetDefinition(((( not closed");
    assert!(matches!(
        result,
        Err(Error::ParseWithSpan { .. }) | Err(Error::Parse(_))
    ));
}

#[test]
fn empty_input_is_parse_error_not_panic() {
    let result = try_parse("");
    assert!(result.is_err());
}

#[test]
fn wrong_shape_is_parse_error() {
    // Valid RON, but not a PlanetDefinition.
    let result = try_parse("SomethingElse(foo: 1)");
    assert!(result.is_err());
}

#[test]
fn truncated_definition_is_parse_error() {
    let result = try_parse(
        r#"PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("x"),
        "#,
    );
    assert!(result.is_err());
}

#[test]
fn garbage_bytes_do_not_panic() {
    // A spray of punctuation and control-ish characters.
    for junk in [
        "{{{}}}",
        "!@#$%^&*()",
        "PlanetDefinition",
        "(:,)",
        "\"\"\"\"",
    ] {
        let _ = try_parse(junk); // must return, never panic
    }
}

#[test]
fn invalid_field_value_is_error() {
    // mass_kg expects an unsigned integer inside Some(...); a string is invalid.
    let result = try_parse(
        r#"PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("x"),
                properties: PhysicalProperties(
                    mass_kg: Some("not a number"),
                    radius_m: None, surface_gravity_mms2: None,
                    orbital_period_s: None, rotation_period_s: None,
                ),
                traits: TraitSet([]),
                modules: [], statuses: [], actions: [], migrations: [],
            )
        )"#,
    );
    assert!(result.is_err());
}
