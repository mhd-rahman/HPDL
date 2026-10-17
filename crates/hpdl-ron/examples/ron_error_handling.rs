//! Demonstrates the distinct failure modes of the RON frontend: a syntax error
//! (parse failure) versus a semantically-invalid-but-well-formed definition.
//!
//! Run with: `cargo run -p hpdl-ron --example ron_error_handling`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hpdl_ron::{Error, RonLoader};

const MALFORMED: &str = "PlanetDefinition(( not valid RON";

const WELL_FORMED_BUT_INVALID: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("void"),
        properties: PhysicalProperties(
            mass_kg: Some(0),
            radius_m: None, surface_gravity_mms2: None,
            orbital_period_s: None, rotation_period_s: None,
        ),
        traits: TraitSet([]),
        modules: [], statuses: [], actions: [], migrations: [],
    )
)
"#;

fn main() {
    let loader = RonLoader::new();

    // 1. Syntax error -> parse failure.
    match loader.load_string("bad.ron", MALFORMED.to_string()) {
        Ok(_) => println!("unexpected: malformed parsed"),
        Err(Error::ParseWithSpan { position, error }) => {
            println!("Parse error at {position}: {error}");
        }
        Err(e) => println!("Parse error: {e}"),
    }

    // 2. Well-formed RON, but zero mass -> semantic validation failure.
    match loader.load_string("void.ron", WELL_FORMED_BUT_INVALID.to_string()) {
        Ok(parsed) => match parsed.validate() {
            Ok(_) => println!("unexpected: zero-mass validated"),
            Err(e) => println!("Validation error:\n{e}"),
        },
        Err(e) => println!("Unexpected parse error: {e}"),
    }
}
