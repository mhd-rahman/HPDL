//! End-to-end integration tests over the public `Validator` API.
//!
//! These complement the per-module unit tests by exercising the whole
//! validation pipeline on complete `PlanetDefinition`s, covering positive,
//! negative, and boundary cases across multiple validators at once.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use hpdl_model::{
    ActionDeclaration, BodyId, Capability, CelestialBody, CelestialIdentity, KeplerianElements,
    LawDomain, LawMode, MagicBridge, ModuleRef, ParentRef, PhysicalProperties, PlanetDefinition,
    ReferenceFrame, RotationModel, StatusDeclaration, TraitSet, WorldLaw,
};
use hpdl_sema::{DiagnosticCode, Severity, Validator, ValidatorConfig};

fn body(id: &str) -> CelestialBody {
    CelestialBody::new(
        BodyId::new(id).unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap()
}

fn def(body: CelestialBody) -> PlanetDefinition {
    PlanetDefinition::new("1.0", body).unwrap()
}

#[test]
fn minimal_definition_validates() {
    let v = Validator::default_config();
    assert!(v.validate(def(body("earth"))).is_ok());
}

#[test]
fn comprehensive_valid_definition_validates() {
    // A fully-populated, internally-consistent definition should pass clean.
    let b = CelestialBody::new(
        BodyId::new("mars").unwrap(),
        PhysicalProperties {
            mass_kg: Some(641_710_000_000_000_000_000_000),
            radius_m: Some(3_389_500),
            surface_gravity_mms2: Some(3_721),
            orbital_period_s: Some(59_355_072),
            rotation_period_s: Some(88_642),
        },
        TraitSet::new(["terrestrial".to_string(), "cold".to_string()]).unwrap(),
        vec![ModuleRef::new("hpdl/stdlib/atmospheric", "1.0").unwrap()],
        vec![StatusDeclaration::new("AtmosphericStability", 0, 100).unwrap()],
        vec![ActionDeclaration::new(
            "TerraformStep",
            ["state.atmosphere.co2"],
            ["state.atmosphere.co2"],
        )
        .unwrap()
        .with_capabilities([Capability::new("state.atmosphere.write").unwrap()])],
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

    let result = Validator::default_config().validate(def(b));
    assert!(result.is_ok(), "expected clean validation, got {result:?}");
}

#[test]
fn zero_mass_is_rejected() {
    let b = CelestialBody::new(
        BodyId::new("void").unwrap(),
        PhysicalProperties {
            mass_kg: Some(0),
            ..PhysicalProperties::default()
        },
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();

    let result = Validator::default_config().validate(def(b));
    let diags = result.expect_err("zero mass must fail");
    assert!(diags.iter().any(|d| d.code == DiagnosticCode::NegativeMass));
}

#[test]
fn parent_cycle_is_rejected_end_to_end() {
    let b = body("sol").with_parent(ParentRef::new(
        BodyId::new("sol").unwrap(),
        ReferenceFrame::EclipticJ2000,
    ));
    let diags = Validator::default_config()
        .validate(def(b))
        .expect_err("self-parent must fail");
    assert!(diags.iter().any(|d| d.code == DiagnosticCode::ParentCycle));
}

#[test]
fn custom_frame_is_unsupported_end_to_end() {
    let b = body("exotic").with_orbit(
        KeplerianElements::new(
            1_000_000,
            0,
            0,
            0,
            0,
            0,
            ReferenceFrame::custom("arcane.ley").unwrap(),
        )
        .unwrap(),
    );
    let diags = Validator::default_config()
        .validate(def(b))
        .expect_err("custom frame must fail");
    assert!(diags
        .iter()
        .any(|d| d.code == DiagnosticCode::UnsupportedOrbitalModel));
}

#[test]
fn multiple_errors_are_all_reported_and_sorted() {
    // A definition with several independent problems should surface all of them,
    // deterministically ordered. The collection limit of 1 also trips a limit error.
    let v = Validator::new(ValidatorConfig::default().with_max_collection_size(1));

    let statuses = vec![
        StatusDeclaration::new("A", 0, 100).unwrap(),
        StatusDeclaration::new("B", 0, 100).unwrap(),
    ];
    let b = CelestialBody::new(
        BodyId::new("messy").unwrap(),
        PhysicalProperties {
            mass_kg: Some(0),
            ..PhysicalProperties::default()
        },
        TraitSet::empty(),
        vec![],
        statuses,
        vec![],
        vec![],
    )
    .unwrap()
    .with_parent(ParentRef::new(
        BodyId::new("messy").unwrap(),
        ReferenceFrame::EclipticJ2000,
    ));

    let diags = v.validate(def(b)).expect_err("should fail");

    // Zero mass + parent cycle + collection-limit are all present.
    let codes: Vec<DiagnosticCode> = diags.iter().map(|d| d.code).collect();
    assert!(codes.contains(&DiagnosticCode::NegativeMass));
    assert!(codes.contains(&DiagnosticCode::ParentCycle));
    assert!(codes.contains(&DiagnosticCode::MaxCollectionSizeExceeded));

    // Deterministic order: diagnostics must be sorted.
    for w in diags.windows(2) {
        assert!(w[0] <= w[1], "diagnostics must be sorted");
    }
}

#[test]
fn strict_mode_promotes_warnings() {
    // An impossibly-light planet triggers a density warning; strict mode turns
    // that (or any warning) into a failure.
    let props = PhysicalProperties {
        mass_kg: Some(1),
        radius_m: Some(1_000_000),
        ..PhysicalProperties::default()
    };
    let b = CelestialBody::new(
        BodyId::new("feather").unwrap(),
        props,
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();

    let lenient = Validator::new(ValidatorConfig::default());
    let strict = Validator::new(ValidatorConfig::strict());

    // Lenient: warning does not fail validation.
    assert!(lenient.validate(def(b.clone())).is_ok());
    // Strict: the same warning fails validation.
    assert!(strict.validate(def(b)).is_err());
}

// ----- world laws & magic (feature 11), end-to-end -----

#[test]
fn world_law_two_authoritative_laws_one_domain_rejected_end_to_end() {
    let gravity: std::collections::BTreeSet<LawDomain> =
        [LawDomain::new("physics.gravity").unwrap()].into();
    let law1 = WorldLaw::new(
        ModuleRef::new("hpdl.law.newtonian", "1.0").unwrap(),
        LawMode::Replaces(gravity.clone()),
        [LawDomain::new("physics.gravity").unwrap()],
        None,
    )
    .unwrap();
    let law2 = WorldLaw::new(
        ModuleRef::new("hpdl.law.relativistic", "1.0").unwrap(),
        LawMode::Replaces(gravity),
        [LawDomain::new("physics.gravity").unwrap()],
        None,
    )
    .unwrap();
    let b = body("dual-gravity").with_world_laws([law1, law2]);

    let result = Validator::default_config().validate(def(b));
    assert!(result.is_err(), "two laws over one domain must fail");
    if let Err(diags) = result {
        assert!(diags
            .iter()
            .any(|d| d.code == DiagnosticCode::MultipleWorldLawsForDomain));
    }
}

#[test]
fn world_law_magic_without_bridge_rejected_end_to_end() {
    let law = WorldLaw::new(
        ModuleRef::new("hpdl.law.aethermancy", "1.0").unwrap(),
        LawMode::Additive,
        [
            LawDomain::new("arcane.aether").unwrap(),
            LawDomain::new("physics.energy").unwrap(),
        ],
        None, // magic reaches physics with no bridge
    )
    .unwrap();
    let b = body("unbridged-magic").with_world_law(law);

    let result = Validator::default_config().validate(def(b));
    assert!(result.is_err(), "magic touching physics needs a bridge");
    if let Err(diags) = result {
        assert!(diags
            .iter()
            .any(|d| d.code == DiagnosticCode::MissingMagicBridge));
    }
}

#[test]
fn world_law_valid_magic_with_bridge_validates_end_to_end() {
    let bridge = MagicBridge::new(
        [LawDomain::new("arcane.aether").unwrap()],
        LawDomain::new("physics.energy").unwrap(),
        ModuleRef::new("hpdl.bridge.aether-energy", "1.0").unwrap(),
    )
    .unwrap();
    let law = WorldLaw::new(
        ModuleRef::new("hpdl.law.aethermancy", "1.0").unwrap(),
        LawMode::Additive,
        [
            LawDomain::new("arcane.aether").unwrap(),
            LawDomain::new("physics.energy").unwrap(),
        ],
        Some(bridge),
    )
    .unwrap();
    let b = body("bridged-magic").with_world_law(law);

    // Default (non-strict) config: the non-executable E6005 warning does not fail.
    let result = Validator::default_config().validate(def(b));
    assert!(
        result.is_ok(),
        "a well-formed bridged magic law should validate under default config"
    );
}

#[test]
fn world_law_unsupported_warning_does_not_block_but_strict_mode_does() {
    let law = WorldLaw::new(
        ModuleRef::new("hpdl.law.extra-gravity", "1.0").unwrap(),
        LawMode::Additive,
        [LawDomain::new("physics.gravity").unwrap()],
        None,
    )
    .unwrap();
    let b = body("extra-gravity").with_world_law(law);

    // Non-strict: passes, but emits an E6005 warning we can observe via strict mode.
    assert!(Validator::default_config().validate(def(b.clone())).is_ok());

    // Strict mode promotes the non-executable warning to a failure.
    let strict = Validator::new(ValidatorConfig::strict());
    let result = strict.validate(def(b));
    assert!(
        result.is_err(),
        "strict mode treats the warning as an error"
    );
    if let Err(diags) = result {
        assert!(diags.iter().any(|d| {
            d.code == DiagnosticCode::UnsupportedWorldLaw && d.severity == Severity::Warning
        }));
    }
}
