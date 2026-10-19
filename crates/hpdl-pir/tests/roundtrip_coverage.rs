//! Full-surface encode/decode round-trips.
//!
//! The minimal golden-vector tests only populate a bare `CelestialBody`, so the
//! wincode `SchemaWrite`/`SchemaRead` code paths for the optional orbital,
//! environment, status, action, migration and module types are never exercised
//! on the encode OR decode side. These tests round-trip **maximally-populated**
//! definitions through `encode_planet_definition` → serialize → `decode_planet_
//! definition`, asserting the decoded value equals the original. That drives
//! every reachable serialize/deserialize branch for the full model surface.
//!
//! Builders and values mirror `hpdl-model/tests/roundtrip.rs` so these stay
//! consistent with the model's own round-trip coverage.

#![allow(clippy::unwrap_used)]

use hpdl_pir::{decode_planet_definition, encode_planet_definition, PirVersion};

/// A body with every orbital declaration field populated.
fn orbital_definition() -> hpdl_model::PlanetDefinition {
    use hpdl_model::{
        BodyId, CelestialBody, CelestialIdentity, KeplerianElements, ParentRef, PhysicalProperties,
        PlanetDefinition, ReferenceFrame, RotationModel, TraitSet,
    };

    let body = CelestialBody::new(
        BodyId::new("mars").unwrap(),
        PhysicalProperties {
            mass_kg: Some(641_710_000_000_000_000_000_000),
            radius_m: Some(3_389_500),
            surface_gravity_mms2: Some(3_721),
            orbital_period_s: Some(59_355_072),
            rotation_period_s: Some(88_642),
        },
        TraitSet::new(["rocky", "thin-atmosphere"]).unwrap(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .with_identity(CelestialIdentity::Planet)
    .with_parent(ParentRef::new(
        BodyId::new("sol").unwrap(),
        ReferenceFrame::EclipticJ2000,
    ))
    .with_orbit(
        KeplerianElements::new(
            227_939_000_000,
            93_400,
            1_850,
            49_558,
            286_502,
            19_412,
            ReferenceFrame::EclipticJ2000,
        )
        .unwrap(),
    )
    .with_rotation(RotationModel::new(88_642, 25_190, ReferenceFrame::EclipticJ2000).unwrap());

    PlanetDefinition::new("1.0", body).unwrap()
}

/// A body with a fully-declared environment (atmosphere, thermal, hydro/cryo,
/// biosphere, asset).
fn environment_definition() -> hpdl_model::PlanetDefinition {
    use hpdl_model::{
        AssetKind, AssetRef, AtmosphereProfile, Biosphere, BodyId, CelestialBody, ComplexityTier,
        Composition, Cryosphere, Digest, EnvironmentDefinition, GasId, Hydrosphere,
        PhysicalProperties, PlanetDefinition, ThermalProfile, TraitSet,
    };

    let composition = Composition::new(
        [
            (GasId::new("n2").unwrap(), 780_800),
            (GasId::new("o2").unwrap(), 209_500),
            (GasId::new("ar").unwrap(), 9_700),
        ],
        0,
    )
    .unwrap();

    let env = EnvironmentDefinition::new(Biosphere::Present(ComplexityTier::Complex))
        .with_atmosphere(AtmosphereProfile::new(101_325, composition, None).unwrap())
        .with_thermal(ThermalProfile::new(288_000, 184_000, 331_000).unwrap())
        .with_hydrosphere(Hydrosphere::new(708_000, Some(GasId::new("h2o").unwrap())).unwrap())
        .with_cryosphere(Cryosphere::new(100_000).unwrap())
        .with_asset(
            AssetRef::new(
                "surface-albedo",
                AssetKind::Texture,
                Digest::new("a".repeat(64)).unwrap(),
            )
            .unwrap(),
        );

    let body = CelestialBody::new(
        BodyId::new("terra-nova").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .with_environment(env);

    PlanetDefinition::new("1.0", body).unwrap()
}

/// A body carrying module refs, statuses, actions and migrations — the `Vec`
/// collection fields of `CelestialBody` that a minimal definition leaves empty.
fn declarations_definition() -> hpdl_model::PlanetDefinition {
    use hpdl_model::{
        ActionDeclaration, BodyId, Capability, CelestialBody, MigrationDeclaration, ModuleRef,
        PhysicalProperties, PlanetDefinition, StatusDeclaration, TraitSet,
    };

    let modules = vec![
        ModuleRef::new("hpdl.atmosphere", "1.2.0").unwrap(),
        ModuleRef::new("hpdl.vitality.eco", "0.3.1").unwrap(),
    ];

    let statuses = vec![
        StatusDeclaration::new("Habitability", 0, 100).unwrap(),
        StatusDeclaration::new("Pollution", 0, 1000).unwrap(),
    ];

    // Populate a capability too, so the action's `capabilities` BTreeSet is
    // non-empty on the wire (exercises that field's serialize/deserialize).
    let actions = vec![ActionDeclaration::new(
        "TerraformStep",
        ["state.atmosphere.co2"],
        ["state.thermal.mean"],
    )
    .unwrap()
    .with_capabilities([Capability::new("state.thermal.write").unwrap()])];

    let migrations = vec![MigrationDeclaration::new("AddOcean", 1, 2).unwrap()];

    let body = CelestialBody::new(
        BodyId::new("kepler-186f").unwrap(),
        PhysicalProperties::default(),
        TraitSet::new(["geology.rocky"]).unwrap(),
        modules,
        statuses,
        actions,
        migrations,
    )
    .unwrap();

    PlanetDefinition::new("1.0", body).unwrap()
}

/// Encode → serialize envelope → decode and assert the decoded definition is
/// byte-for-byte equal to the original.
fn assert_roundtrips(def: &hpdl_model::PlanetDefinition) {
    let envelope = encode_planet_definition(def).unwrap();
    assert_eq!(envelope.version, PirVersion::CURRENT);
    assert!(envelope.verify_hash());

    let bytes = wincode::serialize(&envelope).unwrap();
    let decoded = decode_planet_definition(&bytes).unwrap();

    assert_eq!(&decoded, def, "decoded definition must equal the original");
}

#[test]
fn orbital_body_roundtrips() {
    assert_roundtrips(&orbital_definition());
}

#[test]
fn environment_body_roundtrips() {
    assert_roundtrips(&environment_definition());
}

#[test]
fn declarations_body_roundtrips() {
    assert_roundtrips(&declarations_definition());
}

/// Two distinct fully-populated definitions must hash differently — a sanity
/// check that the extra populated fields actually reach the canonical bytes.
#[test]
fn distinct_full_definitions_differ() {
    let a = encode_planet_definition(&orbital_definition()).unwrap();
    let b = encode_planet_definition(&environment_definition()).unwrap();
    assert_ne!(a.content_hash, b.content_hash);
}
