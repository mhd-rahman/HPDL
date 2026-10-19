//! Canonical decoding for planet definitions.

use crate::encoding::{CANONICAL_CONFIG, MAX_PIR_SIZE};
use crate::envelope::PirEnvelope;
use crate::error::Error;
use crate::version::PirVersion;
use hpdl_model::PlanetDefinition;

/// Decodes a planet definition from canonical PIR bytes.
///
/// # Arguments
/// * `encoded` - The raw PIR-encoded bytes (including envelope).
///
/// # Returns
/// The decoded `PlanetDefinition` on success.
///
/// # Errors
/// * `Error::InputTooLarge` - Input exceeds `MAX_PIR_SIZE`.
/// * `Error::MalformedData` - Envelope deserialization failed or hash mismatch.
/// * `Error::IncompatibleVersion` - Unsupported PIR format version.
///
/// # Example
/// ```
/// use hpdl_model::{PlanetDefinition, BodyId, CelestialBody, PhysicalProperties, TraitSet};
/// use hpdl_pir::{encode_planet_definition, decode_planet_definition};
///
/// let body = CelestialBody::new(
///     BodyId::new("earth").unwrap(),
///     PhysicalProperties::default(),
///     TraitSet::empty(),
///     vec![], vec![], vec![], vec![],
/// ).unwrap();
/// let original = PlanetDefinition::new("1.0", body).unwrap();
///
/// let envelope = encode_planet_definition(&original).expect("encoding failed");
/// let envelope_bytes = wincode::serialize(&envelope).expect("envelope serialization failed");
/// let decoded = decode_planet_definition(&envelope_bytes).expect("decoding failed");
/// assert_eq!(decoded.body().id(), original.body().id());
/// ```
pub fn decode_planet_definition(encoded: &[u8]) -> Result<PlanetDefinition, Error> {
    // Enforce size limit before attempting deserialization
    if encoded.len() > MAX_PIR_SIZE {
        return Err(Error::InputTooLarge {
            size: encoded.len(),
            max: MAX_PIR_SIZE,
        });
    }

    // Deserialize the envelope with the canonical wincode config (default wire
    // format; preallocation limit raised to MAX_PIR_SIZE so decode accepts the
    // same size range it did pre-migration — see CANONICAL_CONFIG).
    let envelope: PirEnvelope = wincode::config::deserialize(encoded, CANONICAL_CONFIG).map_err(
        |e| Error::MalformedData {
            reason: format!("failed to deserialize envelope: {}", e),
        },
    )?;

    // Verify content hash
    if !envelope.verify_hash() {
        return Err(Error::MalformedData {
            reason: "content hash mismatch".to_string(),
        });
    }

    // Check version compatibility
    if !PirVersion::CURRENT.is_compatible(&envelope.version) {
        return Err(Error::IncompatibleVersion {
            found: envelope.version,
            expected: PirVersion::CURRENT,
        });
    }

    // Check payload size separately (defense in depth)
    if envelope.payload.len() > MAX_PIR_SIZE {
        return Err(Error::InputTooLarge {
            size: envelope.payload.len(),
            max: MAX_PIR_SIZE,
        });
    }

    // Deserialize the planet definition from the payload with the canonical config.
    let planet: PlanetDefinition = wincode::config::deserialize(&envelope.payload, CANONICAL_CONFIG)
        .map_err(|e| Error::MalformedData {
            reason: format!("failed to deserialize planet definition: {}", e),
        })?;

    Ok(planet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode_planet_definition;
    use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, TraitSet};

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

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_decode_minimal_planet() {
        let original = minimal_planet("test");

        let envelope = encode_planet_definition(&original).unwrap();
        let envelope_bytes = wincode::serialize(&envelope).unwrap();
        let decoded = decode_planet_definition(&envelope_bytes).unwrap();

        assert_eq!(decoded.body().id(), original.body().id());
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_roundtrip() {
        let original = minimal_planet("roundtrip-test");

        let envelope = encode_planet_definition(&original).unwrap();
        let envelope_bytes = wincode::serialize(&envelope).unwrap();
        let decoded = decode_planet_definition(&envelope_bytes).unwrap();

        assert_eq!(decoded, original);
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_decode_rejects_oversized_input() {
        let oversized = vec![0u8; MAX_PIR_SIZE + 1];
        let result = decode_planet_definition(&oversized);
        assert!(matches!(result, Err(Error::InputTooLarge { .. })));
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_decode_rejects_malformed_data() {
        let garbage = b"not valid PIR data";
        let result = decode_planet_definition(garbage);
        assert!(matches!(result, Err(Error::MalformedData { .. })));
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_decode_rejects_corrupted_hash() {
        let original = minimal_planet("test");

        let mut envelope = encode_planet_definition(&original).unwrap();
        // Corrupt the payload
        if let Some(byte) = envelope.payload.first_mut() {
            *byte = byte.wrapping_add(1);
        }

        let envelope_bytes = wincode::serialize(&envelope).unwrap();
        let result = decode_planet_definition(&envelope_bytes);
        assert!(matches!(result, Err(Error::MalformedData { .. })));
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_decode_rejects_incompatible_version() {
        let original = minimal_planet("test");

        let mut envelope = encode_planet_definition(&original).unwrap();
        // Change version to unsupported major version
        envelope.version = PirVersion::new(99, 0);

        let envelope_bytes = wincode::serialize(&envelope).unwrap();
        let result = decode_planet_definition(&envelope_bytes);
        assert!(matches!(result, Err(Error::IncompatibleVersion { .. })));
    }

    /// An envelope that is itself well-formed — valid content hash, compatible
    /// version, payload under the size cap — but whose `payload` bytes are not a
    /// valid `PlanetDefinition` must fail at the **payload** deserialize step with
    /// `MalformedData`. This exercises the second (inner) decode path, distinct
    /// from `test_decode_rejects_malformed_data` which fails at the outer envelope
    /// deserialize. Building via `PirEnvelope::new` gives a matching hash so the
    /// hash and version gates pass and decode reaches the payload decode.
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_decode_rejects_invalid_payload() {
        // Not a valid encoded PlanetDefinition, but short enough to pass the size
        // caps and hashed consistently so the content-hash check passes.
        let envelope = PirEnvelope::new(PirVersion::CURRENT, vec![0xFF, 0x00, 0x13, 0x37]);
        assert!(envelope.verify_hash());

        let bytes = wincode::serialize(&envelope).unwrap();
        let result = decode_planet_definition(&bytes);
        match result {
            Err(Error::MalformedData { reason }) => {
                assert!(
                    reason.contains("planet definition"),
                    "expected the payload-decode reason, got: {reason}"
                );
            }
            other => panic!("expected MalformedData from an invalid payload, got {other:?}"),
        }
    }

    /// Byte offset of the envelope's `payload` length prefix under the canonical
    /// canonical encoding (fixint, little-endian — identical under wincode and
    /// bincode's default config):
    ///
    /// - `version: PirVersion { major: u16, minor: u16 }` → 4 bytes
    /// - `content_hash: CanonicalHash([u8; 32])` → 32 bytes, no length prefix
    /// - `payload: Vec<u8>` → `u64` length prefix (8 bytes) + data
    ///
    /// So the payload length prefix occupies bytes `[36, 44)`.
    const PAYLOAD_LEN_PREFIX_OFFSET: usize = 4 + 32;

    /// Allocation-bomb defense: an envelope whose inner `payload` length prefix
    /// claims a huge number of bytes while actually supplying few must fail
    /// closed with `MalformedData` rather than attempting to reserve the declared
    /// capacity.
    ///
    /// This pins the decoder's fail-closed behavior under wincode: wincode
    /// refuses to allocate for a declared length it cannot satisfy from the
    /// remaining input (it does not pre-reserve the full declared capacity). The
    /// forged envelope stays well under `MAX_PIR_SIZE`, so the outer size cap does
    /// not catch it — the wincode decode itself must. A regression (e.g. a
    /// serializer config or crate change that pre-reserves capacity) would turn
    /// this into an OOM / abort, which this test exists to catch.
    #[allow(clippy::unwrap_used, clippy::indexing_slicing)]
    #[test]
    fn test_decode_rejects_inflated_payload_length_prefix() {
        let original = minimal_planet("alloc-bomb");
        let envelope = encode_planet_definition(&original).unwrap();
        let mut bytes = wincode::serialize(&envelope).unwrap();

        // Sanity: the stream is long enough to hold the prefix we are about to
        // overwrite, and the layout assumption holds.
        assert!(bytes.len() > PAYLOAD_LEN_PREFIX_OFFSET + 8);

        // Overwrite the 8-byte little-endian payload length with a value far
        // larger than any input could legitimately carry (and larger than
        // MAX_PIR_SIZE), while leaving the actual trailing bytes short. A decoder
        // that trusts the prefix would try to allocate ~16 EiB.
        let huge: u64 = u64::MAX;
        bytes[PAYLOAD_LEN_PREFIX_OFFSET..PAYLOAD_LEN_PREFIX_OFFSET + 8]
            .copy_from_slice(&huge.to_le_bytes());

        // Must fail cleanly — never panic, never over-allocate, never hang.
        let result = decode_planet_definition(&bytes);
        assert!(matches!(result, Err(Error::MalformedData { .. })));
    }

    /// A more modest but still hostile inflated prefix (1 GiB claimed, a handful
    /// of bytes supplied) must also fail closed rather than reserving 1 GiB.
    #[allow(clippy::unwrap_used, clippy::indexing_slicing)]
    #[test]
    fn test_decode_rejects_moderately_inflated_payload_prefix() {
        let original = minimal_planet("alloc-bomb-2");
        let envelope = encode_planet_definition(&original).unwrap();
        let mut bytes = wincode::serialize(&envelope).unwrap();
        assert!(bytes.len() > PAYLOAD_LEN_PREFIX_OFFSET + 8);

        let one_gib: u64 = 1024 * 1024 * 1024;
        bytes[PAYLOAD_LEN_PREFIX_OFFSET..PAYLOAD_LEN_PREFIX_OFFSET + 8]
            .copy_from_slice(&one_gib.to_le_bytes());
        // Truncate so the declared length cannot possibly be satisfied.
        bytes.truncate(PAYLOAD_LEN_PREFIX_OFFSET + 8 + 4);

        let result = decode_planet_definition(&bytes);
        assert!(matches!(result, Err(Error::MalformedData { .. })));
    }
}
