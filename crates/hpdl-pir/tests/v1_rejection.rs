//! End-to-end v1 → v2 rejection (ADR 0014: PIR v2.0 ratified).
//!
//! The in-crate unit tests prove the *version-comparison helper*
//! (`PirVersion::is_compatible`) and a forced-`{99,0}` decode. These tests close
//! the remaining gap in the compatibility story the ratification ADR documents:
//! they drive a **real v1-stamped envelope** through the public
//! `decode_planet_definition` entry point and assert the documented hard break.
//!
//! The intended contract (ADR 0003 + ADR 0014): a v2 decoder **rejects** v1 bytes
//! with [`Error::IncompatibleVersion`]; there is no v1 → v2 migration.

use hpdl_pir::{
    decode_planet_definition, encode_planet_definition, Error, PirEnvelope, PirVersion,
};

// The envelope is serialized with wincode — the same serializer the decoder now
// uses (ADR 0015). The v1-rejection contract is serializer-independent: a v1
// version stamp is rejected regardless of which wire-compatible encoder produced
// the bytes.

/// Build a v2 planet payload we can reuse: the canonical payload bytes of a
/// minimal definition, extracted from a real v2 envelope.
#[allow(clippy::unwrap_used)]
fn v2_minimal_payload() -> Vec<u8> {
    use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};
    let body = CelestialBody::new(
        BodyId::new("earth").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    let planet = PlanetDefinition::new("1.0", body).unwrap();
    // `encode_planet_definition` stamps `PirVersion::CURRENT` (2.0); we only want
    // its payload bytes to reuse under a forged v1 version stamp below.
    encode_planet_definition(&planet).unwrap().payload
}

/// A genuine v1-stamped envelope — version `{1,0}`, with a **valid** content hash
/// over its payload (so the decoder's hash check passes and the failure is
/// unambiguously the version gate, not corruption) — must be rejected by the v2
/// decoder with `IncompatibleVersion { found: 1.0, expected: 2.0 }`.
#[allow(clippy::unwrap_used)]
#[test]
fn v2_decoder_rejects_v1_envelope_with_valid_hash() {
    // `PirEnvelope::new` computes the correct content hash over the payload, so
    // this envelope is internally consistent and only differs from a current
    // artifact by its format version.
    let envelope = PirEnvelope::new(PirVersion::new(1, 0), b"any-v1-payload".to_vec());
    assert!(
        envelope.verify_hash(),
        "precondition: the forged v1 envelope must have a valid content hash so \
         the decode fails on the version gate, not the hash check"
    );

    let bytes = wincode::serialize(&envelope).unwrap();
    let result = decode_planet_definition(&bytes);

    match result {
        Err(Error::IncompatibleVersion { found, expected }) => {
            assert_eq!(found, PirVersion::new(1, 0), "found version should be v1.0");
            assert_eq!(
                expected,
                PirVersion::CURRENT,
                "expected version should be the current (v2.0) decoder"
            );
            assert_eq!(expected, PirVersion::new(2, 0));
        }
        other => panic!("expected Error::IncompatibleVersion, got {other:?}"),
    }
}

/// Even if the v1 envelope carries bytes that *would* deserialize fine under v2
/// (a real v2 payload) but is stamped `{1,0}`, the version gate still rejects it
/// up front — the decoder never falls through to payload decode on a wrong major.
/// This pins that the break is decided by the version field, not incidentally by
/// the payload failing to parse.
#[allow(clippy::unwrap_used)]
#[test]
fn v2_decoder_rejects_v1_stamp_even_with_v2_shaped_payload() {
    let payload = v2_minimal_payload();
    // Stamp it v1 but keep a valid hash over the (v2-shaped) payload.
    let forged = PirEnvelope::new(PirVersion::new(1, 0), payload);
    assert!(forged.verify_hash());

    let bytes = wincode::serialize(&forged).unwrap();
    let result = decode_planet_definition(&bytes);

    assert!(
        matches!(result, Err(Error::IncompatibleVersion { .. })),
        "a v1 version stamp must be rejected before payload decode, got {result:?}"
    );
}

/// Sanity anchor for the ratified current format: the canonical encoder produces
/// `{2,0}`, and that same artifact decodes successfully — so the rejection tests
/// above are rejecting *only* because of the v1 stamp, not a broken pipeline.
#[allow(clippy::unwrap_used)]
#[test]
fn v2_decoder_accepts_current_v2_artifact() {
    use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};
    let body = CelestialBody::new(
        BodyId::new("earth").unwrap(),
        PhysicalProperties::default(),
        TraitSet::empty(),
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .unwrap();
    let planet = PlanetDefinition::new("1.0", body).unwrap();

    let envelope = encode_planet_definition(&planet).unwrap();
    assert_eq!(envelope.version, PirVersion::CURRENT);
    assert_eq!(envelope.version, PirVersion::new(2, 0));

    let bytes = wincode::serialize(&envelope).unwrap();
    let decoded = decode_planet_definition(&bytes).unwrap();
    assert_eq!(decoded.body().id(), planet.body().id());
}
