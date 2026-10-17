//! Canonical PIR emission.
//!
//! Emission delegates entirely to [`hpdl_pir`], which is the single source of
//! canonical bytes and the domain-separated content hash. This crate does not
//! re-implement encoding or hashing — it only drives the encoder and verifies
//! the result's integrity before handing it back.

use hpdl_model::PlanetDefinition;
use hpdl_pir::{encode_planet_definition, PirEnvelope};

use crate::Error;

/// Encode a validated definition into a canonical [`PirEnvelope`].
///
/// The resulting envelope's content hash is verified before returning; a
/// verification failure indicates an internal invariant breach and is surfaced
/// as [`Error::Emission`].
///
/// # Errors
///
/// Returns [`Error::Emission`] if `hpdl-pir` fails to encode the definition or
/// if the computed content hash does not match the payload.
pub(crate) fn emit(definition: &PlanetDefinition) -> Result<PirEnvelope, Error> {
    let envelope =
        encode_planet_definition(definition).map_err(|e| Error::Emission(e.to_string()))?;

    if !envelope.verify_hash() {
        return Err(Error::Emission(
            "content hash verification failed for freshly encoded envelope".to_string(),
        ));
    }

    Ok(envelope)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, TraitSet};

    fn minimal_definition(id: &str) -> PlanetDefinition {
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

    #[test]
    fn emission_verifies() {
        let envelope = emit(&minimal_definition("earth")).unwrap();
        assert!(envelope.verify_hash());
        assert!(!envelope.payload.is_empty());
    }

    #[test]
    fn emission_is_deterministic() {
        let a = emit(&minimal_definition("earth")).unwrap();
        let b = emit(&minimal_definition("earth")).unwrap();
        assert_eq!(a.content_hash, b.content_hash);
        assert_eq!(a.payload, b.payload);
    }

    #[test]
    fn distinct_definitions_differ() {
        let a = emit(&minimal_definition("earth")).unwrap();
        let b = emit(&minimal_definition("mars")).unwrap();
        assert_ne!(a.content_hash, b.content_hash);
    }
}
