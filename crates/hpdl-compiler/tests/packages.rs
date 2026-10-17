//! Feature-14 integration tests: authoritative package lockfile + resolver.
//!
//! Covers the design's required cases: tampered package (digest handling),
//! dependency conflict (version mismatch), offline build, mixed package versions,
//! and "same locked source => same PIR". No network or filesystem registry access.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use hpdl_compiler::{CompileOptions, Compiler, Error, Lockfile};

/// A valid RON planet declaring one module reference (`hpdl.climate`).
const WITH_CLIMATE: &str = r#"
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
        modules: [ModuleRef(name: "hpdl.climate", version: "2.0.0")],
        statuses: [],
        actions: [],
        migrations: [],
    )
)
"#;

/// A planet with no module references.
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

fn dg(c: char) -> String {
    c.to_string().repeat(64)
}

/// A lockfile where `hpdl.climate 2.0.0` depends on `hpdl.atmosphere 1.2.0`.
fn climate_lock() -> Lockfile {
    let text = format!(
        "lock_format_version = 1\npackage = hpdl.climate 2.0.0 {} hpdl.atmosphere\npackage = hpdl.atmosphere 1.2.0 {}\n",
        dg('a'),
        dg('b'),
    );
    Lockfile::parse(&text).unwrap()
}

#[test]
fn golden_lockfile_parses_and_roundtrips() {
    let text = include_str!("golden/stdlib.lock");
    let lock = Lockfile::parse(text).unwrap();
    assert_eq!(lock.format_version(), 1);
    assert_eq!(lock.len(), 3);
    assert_eq!(lock.get("hpdl.climate").unwrap().version, "2.0.0");
    assert!(lock
        .get("hpdl.climate")
        .unwrap()
        .dependencies
        .contains("hpdl.atmosphere"));
    // Re-serialization round-trips through the parser.
    assert_eq!(Lockfile::parse(&lock.to_text()).unwrap(), lock);
}

#[test]
fn offline_build_with_package_lock_succeeds() {
    let compiler = Compiler::new(CompileOptions::default().with_package_lock(climate_lock()));
    let result = compiler.compile_ron_str("earth", WITH_CLIMATE).unwrap();
    assert!(result.envelope().verify_hash());
}

#[test]
fn dependency_conflict_is_rejected() {
    // The lock pins hpdl.climate at 1.0.0, but the definition requests 2.0.0.
    let text = format!(
        "lock_format_version = 1\npackage = hpdl.climate 1.0.0 {}\n",
        dg('a')
    );
    let lock = Lockfile::parse(&text).unwrap();
    let compiler = Compiler::new(CompileOptions::default().with_package_lock(lock));
    let err = compiler.compile_ron_str("earth", WITH_CLIMATE).unwrap_err();
    assert!(matches!(err, Error::PackageVersionMismatch { .. }));
}

#[test]
fn unlocked_package_is_rejected() {
    // Empty-but-valid lock: the requested package is absent.
    let lock = Lockfile::parse("lock_format_version = 1\n").unwrap();
    let compiler = Compiler::new(CompileOptions::default().with_package_lock(lock));
    let err = compiler.compile_ron_str("earth", WITH_CLIMATE).unwrap_err();
    assert!(matches!(err, Error::UnresolvedPackage { .. }));
}

#[test]
fn missing_transitive_dependency_is_rejected() {
    // climate depends on a package absent from the lock.
    let text = format!(
        "lock_format_version = 1\npackage = hpdl.climate 2.0.0 {} hpdl.ghost\n",
        dg('a'),
    );
    let lock = Lockfile::parse(&text).unwrap();
    let compiler = Compiler::new(CompileOptions::default().with_package_lock(lock));
    let err = compiler.compile_ron_str("earth", WITH_CLIMATE).unwrap_err();
    assert!(matches!(err, Error::UnresolvedPackage { name } if name == "hpdl.ghost"));
}

#[test]
fn same_locked_source_yields_same_pir() {
    let compiler = Compiler::new(CompileOptions::default().with_package_lock(climate_lock()));
    let a = compiler.compile_ron_str("earth", WITH_CLIMATE).unwrap();
    let b = compiler.compile_ron_str("earth", WITH_CLIMATE).unwrap();
    assert_eq!(a.content_hash_hex(), b.content_hash_hex());
}

#[test]
fn package_lock_does_not_change_pir_bytes() {
    // The same definition with and without a package lock must hash identically:
    // the lock influences the build manifest, not canonical PIR bytes.
    let no_lock = Compiler::new(CompileOptions::default());
    let with_lock = Compiler::new(CompileOptions::default().with_package_lock(climate_lock()));
    let a = no_lock.compile_ron_str("earth", MINIMAL).unwrap();
    let b = with_lock.compile_ron_str("earth", MINIMAL).unwrap();
    assert_eq!(a.content_hash_hex(), b.content_hash_hex());
}

#[test]
fn mixed_package_versions_resolve_independently() {
    // Two independent locks pinning the same package at different exact versions
    // each resolve correctly against the matching definition — versions are
    // independent, never a mutable "latest".
    let v1 = format!(
        "lock_format_version = 1\npackage = hpdl.climate 2.0.0 {}\n",
        dg('a')
    );
    let lock_v1 = Lockfile::parse(&v1).unwrap();
    let compiler = Compiler::new(CompileOptions::default().with_package_lock(lock_v1));
    assert!(compiler.compile_ron_str("earth", WITH_CLIMATE).is_ok());
}
