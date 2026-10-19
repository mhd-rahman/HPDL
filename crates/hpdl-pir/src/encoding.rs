//! Canonical encoding for planet definitions.

use crate::envelope::PirEnvelope;
use crate::error::Error;
use crate::version::PirVersion;
use hpdl_model::PlanetDefinition;
use wincode::config::Configuration;

/// Maximum allowed size for a single PIR-encoded planet definition (10 MiB).
///
/// This limit prevents unbounded memory allocation from malicious inputs
/// while allowing reasonable planet complexity.
pub const MAX_PIR_SIZE: usize = 10 * 1024 * 1024;

/// Canonical wincode configuration used for all PIR encode/decode.
///
/// This is wincode's **default wire format** (BincodeLen length encoding,
/// little-endian, fixed-width integers, u32 enum tags) — identical bytes to
/// `wincode::serialize`/`deserialize`, so the frozen PIR digests are unchanged.
///
/// The ONLY deviation from the default is the preallocation size limit, raised
/// from wincode's 4 MiB default to [`MAX_PIR_SIZE`] (10 MiB). The preallocation
/// limit is an allocation guard, **not** a wire-format parameter, so changing it
/// does not affect the canonical bytes. Raising it to `MAX_PIR_SIZE` makes that
/// constant the single authoritative size contract on both encode and decode:
/// a definition serializes/deserializes iff it fits in 10 MiB, exactly as before
/// the bincode→wincode migration (ADR 0015). Without this, wincode's 4 MiB
/// default would silently shrink the accepted-input range to 4 MiB.
pub(crate) type CanonicalConfig = Configuration<true, MAX_PIR_SIZE>;

/// The canonical config value, passed to `wincode::config::{serialize, deserialize}`.
pub(crate) const CANONICAL_CONFIG: CanonicalConfig = Configuration::default()
    .with_preallocation_size_limit::<MAX_PIR_SIZE>();

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
    // Encode with the canonical wincode config. This is wincode's default wire
    // format (so bytes are byte-identical to the previous bincode encoding —
    // ADR 0015), with the preallocation limit raised to MAX_PIR_SIZE so that
    // constant remains the single authoritative size cap (see CANONICAL_CONFIG).
    let payload =
        wincode::config::serialize(planet, CANONICAL_CONFIG).map_err(|e| Error::EncodingFailed {
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

    /// The encode-side size guard (`payload.len() > MAX_PIR_SIZE`) must reject an
    /// over-large definition with `EncodingFailed` rather than emitting an
    /// oversized PIR. `CelestialBody` places no count cap on its module list, so a
    /// definition with enough `ModuleRef`s serializes past the 10 MiB cap — a
    /// real, reachable path (the decode side has the mirror `InputTooLarge`
    /// guard, tested separately).
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_encode_rejects_oversized_payload() {
        use hpdl_model::ModuleRef;

        // Each ModuleRef serializes to only a few dozen bytes, so enough of them
        // clear the 10 MiB payload cap. 400k * ~40 B > 10 MiB with margin while
        // keeping the test fast. (The canonical config raises wincode's own
        // preallocation limit to MAX_PIR_SIZE, so the failure here is our size
        // guard, not wincode's default 4 MiB prealloc cap.)
        let modules: Vec<ModuleRef> = (0..400_000)
            .map(|i| ModuleRef::new(format!("hpdl.module.pad-{i:07}"), "1.0.0").unwrap())
            .collect();

        let body = CelestialBody::new(
            BodyId::new("oversized").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            modules,
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        let planet = PlanetDefinition::new("1.0", body).unwrap();

        // Either guard may fire for an over-cap definition, and both yield
        // `EncodingFailed`: our explicit `payload.len() > MAX_PIR_SIZE` check, or
        // wincode's preallocation guard (also set to MAX_PIR_SIZE via
        // CANONICAL_CONFIG), which trips during serialization at the same 10 MiB
        // boundary. The contract under test is "oversized input fails closed with
        // EncodingFailed, never an oversized PIR" — not which guard caught it.
        let result = encode_planet_definition(&planet);
        match result {
            Err(Error::EncodingFailed { reason }) => {
                assert!(
                    reason.contains("exceeds maximum")
                        || reason.contains("preallocation limit"),
                    "expected a size-limit rejection, got: {reason}"
                );
            }
            other => panic!("expected Err(EncodingFailed) for an oversized payload, got {other:?}"),
        }
    }
}
