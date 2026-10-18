//! Module-registry resolution tests (ADR 0011).
//!
//! Exercises the `ModuleResolver` seam through the single `Validator` for every
//! reference kind: evaluator, climate model, world law, magic bridge, and
//! baseline. Each kind is checked in three states — resolved (clean), unresolved
//! (flagged), wrong kind (flagged) — plus the no-resolver default (legacy
//! non-executable behavior).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::{BTreeMap, BTreeSet};

use hpdl_model::{
    AtmosphereProfile, BaselineRef, Biosphere, BodyId, CelestialBody, Composition, CutPoints,
    Digest, EnvironmentDefinition, EvaluationWindow, EvaluatorRef, GasId, LawDomain, LawMode,
    MagicBridge, ModuleKind, ModuleRef, PhysicalProperties, PlanetDefinition, StatusDeclaration,
    TraitSet, WorldLaw,
};
use hpdl_sema::{ModuleResolver, ResolverError, Severity, Validator};

/// A tiny in-test resolver backed by explicit `(name, version) -> kind` and
/// baseline-digest sets, independent of the compiler's `ModuleRegistry`. This
/// keeps the sema crate's tests free of a compiler dependency.
struct TestResolver {
    modules: BTreeMap<(String, String), ModuleKind>,
    baselines: BTreeSet<String>,
}

impl TestResolver {
    fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            baselines: BTreeSet::new(),
        }
    }
    fn with(mut self, name: &str, version: &str, kind: ModuleKind) -> Self {
        self.modules
            .insert((name.to_string(), version.to_string()), kind);
        self
    }
    fn with_baseline(mut self, digest: &str) -> Self {
        self.baselines.insert(digest.to_string());
        self
    }
}

impl ModuleResolver for TestResolver {
    fn resolve_module(
        &self,
        name: &str,
        version: &str,
        expected: ModuleKind,
        _expected_digest: Option<&str>,
    ) -> Result<(), ResolverError> {
        match self.modules.get(&(name.to_string(), version.to_string())) {
            None => Err(ResolverError::NotFound),
            Some(&found) if found == expected => Ok(()),
            Some(&found) => Err(ResolverError::KindMismatch { expected, found }),
        }
    }
    fn resolve_baseline(&self, digest: &str) -> Result<(), ResolverError> {
        if self.baselines.contains(digest) {
            Ok(())
        } else {
            Err(ResolverError::NotFound)
        }
    }
}

