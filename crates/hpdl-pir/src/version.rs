//! PIR format version tracking.

use serde::{Deserialize, Serialize};

/// PIR format version identifier.
///
/// Each version documents breaking changes to the binary format.
/// Decoders MUST reject unknown versions to prevent silent misinterpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PirVersion {
    /// Major version (increments on breaking format changes).
    pub major: u16,
    /// Minor version (increments on backward-compatible additions).
    pub minor: u16,
}

impl PirVersion {
    /// Current PIR format version.
    ///
    /// Bumped to 2.0 for feature 07 (celestial & orbital systems): the
    /// `CelestialBody` struct gained optional orbital declaration fields
    /// (`identity`, `parent`, `orbit`, `rotation`). Because the canonical
    /// encoding is bincode — which is not self-describing and writes a presence
    /// byte for every `Option`, including `None` — adding these fields is a
    /// breaking change to the byte layout. Old (v1) bytes cannot be decoded by
    /// the v2 struct and vice versa, so the major version increments. See
    /// `docs/rfcs/ADR-0003-pir-v2-orbital-fields.md`.
    pub const CURRENT: PirVersion = PirVersion { major: 2, minor: 0 };

    /// Creates a new PIR version.
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Checks if this decoder can read the given format version.
    ///
    /// Compatible if major matches and minor is less than or equal to current.
    #[must_use]
    pub const fn is_compatible(&self, other: &PirVersion) -> bool {
        self.major == other.major && other.minor <= self.minor
    }
}

impl std::fmt::Display for PirVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_version() {
        let current = PirVersion::CURRENT;
        assert_eq!(current.major, 2);
        assert_eq!(current.minor, 0);
    }

    #[test]
    fn test_v1_bytes_are_incompatible_with_v2_decoder() {
        // A v2 decoder must reject v1-encoded data: the byte layout changed.
        let decoder = PirVersion::CURRENT; // 2.0
        let v1_encoded = PirVersion::new(1, 0);
        assert!(!decoder.is_compatible(&v1_encoded));
    }

    #[test]
    fn test_compatibility_same_version() {
        let v1 = PirVersion::new(1, 0);
        let v2 = PirVersion::new(1, 0);
        assert!(v1.is_compatible(&v2));
    }

    #[test]
    fn test_compatibility_newer_minor() {
        let decoder = PirVersion::new(1, 2);
        let encoded = PirVersion::new(1, 0);
        assert!(decoder.is_compatible(&encoded));
    }

    #[test]
    fn test_incompatibility_older_minor() {
        let decoder = PirVersion::new(1, 0);
        let encoded = PirVersion::new(1, 2);
        assert!(!decoder.is_compatible(&encoded));
    }

    #[test]
    fn test_incompatibility_different_major() {
        let decoder = PirVersion::new(1, 0);
        let encoded = PirVersion::new(2, 0);
        assert!(!decoder.is_compatible(&encoded));
    }

    #[test]
    fn test_display() {
        let version = PirVersion::new(1, 0);
        assert_eq!(version.to_string(), "1.0");
    }
}
