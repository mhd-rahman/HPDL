//! Golden hash vector tests.
//!
//! These tests verify that the canonical encoding remains stable across
//! compiler versions and platforms. The golden hashes are **frozen** and must
//! only change when the PIR format version is deliberately incremented.
//!
//! IMPORTANT: Do not regenerate these hashes to make tests pass.
//! If a hash changes, the canonical bytes changed — investigate why and bump the
//! PIR format version (+ ADR + reviewed vector update) rather than editing the
//! frozen value. The human-readable record lives in
//! `crates/hpdl-pir/tests/golden/pir_vectors.json`; the constants below are the
//! enforced copy (kept in sync with that file and with
//! `crates/hpdl-compiler/examples/emit_digests.rs`, which the CI cross-platform
//! digest-parity job runs).

use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};
use hpdl_pir::{encode_planet_definition, PirVersion};

/// Frozen content hash of the `minimal` vector (`BodyId("earth")`, default
/// physical properties, no traits/modules/statuses/actions/migrations) under PIR
/// format v2.0. Mirrors `emit_digests.rs` `minimal` and `pir_vectors.json`.
const FROZEN_MINIMAL_HASH: &str =
    "608a90a7f19de7551bcf2d72bc44e1756739434f88ae1c7a6c61d24008d9ee9f";

/// Frozen content hash of the `traits` vector (`BodyId("mars")`, Mars physical
/// properties, traits `["rocky", "thin-atmosphere"]`) under PIR format v2.0.
/// Mirrors `emit_digests.rs` `traits` and `pir_vectors.json`.
const FROZEN_TRAITS_HASH: &str = "9bc40d4ecb23bed96be2f43fe2fd6acbb8df9bc275c820cddbe9a99a26c01ede";

#[allow(clippy::unwrap_used)]
fn minimal_planet(id: &str) -> PlanetDefinition {
    let body = CelestialBody::new(
        BodyId::new(id).unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    PlanetDefinition::new("1.0", body).unwrap()
}

/// The exact `minimal` definition emitted by `hpdl-compiler`'s `emit_digests`
/// example — `BodyId("earth")` with Earth physical properties.
#[allow(clippy::unwrap_used)]
fn minimal_vector() -> PlanetDefinition {
    let body = CelestialBody::new(
        BodyId::new("earth").unwrap(),
        PhysicalProperties {
            mass_kg: Some(5_972_000_000_000_000_000_000_000),
            radius_m: Some(6_371_000),
            surface_gravity_mms2: Some(9_810),
            orbital_period_s: None,
            rotation_period_s: None,
        },
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    PlanetDefinition::new("1.0", body).unwrap()
}

/// The exact `traits` definition emitted by `emit_digests` — `BodyId("mars")`
/// with Mars physical properties and two traits.
#[allow(clippy::unwrap_used)]
fn traits_vector() -> PlanetDefinition {
    let body = CelestialBody::new(
        BodyId::new("mars").unwrap(),
        PhysicalProperties {
            mass_kg: Some(641_710_000_000_000_000_000_000),
            radius_m: Some(3_389_500),
            surface_gravity_mms2: Some(3_720),
            orbital_period_s: Some(59_354_294),
            rotation_period_s: Some(88_642),
        },
        TraitSet::new(["rocky", "thin-atmosphere"]).unwrap(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    PlanetDefinition::new("1.0", body).unwrap()
}

#[test]
fn golden_minimal_vector_matches_frozen_digest() {
    let envelope = encode_planet_definition(&minimal_vector()).expect("encoding failed");

    // Format version must be the current PIR format (v2.0 as of feature 07; see
    // docs/rfcs/0003-pir-v2-orbital-fields.md).
    assert_eq!(envelope.version, PirVersion::CURRENT);

    // The content hash must EXACTLY equal the frozen value. A mismatch means the
    // canonical bytes changed — do not edit this value to pass; bump the PIR
    // format version with an ADR and a reviewed vector update.
    assert_eq!(
        envelope.content_hash.to_hex(),
        FROZEN_MINIMAL_HASH,
        "canonical PIR hash drift for the `minimal` vector — see tests/golden/pir_vectors.json"
    );
}

#[test]
fn golden_traits_vector_matches_frozen_digest() {
    let envelope = encode_planet_definition(&traits_vector()).expect("encoding failed");
    assert_eq!(envelope.version, PirVersion::CURRENT);
    assert_eq!(
        envelope.content_hash.to_hex(),
        FROZEN_TRAITS_HASH,
        "canonical PIR hash drift for the `traits` vector — see tests/golden/pir_vectors.json"
    );
}

#[test]
fn test_golden_hash_minimal_planet() {
    // Structural invariants on a bare minimal planet (kept from the original
    // suite): 64 hex chars, non-zero, correct format version.
    let planet = minimal_planet("earth");
    let envelope = encode_planet_definition(&planet).expect("encoding failed");
    assert_eq!(envelope.version, PirVersion::CURRENT);
    let hex = envelope.content_hash.to_hex();
    assert_eq!(hex.len(), 64);
    assert!(!hex.chars().all(|c| c == '0'));
}

#[test]
fn test_golden_hash_with_id_change() {
    // Two planets that differ only in ID should have different hashes
    let planet1 = minimal_planet("earth");
    let planet2 = minimal_planet("mars");

    let env1 = encode_planet_definition(&planet1).expect("encoding failed");
    let env2 = encode_planet_definition(&planet2).expect("encoding failed");

    assert_ne!(env1.content_hash, env2.content_hash);
}

#[test]
fn test_golden_hash_determinism_across_encodings() {
    // The same planet encoded multiple times should always produce the same hash
    let planet = minimal_planet("venus");

    let env1 = encode_planet_definition(&planet).expect("encoding 1 failed");
    let env2 = encode_planet_definition(&planet).expect("encoding 2 failed");
    let env3 = encode_planet_definition(&planet).expect("encoding 3 failed");

    assert_eq!(env1.content_hash, env2.content_hash);
    assert_eq!(env2.content_hash, env3.content_hash);
    assert_eq!(env1.payload, env2.payload);
}
