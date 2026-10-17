//! Error types for the RON frontend.

use std::io;
use thiserror::Error;

/// Errors produced by the RON frontend.
#[derive(Debug, Error)]
pub enum Error {
    /// RON parsing or deserialization failed.
    #[error("RON parse error: {0}")]
    Parse(String),

    /// RON parsing failed with span information.
    #[error("RON parse error at {position}: {error}")]
    ParseWithSpan {
        /// The underlying error
        error: String,
        /// Position in the source
        position: String,
    },

    /// Semantic validation failed (from hpdl-sema).
    #[error("Semantic validation error: {0}")]
    Sema(#[from] hpdl_sema::Error),

    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Security limit exceeded.
    #[error("Security limit exceeded: {0}")]
    Security(String),

    /// Invalid UTF-8 encoding.
    #[error("Invalid UTF-8 encoding: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    /// Path traversal attempt detected.
    #[error("Path traversal attempt: {0}")]
    PathTraversal(String),

    /// File size exceeds maximum allowed.
    #[error("File size {actual} exceeds maximum {maximum}")]
    FileSizeExceeded {
        /// Actual file size in bytes.
        actual: u64,
        /// Maximum allowed size in bytes.
        maximum: u64,
    },

    /// Nesting depth exceeds maximum allowed.
    #[error("Nesting depth {actual} exceeds maximum {maximum}")]
    NestingDepthExceeded {
        /// Actual nesting depth.
        actual: usize,
        /// Maximum allowed depth.
        maximum: usize,
    },

    /// Collection length exceeds maximum allowed.
    #[error("Collection length {actual} exceeds maximum {maximum}")]
    CollectionLengthExceeded {
        /// Actual collection length.
        actual: usize,
        /// Maximum allowed length.
        maximum: usize,
    },
}

impl From<ron::error::SpannedError> for Error {
    fn from(err: ron::error::SpannedError) -> Self {
        Self::ParseWithSpan {
            error: err.code.to_string(),
            position: format!("{}:{}", err.position.line, err.position.col),
        }
    }
}

impl Error {
    /// Create a security error with a custom message.
    pub fn security(msg: impl Into<String>) -> Self {
        Self::Security(msg.into())
    }

    /// Create a path traversal error.
    pub fn path_traversal(path: impl Into<String>) -> Self {
        Self::PathTraversal(path.into())
    }

    /// Create a file size exceeded error.
    pub fn file_size_exceeded(actual: u64, maximum: u64) -> Self {
        Self::FileSizeExceeded { actual, maximum }
    }

    /// Create a nesting depth exceeded error.
    pub fn nesting_depth_exceeded(actual: usize, maximum: usize) -> Self {
        Self::NestingDepthExceeded { actual, maximum }
    }

    /// Create a collection length exceeded error.
    pub fn collection_length_exceeded(actual: usize, maximum: usize) -> Self {
        Self::CollectionLengthExceeded { actual, maximum }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_security_error_construction() {
        let err = Error::security("Test security violation");
        assert!(matches!(err, Error::Security(_)));
        assert_eq!(
            err.to_string(),
            "Security limit exceeded: Test security violation"
        );
    }

    #[test]
    fn test_path_traversal_error() {
        let err = Error::path_traversal("../../etc/passwd");
        assert!(matches!(err, Error::PathTraversal(_)));
        assert_eq!(err.to_string(), "Path traversal attempt: ../../etc/passwd");
    }

    #[test]
    fn test_file_size_exceeded_error() {
        let err = Error::file_size_exceeded(20_000_000, 10_000_000);
        assert!(matches!(err, Error::FileSizeExceeded { .. }));
        assert_eq!(
            err.to_string(),
            "File size 20000000 exceeds maximum 10000000"
        );
    }

    #[test]
    fn test_nesting_depth_exceeded_error() {
        let err = Error::nesting_depth_exceeded(100, 64);
        assert!(matches!(err, Error::NestingDepthExceeded { .. }));
        assert_eq!(err.to_string(), "Nesting depth 100 exceeds maximum 64");
    }

    #[test]
    fn test_collection_length_exceeded_error() {
        let err = Error::collection_length_exceeded(15_000, 10_000);
        assert!(matches!(err, Error::CollectionLengthExceeded { .. }));
        assert_eq!(
            err.to_string(),
            "Collection length 15000 exceeds maximum 10000"
        );
    }

    #[test]
    fn test_ron_error_conversion() {
        // Test that SpannedError conversion works
        let ron_content = "invalid {{{ RON";
        let result: Result<hpdl_model::PlanetDefinition, ron::error::SpannedError> =
            ron::from_str(ron_content);

        if let Err(spanned_err) = result {
            let our_err: Error = spanned_err.into();
            assert!(matches!(our_err, Error::ParseWithSpan { .. }));
        }
    }
}
