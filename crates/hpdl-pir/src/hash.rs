//! Canonical content-addressed hashing for PIR data.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// A cryptographic hash of canonical PIR bytes.
///
/// Uses SHA-256 with a domain separator to prevent cross-protocol attacks.
/// The domain separator includes protocol ID and format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanonicalHash([u8; 32]);

impl CanonicalHash {
    /// Domain separator for HPDL PIR v2.x hashes.
    ///
    /// This prevents hash collisions with other protocols or format versions.
    /// The version suffix is bound to the PIR major version: v2 was introduced
    /// with feature 07 (orbital fields on `CelestialBody`). A v1 and a v2
    /// encoding of "the same" planet therefore hash to different, deliberately
    /// non-colliding digests. See
    /// `docs/rfcs/ADR-0003-pir-v2-orbital-fields.md`.
    const DOMAIN_SEPARATOR: &'static [u8] = b"HPDL-PIR-v2";

    /// Computes the canonical hash of PIR bytes.
    ///
    /// # Arguments
    /// * `pir_bytes` - The canonical PIR-encoded bytes to hash.
    ///
    /// # Returns
    /// A 32-byte SHA-256 digest with domain separation.
    #[must_use]
    pub fn compute(pir_bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(Self::DOMAIN_SEPARATOR);
        hasher.update(pir_bytes);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Self(hash)
    }

    /// Returns the hash bytes as a slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the hash as a hexadecimal string.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Creates a hash from raw bytes.
    ///
    /// This is primarily for testing and deserialization.
    /// Prefer `compute` for normal use.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for CanonicalHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input_hash() {
        let hash = CanonicalHash::compute(&[]);
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_deterministic_hash() {
        let data = b"test data";
        let hash1 = CanonicalHash::compute(data);
        let hash2 = CanonicalHash::compute(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_data_different_hash() {
        let hash1 = CanonicalHash::compute(b"data1");
        let hash2 = CanonicalHash::compute(b"data2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_domain_separation() {
        // Hash of just the data without domain separator
        let direct_hash = {
            let mut hasher = Sha256::new();
            hasher.update(b"test");
            hasher.finalize()
        };

        // Hash with domain separator
        let canonical_hash = CanonicalHash::compute(b"test");

        // They should be different due to domain separation
        assert_ne!(direct_hash.as_slice(), canonical_hash.as_bytes());
    }

    #[test]
    fn test_hex_encoding() {
        let hash = CanonicalHash::from_bytes([
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        assert!(hash.to_hex().starts_with("123456789abcdef0"));
    }

    #[test]
    fn test_display() {
        let hash = CanonicalHash::from_bytes([0xff; 32]);
        let display = format!("{}", hash);
        assert_eq!(display.len(), 64); // 32 bytes = 64 hex chars
        assert!(display.chars().all(|c| c == 'f'));
    }
}
