//! End-to-end pipeline integration tests (positive and negative paths).

use hpdl_compiler::{BuildLock, CompileOptions, Compiler, Error, ResourceLimits};

/// A minimal valid RON planet with no module references.
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

/// A valid RON planet that declares one module reference.
const WITH_MODULE: &str = r#"
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
        modules: [ModuleRef(name: "hpdl.atmosphere", version: "1.2.0")],
        statuses: [],
        actions: [],
        migrations: [],
    )
)
"#;

#[test]
fn minimal_compiles_and_verifies() {
    let compiler = Compiler::new(CompileOptions::default());
    let result = compiler.compile_ron_str("earth", MINIMAL).expect("compile");

    assert!(result.envelope().verify_hash());
    assert_eq!(result.content_hash_hex().len(), 64);

    let manifest = result.manifest();
    assert_eq!(manifest.source_name, "earth");
    // PIR format v2 as of feature 07 (orbital fields); see ADR 0003.
    assert_eq!(manifest.pir_version.major, 2);
    assert!(manifest.modules.is_empty());
}

#[test]
fn oversized_source_is_limit_error() {
    let limits = ResourceLimits {
        max_source_bytes: 16,
        max_modules: 1024,
    };
    let compiler = Compiler::new(CompileOptions::default().with_limits(limits));
    let err = compiler.compile_ron_str("earth", MINIMAL).unwrap_err();
    assert!(matches!(err, Error::Limit(_)));
}

#[test]
fn malformed_ron_is_frontend_error() {
    let compiler = Compiler::new(CompileOptions::default());
    let err = compiler
        .compile_ron_str("bad", "not ron at all {{{")
        .unwrap_err();
    assert!(matches!(err, Error::Frontend { .. }));
}

#[test]
fn module_without_lock_is_unresolved() {
    let compiler = Compiler::new(CompileOptions::default());
    let err = compiler.compile_ron_str("earth", WITH_MODULE).unwrap_err();
    assert!(matches!(err, Error::UnresolvedModule { .. }));
}

#[test]
fn module_with_matching_lock_compiles() {
    let lock = BuildLock::parse("hpdl.atmosphere 1.2.0 deadbeef").unwrap();
    let compiler = Compiler::new(CompileOptions::default().with_lock(lock));
    let result = compiler
        .compile_ron_str("earth", WITH_MODULE)
        .expect("compile");

    let modules = &result.manifest().modules;
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].name, "hpdl.atmosphere");
    assert_eq!(modules[0].version, "1.2.0");
    assert_eq!(modules[0].content_hash, "deadbeef");
}

#[test]
fn module_version_mismatch_fails() {
    let lock = BuildLock::parse("hpdl.atmosphere 9.9.9 deadbeef").unwrap();
    let compiler = Compiler::new(CompileOptions::default().with_lock(lock));
    let err = compiler.compile_ron_str("earth", WITH_MODULE).unwrap_err();
    assert!(matches!(err, Error::ModuleVersionMismatch { .. }));
}

#[test]
fn max_modules_limit_enforced() {
    let lock = BuildLock::parse("hpdl.atmosphere 1.2.0 deadbeef").unwrap();
    let options = CompileOptions::default()
        .with_lock(lock)
        .with_limits(ResourceLimits {
            max_source_bytes: usize::MAX,
            max_modules: 0,
        });
    let compiler = Compiler::new(options);
    let err = compiler.compile_ron_str("earth", WITH_MODULE).unwrap_err();
    assert!(matches!(err, Error::Limit(_)));
}

#[test]
fn lock_parse_wrong_arity_is_lock_parse_error() {
    let err = BuildLock::parse("hpdl.atmosphere 1.2.0").unwrap_err();
    assert!(matches!(err, Error::LockParse { .. }));
}

#[test]
fn lock_parse_duplicate_is_lock_parse_error() {
    let err = BuildLock::parse("m 1.0.0 aaaa\nm 1.0.0 bbbb").unwrap_err();
    assert!(matches!(err, Error::LockParse { line: 2, .. }));
}
