//! Parse a RON planet definition from an in-memory string.
//!
//! Run with: `cargo run -p hpdl-ron --example basic_usage`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hpdl_ron::RonLoader;

const SOURCE: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("kepler-186f"),
        properties: PhysicalProperties(
            mass_kg: Some(6500000000000000000000000),
            radius_m: Some(7100000),
            surface_gravity_mms2: None,
            orbital_period_s: None,
            rotation_period_s: None,
        ),
        traits: TraitSet(["terrestrial", "habitable-zone"]),
        modules: [],
        statuses: [],
        actions: [],
        migrations: [],
    )
)
"#;

fn main() {
    let loader = RonLoader::new();
    match loader.load_string("kepler-186f.ron", SOURCE.to_string()) {
        Ok(parsed) => {
            let def = parsed.definition();
            println!("Parsed planet: {}", def.body().id());
            println!("Language version: {}", def.language_version());
            println!("Traits: {:?}", def.body().traits());
        }
        Err(e) => eprintln!("Parse failed: {e}"),
    }
}
