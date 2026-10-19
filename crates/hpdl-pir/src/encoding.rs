//! Canonical encoding for planet definitions.

use crate::envelope::PirEnvelope;
use crate::error::Error;
use crate::version::PirVersion;
use hpdl_model::PlanetDefinition;

/// Maximum allowed size for a single PIR-encoded planet definition (10 MiB).
///
/// This limit prevents unbounded memory allocation from malicious inputs
/// while allowing reasonable planet complexity.
pub const MAX_PIR_SIZE: usize = 10 * 1024 * 1024;

/// Encodes a planet definition into canonical PIR bytes.
///
/// The encoding is deterministic: the same planet definition always produces
/// identical bytes regardless of platform or compiler version.
///
/// # Arguments
/// * `planet` - The planet definition to encode.
///
/// # Returns
/// A `PirEnvelope` containing the versioned, content-addressed encoding.
///
/// # Errors
/// Returns `Error::EncodingFailed` if serialization fails.
///
/// # Example
/// ```
/// use hpdl_model::{PlanetDefinition, BodyId, CelestialBody, PhysicalProperties, TraitSet};
/// use hpdl_pir::encode_planet_definition;
///
/// let body = CelestialBody::new(
///     BodyId::new("earth").unwrap(),
///     PhysicalProperties::default(),
///     TraitSet::empty(),
///     vec![], vec![], vec![], vec![],
/// ).unwrap();
/// let planet = PlanetDefinition::new("1.0", body).unwrap();
///
/// let envelope = encode_planet_definition(&planet).expect("encoding failed");
/// assert!(envelope.verify_hash());
/// ```
pub fn encode_planet_definition(planet: &PlanetDefinition) -> Result<PirEnvelope, Error> {
    // Use wincode's default configuration for canonical encoding. wincode is
    // bincode-wire-compatible: fixed endianness, fixint lengths, no
    // variable-length encoding tricks, so output is deterministic and
    // byte-identical to the previous bincode encoding (ADR 0015).
    let payload = wincode::serialize(planet).map_err(|e| Error::EncodingFailed {
        reason: format!("wincode serialization failed: {}", e),
    })?;

    // Enforce size limit to prevent unbounded allocations
    if payload.len() > MAX_PIR_SIZE {
        return Err(Error::EncodingFailed {
            reason: format!(
                "encoded size {} exceeds maximum {}",
                payload.len(),
                MAX_PIR_SIZE
            ),
        });
    }

    Ok(PirEnvelope::new(PirVersion::CURRENT, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_encode_minimal_planet() {
        let planet = minimal_planet("test");

        let envelope = encode_planet_definition(&planet).unwrap();
        assert_eq!(envelope.version, PirVersion::CURRENT);
        assert!(envelope.verify_hash());
        assert!(!envelope.payload.is_empty());
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_encode_deterministic() {
        let planet = minimal_planet("determinism-test");

        let env1 = encode_planet_definition(&planet).unwrap();
        let env2 = encode_planet_definition(&planet).unwrap();

        assert_eq!(env1.content_hash, env2.content_hash);
        assert_eq!(env1.payload, env2.payload);
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_encode_different_planets_different_hashes() {
        let planet1 = minimal_planet("planet1");
        let planet2 = minimal_planet("planet2");

        let env1 = encode_planet_definition(&planet1).unwrap();
        let env2 = encode_planet_definition(&planet2).unwrap();

        assert_ne!(env1.content_hash, env2.content_hash);
    }
}
