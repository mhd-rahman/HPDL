//! Native-HPDL ↔ RON canonical-PIR equivalence (ADR 0010).
//!
//! The determinism invariant requires that equivalent semantics authored in
//! native `.hpdl` or in RON produce **byte-identical** canonical PIR and the same
//! content hash. These tests compile a comprehensive definition — exercising every
//! block the native grammar v1.0 supports — through both frontends and assert the
//! payload bytes and hashes match exactly.

use hpdl_compiler::{CompileOptions, Compiler};

/// The same planet, written in native HPDL and in RON. Exercises physical,
/// traits, statuses, actions (+capabilities), migrations, and the full orbital +
/// environment surface. Modules are omitted here (they require a lockfile to
/// resolve and do not affect PIR bytes — see `packages.rs`
/// `package_lock_does_not_change_pir_bytes`); module parity is covered by the
/// model-equality test in `hpdl-syntax/tests/cross_frontend_tests.rs`.
const HPDL: &str = r#"
language hpdl "1.0";
planet terra {
    id: "terra",
    physical: {
        mass_kg: 5972000000000000000000000,
        radius_m: 6371000,
    },
    traits: { tags: ["terrestrial"] },
    statuses: [
        StatusDeclaration(name: "Vitality", scale_min: 0, scale_max: 100),
    ],
    actions: [
        ActionDeclaration(
            name: "Terraform",
            reads: ["state.atmosphere.co2"],
            writes: ["state.atmosphere.co2"],
            capabilities: [ Capability("state.atmosphere.write") ],
        ),
    ],
    migrations: [
        MigrationDeclaration(name: "add-ocean", from_version: 1, to_version: 2),
    ],
    identity: Planet,
    parent: ParentRef(target: BodyId("sol"), frame: EclipticJ2000),
    orbit: KeplerianElements(
        semi_major_axis_m: 149598000000,
        eccentricity_micro: 16700,
        inclination_mdeg: 0,
        lon_ascending_node_mdeg: 174000,
        arg_periapsis_mdeg: 288000,
        mean_anomaly_epoch_mdeg: 0,
        frame: EclipticJ2000,
    ),
    rotation: RotationModel(
        sidereal_period_s: 86164,
        axial_tilt_mdeg: 23440,
        frame: EclipticJ2000,
    ),
    environment: EnvironmentDefinition(
        biosphere: Present(Microbial),
        atmosphere: AtmosphereProfile(
            surface_pressure_pa: 101325,
            composition: Composition(
                components: [ (GasId("n2"), 780000), (GasId("o2"), 220000) ],
                tolerance_ppm: 0,
            ),
        ),
        thermal: ThermalProfile(mean_mk: 288000, min_mk: 184000, max_mk: 331000),
        hydrosphere: Hydrosphere(surface_coverage_ppm: 708000),
        cryosphere: Cryosphere(surface_coverage_ppm: 100000),
    ),
}
"#;

const RON: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("terra"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),
            radius_m: Some(6371000),
            surface_gravity_mms2: None,
            orbital_period_s: None,
            rotation_period_s: None,
        ),
        traits: TraitSet(["terrestrial"]),
        modules: [],
        statuses: [
            StatusDeclaration(name: "Vitality", scale_min: 0, scale_max: 100),
        ],
        actions: [
            ActionDeclaration(
                name: "Terraform",
                reads: ["state.atmosphere.co2"],
                writes: ["state.atmosphere.co2"],
                capabilities: [ Capability("state.atmosphere.write") ],
            ),
        ],
        migrations: [
            MigrationDeclaration(name: "add-ocean", from_version: 1, to_version: 2),
        ],
        identity: Some(Planet),
        parent: Some(ParentRef(target: BodyId("sol"), frame: EclipticJ2000)),
        orbit: Some(KeplerianElements(
            semi_major_axis_m: 149598000000,
            eccentricity_micro: 16700,
            inclination_mdeg: 0,
            lon_ascending_node_mdeg: 174000,
            arg_periapsis_mdeg: 288000,
            mean_anomaly_epoch_mdeg: 0,
            frame: EclipticJ2000,
        )),
        rotation: Some(RotationModel(
            sidereal_period_s: 86164,
            axial_tilt_mdeg: 23440,
            frame: EclipticJ2000,
        )),
        environment: Some(EnvironmentDefinition(
            biosphere: Present(Microbial),
            atmosphere: Some(AtmosphereProfile(
                surface_pressure_pa: 101325,
                composition: Composition(
                    components: { GasId("n2"): 780000, GasId("o2"): 220000 },
                    tolerance_ppm: 0,
                ),
            )),
            thermal: Some(ThermalProfile(mean_mk: 288000, min_mk: 184000, max_mk: 331000)),
            hydrosphere: Some(Hydrosphere(surface_coverage_ppm: 708000, dominant_liquid: None)),
            cryosphere: Some(Cryosphere(surface_coverage_ppm: 100000)),
            assets: {},
        )),
    )
)
"#;

#[test]
fn hpdl_and_ron_produce_identical_pir_bytes() {
    let compiler = Compiler::new(CompileOptions::default());

    let from_hpdl = compiler
        .compile_hpdl_str("terra", HPDL)
        .expect("hpdl compile");
    let from_ron = compiler.compile_ron_str("terra", RON).expect("ron compile");

    assert_eq!(
        from_hpdl.envelope().payload,
        from_ron.envelope().payload,
        "canonical PIR payload bytes must be identical across native HPDL and RON"
    );
    assert_eq!(
        from_hpdl.envelope().content_hash,
        from_ron.envelope().content_hash,
        "content hashes must be identical across native HPDL and RON"
    );
    // Sanity: the hash is well-formed and self-consistent.
    assert!(from_hpdl.envelope().verify_hash());
    assert_eq!(from_hpdl.content_hash_hex().len(), 64);
}

#[test]
fn minimal_hpdl_and_ron_match() {
    let compiler = Compiler::new(CompileOptions::default());

    const MIN_HPDL: &str = r#"
        language hpdl "1.0";
        planet m {
            id: "m",
            physical: { mass_kg: 1000000000000000000000000, radius_m: 1000000 },
        }
    "#;
    const MIN_RON: &str = r#"
        PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("m"),
                properties: PhysicalProperties(
                    mass_kg: Some(1000000000000000000000000),
                    radius_m: Some(1000000),
                    surface_gravity_mms2: None,
                    orbital_period_s: None,
                    rotation_period_s: None,
                ),
                traits: TraitSet([]),
                modules: [], statuses: [], actions: [], migrations: [],
            )
        )
    "#;

    let from_hpdl = compiler.compile_hpdl_str("m", MIN_HPDL).expect("hpdl");
    let from_ron = compiler.compile_ron_str("m", MIN_RON).expect("ron");
    assert_eq!(from_hpdl.envelope().payload, from_ron.envelope().payload);
    assert_eq!(
        from_hpdl.envelope().content_hash,
        from_ron.envelope().content_hash
    );
}
