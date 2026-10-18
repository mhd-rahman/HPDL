//! End-to-end module-registry resolution through the compiler (ADR 0011).
//!
//! Proves that a configured `ModuleRegistry` lets a definition that declares an
//! evaluator compile (the evaluator resolves), that an unregistered reference
//! fails, and that attaching a registry does not change canonical PIR bytes.

use hpdl_compiler::{
    CompileOptions, Compiler, Digest, ModuleKind, ModuleRegistry, PackageId, RegisteredModule,
};

/// A RON planet declaring a status with an evaluator + window + cut points.
/// Without a registry this fails (E9202); with a matching registry it compiles.
const WITH_EVALUATOR: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("earth"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),
            radius_m: Some(6371000),
            surface_gravity_mms2: None,
            orbital_period_s: None,
            rotation_period_s: None,
        ),
        traits: TraitSet([]),
        modules: [],
        statuses: [
            StatusDeclaration(
                name: "Vitality",
                scale_min: 0,
                scale_max: 100,
                evaluator: Some(EvaluatorRef(module: ModuleRef(name: "hpdl.vitality.eco", version: "1.0.0"))),
                window: Some(EvaluationWindow(span: 1000, scope: None)),
                cut_points: Some(CutPoints(scale_min: 0, scale_max: 100, cuts: [(0, "low"), (50, "high")])),
            ),
        ],
        actions: [],
        migrations: [],
    )
)
"#;

/// The same planet with no evaluator — compiles with or without a registry.
const PLAIN: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("earth"),
        properties: PhysicalProperties(
            mass_kg: Some(5972000000000000000000000),
            radius_m: Some(6371000),
            surface_gravity_mms2: None,
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

fn registry() -> ModuleRegistry {
    ModuleRegistry::from_modules([RegisteredModule::new(
        PackageId::new("hpdl.vitality.eco", "1.0.0").unwrap(),
        ModuleKind::Evaluator,
        Digest::new("a".repeat(64)).unwrap(),
    )])
}

#[test]
fn evaluator_fails_without_registry() {
    let compiler = Compiler::new(CompileOptions::default());
    let result = compiler.compile_ron_str("earth", WITH_EVALUATOR);
    assert!(
        result.is_err(),
        "an evaluator with no registry must fail (E9202)"
    );
}

#[test]
fn evaluator_compiles_with_matching_registry() {
    let options = CompileOptions::default().with_registry(registry());
    let compiler = Compiler::new(options);
    let result = compiler.compile_ron_str("earth", WITH_EVALUATOR);
    assert!(
        result.is_ok(),
        "a registered evaluator must resolve and compile: {:?}",
        result.err()
    );
}

#[test]
fn unregistered_evaluator_fails_with_registry() {
    // Registry present, but registers a different module than the one referenced.
    let other = ModuleRegistry::from_modules([RegisteredModule::new(
        PackageId::new("hpdl.vitality.other", "1.0.0").unwrap(),
        ModuleKind::Evaluator,
        Digest::new("a".repeat(64)).unwrap(),
    )]);
    let options = CompileOptions::default().with_registry(other);
    let compiler = Compiler::new(options);
    assert!(compiler.compile_ron_str("earth", WITH_EVALUATOR).is_err());
}

#[test]
fn registry_does_not_change_pir_bytes() {
    // A definition that compiles with and without a registry must hash identically:
    // the registry is a build input, not part of the canonical payload.
    let without = Compiler::new(CompileOptions::default())
        .compile_ron_str("earth", PLAIN)
        .expect("compiles without registry");
    let with = Compiler::new(CompileOptions::default().with_registry(registry()))
        .compile_ron_str("earth", PLAIN)
        .expect("compiles with registry");
    assert_eq!(
        without.envelope().payload,
        with.envelope().payload,
        "attaching a registry must not change canonical PIR bytes"
    );
    assert_eq!(
        without.envelope().content_hash,
        with.envelope().content_hash
    );
}
