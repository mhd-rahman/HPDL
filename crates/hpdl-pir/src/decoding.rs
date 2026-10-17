//! Canonical decoding for planet definitions.

use crate::encoding::MAX_PIR_SIZE;
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
/// let envelope_bytes = bincode::serialize(&envelope).expect("envelope serialization failed");
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

    // Deserialize the envelope
    let envelope: PirEnvelope =
        bincode::deserialize(encoded).map_err(|e| Error::MalformedData {
            reason: format!("failed to deserialize envelope: {}", e),
        })?;

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

    // Deserialize the planet definition from the payload
    let planet: PlanetDefinition =
        bincode::deserialize(&envelope.payload).map_err(|e| Error::MalformedData {
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
        let envelope_bytes = bincode::serialize(&envelope).unwrap();
        let decoded = decode_planet_definition(&envelope_bytes).unwrap();

        assert_eq!(decoded.body().id(), original.body().id());
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_roundtrip() {
        let original = minimal_planet("roundtrip-test");

        let envelope = encode_planet_definition(&original).unwrap();
        let envelope_bytes = bincode::serialize(&envelope).unwrap();
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
        let garbage = b"not valid bincode data";
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

        let envelope_bytes = bincode::serialize(&envelope).unwrap();
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

        let envelope_bytes = bincode::serialize(&envelope).unwrap();
        let result = decode_planet_definition(&envelope_bytes);
        assert!(matches!(result, Err(Error::IncompatibleVersion { .. })));
    }
}
