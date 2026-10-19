//! Golden build fixture — freezes the canonical content hash and the build
//! manifest text for a known planet so cross-platform reproducibility is
//! verifiable.
//!
//! Updating the frozen fixture must be a deliberate, reviewed change. If the
//! canonical PIR bytes themselves change, that requires a PIR format-version
//! decision in `hpdl-pir`, and the fixture must never be silently regenerated to
//! make this test pass.
//!
//! This fixture was updated for PIR format v2.0 (feature 07): adding optional
//! orbital fields to the canonically-encoded `CelestialBody` changed canonical
//! PIR bytes, so the format major version and hash domain separator were bumped.
//! See `docs/rfcs/0003-pir-v2-orbital-fields.md`.
//!
//! It was updated again for feature 08 (planetary environments), which added an
//! optional `environment` field to the same struct. Because PIR v2.0 was still
//! unreleased, this was folded into v2.0 (OD-08.1, option A) rather than bumping
//! the format version — the hash changed deliberately, the version did not. See
//! the amendment in `docs/rfcs/0003-pir-v2-orbital-fields.md`.
//!
//! The canonical serializer was later migrated from bincode 1.3.3 to the
//! bincode-wire-compatible wincode (ADR 0015). That swap is **byte-neutral**:
//! this frozen hash is unchanged, which is exactly what proves the migration did
//! not alter canonical PIR bytes.

use hpdl_compiler::{CompileOptions, Compiler};

/// Fixed planet source. Changing this changes the golden fixture by design.
const GOLDEN_SOURCE: &str = r#"
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("golden"),
        properties: PhysicalProperties(
            mass_kg: Some(1234567890000000000000000),
            radius_m: Some(6000000),
            surface_gravity_mms2: Some(9800),
            orbital_period_s: Some(31557600),
            rotation_period_s: Some(86400),
        ),
        traits: TraitSet([]),
        modules: [],
        statuses: [],
        actions: [],
        migrations: [],
    )
)
"#;

/// The frozen manifest text for `GOLDEN_SOURCE`.
const GOLDEN_MANIFEST: &str = include_str!("golden/minimal_build.txt");

#[test]
fn golden_build_is_reproducible() {
    let compiler = Compiler::new(CompileOptions::default());
    let result = compiler
        .compile_ron_str("golden", GOLDEN_SOURCE)
        .expect("golden compile");

    let actual = result.manifest().to_text();

    assert_eq!(
        actual, GOLDEN_MANIFEST,
        "golden manifest drift detected.\n--- expected ---\n{GOLDEN_MANIFEST}\n--- actual ---\n{actual}\n\
         If this change is intentional, update tests/golden/minimal_build.txt deliberately."
    );

    // The content hash is embedded in the manifest; also assert integrity.
    assert!(result.envelope().verify_hash());
}