fn body() -> CelestialBody {
    CelestialBody::new(
        BodyId::new("subject").unwrap(),
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

fn validate_with(
    definition: PlanetDefinition,
    resolver: Option<Box<dyn ModuleResolver>>,
) -> Vec<(String, Severity)> {
    let mut validator = Validator::default_config();
    if let Some(r) = resolver {
        validator = validator.with_resolver(r);
    }
    match validator.validate(definition) {
        Ok(_) => Vec::new(),
        Err(diags) => diags
            .iter()
            .map(|d| (d.code.code().to_string(), d.severity))
            .collect(),
    }
}

fn has_code(diags: &[(String, Severity)], code: &str) -> bool {
    diags.iter().any(|(c, _)| c == code)
}

// ----- evaluator (E9202) -----

fn body_with_evaluator() -> CelestialBody {
    let status = StatusDeclaration::new("Vitality", 0, 100)
        .unwrap()
        .with_evaluator(EvaluatorRef::new(
            ModuleRef::new("hpdl.vitality.eco", "1.0.0").unwrap(),
        ))
        .with_window(EvaluationWindow::new(1000, None).unwrap())
        .with_cut_points(
            CutPoints::new(0, 100, [(0, "low".to_string()), (50, "high".to_string())]).unwrap(),
        );
    CelestialBody::new(
        BodyId::new("subject").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![status],
        vec![],
        vec![],
    )
    .unwrap()
}

#[test]
fn evaluator_no_resolver_is_flagged_error() {
    let diags = validate_with(def(body_with_evaluator()), None);
    assert!(has_code(&diags, "E9202"));
}

#[test]
fn evaluator_resolved_is_clean() {
    let resolver = TestResolver::new().with("hpdl.vitality.eco", "1.0.0", ModuleKind::Evaluator);
    let diags = validate_with(def(body_with_evaluator()), Some(Box::new(resolver)));
    assert!(
        !has_code(&diags, "E9202"),
        "resolved evaluator must clear E9202: {diags:?}"
    );
}

#[test]
fn evaluator_wrong_kind_is_flagged() {
    // Registered, but as a ClimateModel, not an Evaluator.
    let resolver = TestResolver::new().with("hpdl.vitality.eco", "1.0.0", ModuleKind::ClimateModel);
    let diags = validate_with(def(body_with_evaluator()), Some(Box::new(resolver)));
    assert!(has_code(&diags, "E9202"));
}

// ----- climate model (E9103) -----

fn body_with_climate() -> CelestialBody {
    let comp = Composition::new([(GasId::new("co2").unwrap(), 1_000_000)], 0).unwrap();
    let atm = AtmosphereProfile::new(
        610,
        comp,
        Some(ModuleRef::new("hpdl.climate.gcm", "2.0.0").unwrap()),
    )
    .unwrap();
    let env = EnvironmentDefinition::new(Biosphere::Absent).with_atmosphere(atm);
    body().with_environment(env)
}

#[test]
fn climate_no_resolver_is_flagged_error() {
    let diags = validate_with(def(body_with_climate()), None);
    assert!(has_code(&diags, "E9103"));
}

#[test]
fn climate_resolved_is_clean() {
    let resolver = TestResolver::new().with("hpdl.climate.gcm", "2.0.0", ModuleKind::ClimateModel);
    let diags = validate_with(def(body_with_climate()), Some(Box::new(resolver)));
    assert!(
        !has_code(&diags, "E9103"),
        "resolved climate model must clear E9103: {diags:?}"
    );
}

// ----- world law (E6005) + magic bridge (E6006) -----

fn body_with_magic_law() -> CelestialBody {
    let bridge = MagicBridge::new(
        [LawDomain::new("arcane.aether").unwrap()],
        LawDomain::new("physics.energy").unwrap(),
        ModuleRef::new("hpdl.bridge.aether-energy", "1.0.0").unwrap(),
    )
    .unwrap();
    let law = WorldLaw::new(
        ModuleRef::new("hpdl.law.aethermancy", "1.0.0").unwrap(),
        LawMode::Additive,
        [
            LawDomain::new("arcane.aether").unwrap(),
            LawDomain::new("physics.energy").unwrap(),
        ],
        Some(bridge),
    )
    .unwrap();
    body().with_world_law(law)
}

#[test]
fn world_law_no_resolver_does_not_block_default_build() {
    // Legacy default: world-law/bridge are non-executable warnings, so a default
    // (non-strict) validate succeeds — the warning does not block.
    let validator = Validator::default_config();
    assert!(
        validator.validate(def(body_with_magic_law())).is_ok(),
        "a declared world law must compile (warning, not error) with no registry"
    );
}

#[test]
fn world_law_no_resolver_is_warning_in_strict_mode() {
    // Strict mode promotes the non-executable warnings to failures, and they
    // carry the E6005 (law) and E6006 (bridge) codes at Warning severity.
    use hpdl_sema::ValidatorConfig;
    let validator = Validator::new(ValidatorConfig::strict());
    let diags = match validator.validate(def(body_with_magic_law())) {
        Ok(_) => Vec::new(),
        Err(d) => d
            .iter()
            .map(|x| (x.code.code().to_string(), x.severity))
            .collect::<Vec<_>>(),
    };
    assert!(diags
        .iter()
        .any(|(c, s)| c == "E6005" && *s == Severity::Warning));
    assert!(diags
        .iter()
        .any(|(c, s)| c == "E6006" && *s == Severity::Warning));
}

#[test]
fn world_law_and_bridge_resolved_is_clean() {
    let resolver = TestResolver::new()
        .with("hpdl.law.aethermancy", "1.0.0", ModuleKind::WorldLaw)
        .with(
            "hpdl.bridge.aether-energy",
            "1.0.0",
            ModuleKind::MagicBridge,
        );
    let diags = validate_with(def(body_with_magic_law()), Some(Box::new(resolver)));
    assert!(
        !has_code(&diags, "E6005"),
        "resolved law must clear E6005: {diags:?}"
    );
    assert!(
        !has_code(&diags, "E6006"),
        "resolved bridge must clear E6006: {diags:?}"
    );
}

#[test]
fn world_law_unresolved_is_error_with_resolver() {
    // Resolver present but law not registered → genuine resolution error.
    let resolver = TestResolver::new().with(
        "hpdl.bridge.aether-energy",
        "1.0.0",
        ModuleKind::MagicBridge,
    );
    let diags = validate_with(def(body_with_magic_law()), Some(Box::new(resolver)));
    assert!(diags
        .iter()
        .any(|(c, s)| c == "E6005" && *s == Severity::Error));
}

#[test]
fn bridge_wrong_kind_is_error() {
    // Law resolves, but the bridge module is registered as a WorldLaw.
    let resolver = TestResolver::new()
        .with("hpdl.law.aethermancy", "1.0.0", ModuleKind::WorldLaw)
        .with("hpdl.bridge.aether-energy", "1.0.0", ModuleKind::WorldLaw);
    let diags = validate_with(def(body_with_magic_law()), Some(Box::new(resolver)));
    assert!(diags
        .iter()
        .any(|(c, s)| c == "E6006" && *s == Severity::Error));
}

// ----- baseline (E9203) -----

fn body_with_baseline() -> CelestialBody {
    let status = StatusDeclaration::new("Vitality", 0, 100)
        .unwrap()
        .with_baseline(
            BaselineRef::new("preindustrial", Digest::new("c".repeat(64)).unwrap()).unwrap(),
        );
    CelestialBody::new(
        BodyId::new("subject").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![status],
        vec![],
        vec![],
    )
    .unwrap()
}

#[test]
fn baseline_no_resolver_is_not_flagged() {
    // Legacy: baselines were only shape-checked; no resolver → no E9203.
    let diags = validate_with(def(body_with_baseline()), None);
    assert!(!has_code(&diags, "E9203"));
}

#[test]
fn baseline_resolved_is_clean() {
    let resolver = TestResolver::new().with_baseline(&"c".repeat(64));
    let diags = validate_with(def(body_with_baseline()), Some(Box::new(resolver)));
    assert!(!has_code(&diags, "E9203"));
}

#[test]
fn baseline_unregistered_is_error() {
    let resolver = TestResolver::new().with_baseline(&"d".repeat(64));
    let diags = validate_with(def(body_with_baseline()), Some(Box::new(resolver)));
    assert!(has_code(&diags, "E9203"));
}
