//! PIR envelope structure.

use crate::hash::CanonicalHash;
use crate::version::PirVersion;
use serde::{Deserialize, Serialize};

/// The outer envelope for canonical PIR data.
///
/// Contains version metadata and the encoded definition payload.
/// Separates format versioning from domain content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PirEnvelope {
    /// PIR format version.
    pub version: PirVersion,

    /// Canonical hash of the payload bytes.
    ///
    /// This hash is computed from the serialized `payload` field only,
    /// allowing version-independent content addressing.
    pub content_hash: CanonicalHash,

    /// The encoded planet definition payload.
    ///
    /// This is the canonical serialized form of `PlanetDefinition`.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl PirEnvelope {
    /// Creates a new PIR envelope with the given version and payload.
    ///
    /// Computes the content hash automatically from the payload.
    ///
    /// # Arguments
    /// * `version` - The PIR format version.
    /// * `payload` - The canonical encoded planet definition bytes.
    #[must_use]
    pub fn new(version: PirVersion, payload: Vec<u8>) -> Self {
        let content_hash = CanonicalHash::compute(&payload);
        Self {
            version,
            content_hash,
            payload,
        }
    }

    /// Verifies that the stored content hash matches the payload.
    ///
    /// # Returns
    /// `true` if the hash is valid, `false` otherwise.
    #[must_use]
    pub fn verify_hash(&self) -> bool {
        let computed = CanonicalHash::compute(&self.payload);
        computed == self.content_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_envelope() {
        let version = PirVersion::new(1, 0);
        let payload = b"test payload".to_vec();
        let envelope = PirEnvelope::new(version, payload.clone());

        assert_eq!(envelope.version, version);
        assert_eq!(envelope.payload, payload);
    }

    #[test]
    fn test_verify_hash_valid() {
        let envelope = PirEnvelope::new(PirVersion::new(1, 0), b"test".to_vec());
        assert!(envelope.verify_hash());
    }

    #[test]
    fn test_verify_hash_invalid() {
        let mut envelope = PirEnvelope::new(PirVersion::new(1, 0), b"original".to_vec());
        // Corrupt the payload
        envelope.payload = b"corrupted".to_vec();
        assert!(!envelope.verify_hash());
    }

    #[test]
    fn test_same_payload_same_hash() {
        let payload = b"consistent data".to_vec();
        let env1 = PirEnvelope::new(PirVersion::new(1, 0), payload.clone());
        let env2 = PirEnvelope::new(PirVersion::new(1, 0), payload);
        assert_eq!(env1.content_hash, env2.content_hash);
    }

    #[test]
    fn test_different_payload_different_hash() {
        let env1 = PirEnvelope::new(PirVersion::new(1, 0), b"data1".to_vec());
        let env2 = PirEnvelope::new(PirVersion::new(1, 0), b"data2".to_vec());
        assert_ne!(env1.content_hash, env2.content_hash);
    }
}
