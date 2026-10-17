//! PIR encoding and decoding errors.

use crate::version::PirVersion;
use std::fmt;

/// Errors that can occur during PIR encoding or decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Incompatible PIR format version.
    IncompatibleVersion {
        /// The version found in the encoded data.
        found: PirVersion,
        /// The version supported by this decoder.
        expected: PirVersion,
    },

    /// Input exceeds maximum allowed size.
    InputTooLarge {
        /// The size of the input in bytes.
        size: usize,
        /// The maximum allowed size in bytes.
        max: usize,
    },

    /// Decoding failed due to malformed data.
    MalformedData {
        /// Description of what went wrong.
        reason: String,
    },

    /// Encoding failed.
    EncodingFailed {
        /// Description of what went wrong.
        reason: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IncompatibleVersion { found, expected } => {
                write!(
                    f,
                    "Incompatible PIR version: found {}, expected {}",
                    found, expected
                )
            }
            Error::InputTooLarge { size, max } => {
                write!(
                    f,
                    "Input too large: {} bytes exceeds maximum of {} bytes",
                    size, max
                )
            }
            Error::MalformedData { reason } => {
                write!(f, "Malformed PIR data: {}", reason)
            }
            Error::EncodingFailed { reason } => {
                write!(f, "PIR encoding failed: {}", reason)
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_incompatible_version_display() {
        let err = Error::IncompatibleVersion {
            found: PirVersion::new(2, 0),
            expected: PirVersion::new(1, 0),
        };
        assert_eq!(
            err.to_string(),
            "Incompatible PIR version: found 2.0, expected 1.0"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_input_too_large_display() {
        let err = Error::InputTooLarge {
            size: 1024,
            max: 512,
        };
        assert_eq!(
            err.to_string(),
            "Input too large: 1024 bytes exceeds maximum of 512 bytes"
        );
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_malformed_data_display() {
        let err = Error::MalformedData {
            reason: "unexpected EOF".to_string(),
        };
        assert_eq!(err.to_string(), "Malformed PIR data: unexpected EOF");
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_encoding_failed_display() {
        let err = Error::EncodingFailed {
            reason: "invalid field".to_string(),
        };
        assert_eq!(err.to_string(), "PIR encoding failed: invalid field");
    }
}
