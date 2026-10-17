//! Emit canonical PIR content digests for a fixed set of definitions.
//!
//! Used by the CI `digest-parity` job: each OS runs this example and the
//! resulting `name=<hash>` lines are diffed across platforms to prove that
//! equivalent semantics yield byte-identical canonical PIR everywhere
//! (determinism steering: "compare PIR and replay hashes across OS jobs").
//!
//! Output is deterministic: fixtures are compiled in a fixed order and each line
//! is `<name>=<64-hex content hash>`. No wall-clock, locale, or hashmap iteration
//! reaches the output.

use hpdl_compiler::{CompileOptions, Compiler};

/// A minimal planet with no module references.
const MINIMAL: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("earth"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),
            radius_m: Some(6371000),
            surface_gravity_mms2: Some(9810),
            orbital_period_s: None,
            rotation_period_s: None,
        ),
        traits: TraitSet([]),
        modules: [],
        statuses: [],
        actions: [],
        migrations: [],
    )
)
"#;

/// A planet carrying a couple of traits and physical fields.
const TRAITS: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("mars"),
        properties: PhysicalProperties(
            mass_kg: Some(641710000000000000000000),
            radius_m: Some(3389500),
            surface_gravity_mms2: Some(3720),
            orbital_period_s: Some(59354294),
            rotation_period_s: Some(88642),
        ),
        traits: TraitSet(["rocky", "thin-atmosphere"]),
        modules: [],
        statuses: [],
        actions: [],
        migrations: [],
    )
)
"#;

fn main() {
    // (name, source) in a fixed order — never a HashMap.
    let fixtures: &[(&str, &str)] = &[("minimal", MINIMAL), ("traits", TRAITS)];

    let compiler = Compiler::new(CompileOptions::default());
    for (name, source) in fixtures {
        match compiler.compile_ron_str(name, source) {
            Ok(result) => println!("{name}={}", result.content_hash_hex()),
            Err(e) => {
                eprintln!("error compiling `{name}`: {e}");
                std::process::exit(1);
            }
        }
    }
}
