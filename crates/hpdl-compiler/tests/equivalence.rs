//! Cross-frontend equivalence: the shared `compile_validated` tail must yield
//! byte-identical PIR to the RON entry path for the same semantics.

use hpdl_compiler::{CompileOptions, Compiler};
use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};
use hpdl_sema::{Validator, ValidatorConfig};

const RON: &str = r#"
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

/// Build the semantically identical definition directly in Rust.
fn direct_definition() -> PlanetDefinition {
    let props = PhysicalProperties {
        mass_kg: Some(5_972_000_000_000_000_000_000_000),
        radius_m: Some(6_371_000),
        surface_gravity_mms2: Some(9_810),
        orbital_period_s: None,
        rotation_period_s: None,
    };
    let body = CelestialBody::new(
        BodyId::new("earth").expect("body id"),
        props,
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .expect("body");
    PlanetDefinition::new("1.0", body).expect("definition")
}

#[test]
fn validated_and_ron_paths_match() {
    let compiler = Compiler::new(CompileOptions::default());

    // RON entry path.
    let from_ron = compiler.compile_ron_str("earth", RON).expect("ron compile");

    // Direct validated path.
    let validator = Validator::new(ValidatorConfig::default());
    let validated = validator.validate(direct_definition()).expect("validation");
    let from_validated = compiler
        .compile_validated("earth", validated)
        .expect("validated compile");

    assert_eq!(
        from_ron.envelope().payload,
        from_validated.envelope().payload,
        "payload bytes must match across entry paths"
    );
    assert_eq!(
        from_ron.envelope().content_hash,
        from_validated.envelope().content_hash,
        "content hashes must match across entry paths"
    );
}
