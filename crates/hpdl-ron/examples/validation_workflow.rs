//! End-to-end workflow: parse RON, then validate through the shared `hpdl-sema`
//! pipeline.
//!
//! Run with: `cargo run -p hpdl-ron --example validation_workflow`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hpdl_ron::RonLoader;

const SOURCE: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("terra-nova"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),
            radius_m: Some(6371000),
            surface_gravity_mms2: Some(9810),
            orbital_period_s: Some(31557600),
            rotation_period_s: Some(86164),
        ),
        traits: TraitSet(["terrestrial"]),
        modules: [],
        statuses: [
            StatusDeclaration(name: "AtmosphericStability", scale_min: 0, scale_max: 100),
        ],
        actions: [],
        migrations: [],
    )
)
"#;

fn main() {
    let loader = RonLoader::new();
    let parsed = match loader.load_string("terra-nova.ron", SOURCE.to_string()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse failed: {e}");
            return;
        }
    };

    println!("Parsed {}. Validating...", parsed.definition().body().id());

    match parsed.validate() {
        Ok(validated) => {
            println!(
                "Validation succeeded (validator {}).",
                validated.metadata().validator_version
            );
        }
        Err(e) => eprintln!("Validation failed:\n{e}"),
    }
}
