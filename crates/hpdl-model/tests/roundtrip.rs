//! Golden roundtrip tests for `hpdl-model`.
//!
//! Each test loads a fixture from `tests/golden/`, deserializes it into a
//! `PlanetDefinition`, re-serializes to JSON, and asserts the output matches
//! the fixture byte-for-byte (modulo trailing whitespace).
//!
//! These tests are intentionally data-driven so new fixtures can be added
//! without writing new test functions.
//!
//! Golden fixtures are deliberately frozen: updating `minimal_planet.json` or
//! `orbital_body.json` is a reviewed change. If a change alters canonical PIR
//! bytes (feature 07 fields are not yet lowered into PIR — see OD-07.1), it also
//! requires a PIR format-version decision, which is out of scope for this crate.

// Test code legitimately uses unwrap/expect for setup assertions.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

/// A serde-compatible mirror of the model types, used only for golden tests.
/// Real serde derives on model types are deferred until the RON frontend (OD-07).
mod mirror {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct PhysicalProperties {
        pub mass_kg: Option<u128>,
        pub radius_m: Option<u128>,
        pub surface_gravity_mms2: Option<u64>,
        pub orbital_period_s: Option<u64>,
        pub rotation_period_s: Option<i64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ModuleRef {
        pub name: String,
        pub version: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct StatusDeclaration {
        pub name: String,
        pub scale_min: i64,
        pub scale_max: i64,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ActionDeclaration {
        pub name: String,
        pub reads: Vec<String>,
        pub writes: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct MigrationDeclaration {
        pub name: String,
        pub from_version: u32,
        pub to_version: u32,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CelestialBody {
        pub id: String,
        pub properties: PhysicalProperties,
        pub traits: Vec<String>,
        pub modules: Vec<ModuleRef>,
        pub statuses: Vec<StatusDeclaration>,
        pub actions: Vec<ActionDeclaration>,
        pub migrations: Vec<MigrationDeclaration>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct PlanetDefinition {
        pub language_version: String,
        pub body: CelestialBody,
    }
}

/// Convert a mirror `PlanetDefinition` to the real model types and verify it
/// round-trips through the constructors without error.
fn assert_mirror_valid(mirror: &mirror::PlanetDefinition) {
    use hpdl_model::{
        ActionDeclaration, BodyId, CelestialBody, MigrationDeclaration, ModuleRef,
        PhysicalProperties, PlanetDefinition, StatusDeclaration, TraitSet,
    };

    let id = BodyId::new(&mirror.body.id).expect("invalid BodyId in fixture");

    let props = PhysicalProperties {
        mass_kg: mirror.body.properties.mass_kg,
        radius_m: mirror.body.properties.radius_m,
        surface_gravity_mms2: mirror.body.properties.surface_gravity_mms2,
        orbital_period_s: mirror.body.properties.orbital_period_s,
        rotation_period_s: mirror.body.properties.rotation_period_s,
    };

    let traits =
        TraitSet::new(mirror.body.traits.iter().cloned()).expect("invalid trait in fixture");

    let modules: Vec<_> = mirror
        .body
        .modules
        .iter()
        .map(|m| ModuleRef::new(&m.name, &m.version).expect("invalid ModuleRef in fixture"))
        .collect();

    let statuses: Vec<_> = mirror
        .body
        .statuses
        .iter()
        .map(|s| {
            StatusDeclaration::new(&s.name, s.scale_min, s.scale_max)
                .expect("invalid StatusDeclaration in fixture")
        })
        .collect();

    let actions: Vec<_> = mirror
        .body
        .actions
        .iter()
        .map(|a| {
            ActionDeclaration::new(&a.name, a.reads.iter().cloned(), a.writes.iter().cloned())
                .expect("invalid ActionDeclaration in fixture")
        })
        .collect();

    let migrations: Vec<_> = mirror
        .body
        .migrations
        .iter()
        .map(|m| {
            MigrationDeclaration::new(&m.name, m.from_version, m.to_version)
                .expect("invalid MigrationDeclaration in fixture")
        })
        .collect();

    let body = CelestialBody::new(id, props, traits, modules, statuses, actions, migrations)
        .expect("invalid CelestialBody in fixture");

    PlanetDefinition::new(&mirror.language_version, body)
        .expect("invalid PlanetDefinition in fixture");
}

#[test]
fn golden_minimal_planet_roundtrips() {
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/minimal_planet.json");

    let fixture_text = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", fixture_path.display()));

    let parsed: mirror::PlanetDefinition =
        serde_json::from_str(&fixture_text).expect("fixture is not valid JSON");

    // Verify the model constructors accept the fixture data
    assert_mirror_valid(&parsed);

    // Verify re-serialization matches the fixture (canonical JSON)
    let reserialized = serde_json::to_string_pretty(&parsed).expect("re-serialization failed");

    // Normalize line endings for cross-platform comparison
    let fixture_normalized = fixture_text.trim_end().replace("\r\n", "\n");
    let reserialized_normalized = reserialized.trim_end().replace("\r\n", "\n");

    assert_eq!(
        fixture_normalized, reserialized_normalized,
        "golden fixture did not roundtrip cleanly"
    );
}

/// Build a fully-declared orbital body using the real model types.
///
/// This exercises the actual serde derives on the orbital types (feature 07),
/// not the mirror structs.
fn orbital_body_definition() -> hpdl_model::PlanetDefinition {
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
        TraitSet::empty(),
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

/// Build a fully-declared environment body using the real model types.
fn environment_body_definition() -> hpdl_model::PlanetDefinition {
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

/// Build a body with a fully-declared multidimensional status using real types.
fn status_vitality_definition() -> hpdl_model::PlanetDefinition {
    use hpdl_model::{
        BaselineRef, BodyId, CelestialBody, CutPoints, Digest, EvaluationWindow, EvaluatorRef,
        ModuleRef, PhysicalProperties, PlanetDefinition, StatusDeclaration, StatusDimension,
        TraitSet,
    };

    let cut_points = CutPoints::new(
        0,
        100,
        [
            (0, "critical".to_string()),
            (25, "degraded".to_string()),
            (50, "stressed".to_string()),
            (75, "stable".to_string()),
        ],
    )
    .unwrap();

    let status = StatusDeclaration::new("EcosystemVitality", 0, 100)
        .unwrap()
        .with_dimension(StatusDimension::NativeEcosystem)
        .with_baseline(
            BaselineRef::new("preindustrial", Digest::new("b".repeat(64)).unwrap()).unwrap(),
        )
        .with_evaluator(EvaluatorRef::new(
            ModuleRef::new("hpdl.vitality.eco", "1.0").unwrap(),
        ))
        .with_cut_points(cut_points)
        .with_window(EvaluationWindow::new(1000, Some("surface".to_string())).unwrap());

    let body = CelestialBody::new(
        BodyId::new("terra-nova").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![status],
        vec![],
        vec![],
    )
    .unwrap();

    PlanetDefinition::new("1.0", body).unwrap()
}

#[test]
fn golden_status_vitality_body_roundtrips() {
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/status_vitality_body.json");

    let def = status_vitality_definition();
    let serialized = serde_json::to_string_pretty(&def).expect("serialization failed");

    let fixture_text = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", fixture_path.display()));

    let fixture_normalized = fixture_text.trim_end().replace("\r\n", "\n");
    let serialized_normalized = serialized.trim_end().replace("\r\n", "\n");

    assert_eq!(
        fixture_normalized, serialized_normalized,
        "status-vitality golden fixture did not match the serialized model"
    );

    let parsed: hpdl_model::PlanetDefinition =
        serde_json::from_str(&fixture_text).expect("fixture is not valid for the model");
    assert_eq!(parsed, def, "fixture did not deserialize to the same model");
}

#[test]
fn golden_environment_body_roundtrips() {
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/environment_body.json");

    let def = environment_body_definition();
    let serialized = serde_json::to_string_pretty(&def).expect("serialization failed");

    let fixture_text = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", fixture_path.display()));

    let fixture_normalized = fixture_text.trim_end().replace("\r\n", "\n");
    let serialized_normalized = serialized.trim_end().replace("\r\n", "\n");

    assert_eq!(
        fixture_normalized, serialized_normalized,
        "environment golden fixture did not match the serialized model"
    );

    let parsed: hpdl_model::PlanetDefinition =
        serde_json::from_str(&fixture_text).expect("fixture is not valid for the model");
    assert_eq!(parsed, def, "fixture did not deserialize to the same model");
}

#[test]
fn golden_orbital_body_roundtrips() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/orbital_body.json");

    let def = orbital_body_definition();

    // Serialize the real model type.
    let serialized = serde_json::to_string_pretty(&def).expect("serialization failed");

    let fixture_text = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", fixture_path.display()));

    let fixture_normalized = fixture_text.trim_end().replace("\r\n", "\n");
    let serialized_normalized = serialized.trim_end().replace("\r\n", "\n");

    assert_eq!(
        fixture_normalized, serialized_normalized,
        "orbital golden fixture did not match the serialized model"
    );

    // Deserialize the fixture back into the real model and verify equality.
    let parsed: hpdl_model::PlanetDefinition =
        serde_json::from_str(&fixture_text).expect("fixture is not valid for the model");
    assert_eq!(parsed, def, "fixture did not deserialize to the same model");
}

/// Build a body declaring world laws (feature 11): an additive physical law, a
/// replacement law over a disjoint domain, and a magic law with a declared
/// bridge. Exercises the real serde derives on the world-law types.
fn world_law_body_definition() -> hpdl_model::PlanetDefinition {
    use hpdl_model::{
        BodyId, CelestialBody, LawDomain, LawMode, MagicBridge, ModuleRef, PhysicalProperties,
        PlanetDefinition, TraitSet, WorldLaw,
    };

    let additive = WorldLaw::new(
        ModuleRef::new("hpdl.law.extra-gravity", "1.0").unwrap(),
        LawMode::Additive,
        [LawDomain::new("physics.gravity").unwrap()],
        None,
    )
    .unwrap();

    let replacement = WorldLaw::new(
        ModuleRef::new("hpdl.law.custom-thermo", "1.0").unwrap(),
        LawMode::Replaces([LawDomain::new("physics.thermodynamics").unwrap()].into()),
        [LawDomain::new("physics.thermodynamics").unwrap()],
        None,
    )
    .unwrap();

    let magic = WorldLaw::new(
        ModuleRef::new("hpdl.law.aethermancy", "1.0").unwrap(),
        LawMode::Additive,
        [
            LawDomain::new("arcane.aether").unwrap(),
            LawDomain::new("physics.energy").unwrap(),
        ],
        Some(
            MagicBridge::new(
                [LawDomain::new("arcane.aether").unwrap()],
                LawDomain::new("physics.energy").unwrap(),
                ModuleRef::new("hpdl.bridge.aether-energy", "1.0").unwrap(),
            )
            .unwrap(),
        ),
    )
    .unwrap();

    let body = CelestialBody::new(
        BodyId::new("arcanis").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
    .with_world_laws([additive, replacement, magic]);

    PlanetDefinition::new("1.0", body).unwrap()
}

#[test]
fn world_law_body_carries_laws_in_memory() {
    use hpdl_model::{LawMode, PHYSICAL_NAMESPACE};

    let def = world_law_body_definition();
    let laws = def.body().world_laws();

    // All three declared laws are carried on the live model for the validator.
    assert_eq!(laws.len(), 3);

    // Additive physical law.
    assert!(matches!(laws[0].mode(), LawMode::Additive));
    assert!(laws[0].domains().all(|d| d.is_physical()));
    assert!(laws[0].authoritative_domains().is_empty());

    // Replacement law is authoritative over its disjoint domain.
    assert!(matches!(laws[1].mode(), LawMode::Replaces(_)));
    assert_eq!(laws[1].authoritative_domains().len(), 1);

    // Magic law with a declared bridge into the physical namespace.
    assert!(laws[2].is_magic());
    assert!(laws[2].touches_physical());
    let bridge = laws[2].bridge().expect("magic law declares a bridge");
    assert!(bridge.target().as_str().starts_with(PHYSICAL_NAMESPACE));
}

/// World laws are deliberately excluded from serialization this feature
/// (spec 11, OD-11.2): canonical PIR bytes and existing golden vectors must not
/// change. This test freezes that decision — if a future change starts
/// serializing world laws (e.g. a PIR revision), this test must be updated
/// deliberately alongside a format-version decision.
#[test]
fn world_laws_are_not_serialized_this_feature() {
    let def = world_law_body_definition();
    assert_eq!(def.body().world_laws().len(), 3, "laws present in memory");

    let json = serde_json::to_string(&def).expect("serialization failed");
    assert!(
        !json.contains("world_laws"),
        "world_laws must be omitted from serialization (OD-11.2): PIR bytes unchanged"
    );
    assert!(
        !json.contains("aethermancy"),
        "no world-law content should leak into serialized output"
    );

    // Deserializing output with no world_laws field yields an empty set, and the
    // rest of the body round-trips unchanged.
    let parsed: hpdl_model::PlanetDefinition =
        serde_json::from_str(&json).expect("round-trip parse failed");
    assert!(parsed.body().world_laws().is_empty());
    assert_eq!(parsed.body().id().as_str(), "arcanis");
}

#[test]
fn golden_minimal_planet_model_invariants() {
    use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};

    // Construct the same definition programmatically and verify invariants
    let body = CelestialBody::new(
        BodyId::new("proxima-b").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();

    let def = PlanetDefinition::new("1.0", body).unwrap();

    assert_eq!(def.language_version(), "1.0");
    assert_eq!(def.body().id().as_str(), "proxima-b");
    assert!(
        def.body().properties().mass_kg.is_none(),
        "no Earth defaults"
    );
    assert!(def.body().traits().is_empty());
    assert!(def.body().actions().is_empty());
    assert!(def.body().statuses().is_empty());
    assert!(def.body().migrations().is_empty());
}
