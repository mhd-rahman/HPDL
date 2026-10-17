//! Security configuration and resource limits for RON parsing.

use crate::Error;

/// Security configuration for protecting against malicious RON input.
///
/// These limits prevent resource exhaustion attacks from malformed or
/// malicious RON files.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum file size in bytes (default: 10 MB).
    max_file_size: u64,

    /// Maximum nesting depth for structures (default: 64).
    max_nesting_depth: usize,

    /// Maximum length for collections (arrays, maps) (default: 10,000).
    max_collection_length: usize,
}

impl SecurityConfig {
    /// Default maximum file size: 10 MB.
    pub const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

    /// Default maximum nesting depth: 64 levels.
    pub const DEFAULT_MAX_NESTING_DEPTH: usize = 64;

    /// Default maximum collection length: 10,000 items.
    pub const DEFAULT_MAX_COLLECTION_LENGTH: usize = 10_000;

    /// Create a new security configuration with default limits.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for custom security configuration.
    pub fn builder() -> SecurityConfigBuilder {
        SecurityConfigBuilder::new()
    }

    /// Get the maximum file size limit.
    pub fn max_file_size(&self) -> u64 {
        self.max_file_size
    }

    /// Get the maximum nesting depth limit.
    pub fn max_nesting_depth(&self) -> usize {
        self.max_nesting_depth
    }

    /// Get the maximum collection length limit.
    pub fn max_collection_length(&self) -> usize {
        self.max_collection_length
    }

    /// Validate that a file size is within limits.
    ///
    /// # Errors
    ///
    /// Returns `Error::FileSizeExceeded` if the size exceeds the limit.
    pub fn validate_file_size(&self, size: u64) -> Result<(), Error> {
        if size > self.max_file_size {
            Err(Error::file_size_exceeded(size, self.max_file_size))
        } else {
            Ok(())
        }
    }

    /// Validate that a nesting depth is within limits.
    ///
    /// # Errors
    ///
    /// Returns `Error::NestingDepthExceeded` if the depth exceeds the limit.
    pub fn validate_nesting_depth(&self, depth: usize) -> Result<(), Error> {
        if depth > self.max_nesting_depth {
            Err(Error::nesting_depth_exceeded(depth, self.max_nesting_depth))
        } else {
            Ok(())
        }
    }

    /// Validate that a collection length is within limits.
    ///
    /// # Errors
    ///
    /// Returns `Error::CollectionLengthExceeded` if the length exceeds the limit.
    pub fn validate_collection_length(&self, length: usize) -> Result<(), Error> {
        if length > self.max_collection_length {
            Err(Error::collection_length_exceeded(
                length,
                self.max_collection_length,
            ))
        } else {
            Ok(())
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_file_size: Self::DEFAULT_MAX_FILE_SIZE,
            max_nesting_depth: Self::DEFAULT_MAX_NESTING_DEPTH,
            max_collection_length: Self::DEFAULT_MAX_COLLECTION_LENGTH,
        }
    }
}

/// Builder for creating custom security configurations.
#[derive(Debug)]
pub struct SecurityConfigBuilder {
    max_file_size: u64,
    max_nesting_depth: usize,
    max_collection_length: usize,
}

impl SecurityConfigBuilder {
    /// Create a new builder with default values.
    fn new() -> Self {
        Self {
            max_file_size: SecurityConfig::DEFAULT_MAX_FILE_SIZE,
            max_nesting_depth: SecurityConfig::DEFAULT_MAX_NESTING_DEPTH,
            max_collection_length: SecurityConfig::DEFAULT_MAX_COLLECTION_LENGTH,
        }
    }

    /// Set the maximum file size in bytes.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    #[must_use]
    pub fn max_file_size(mut self, size: u64) -> Self {
        assert!(size > 0, "max_file_size must be greater than 0");
        self.max_file_size = size;
        self
    }

    /// Set the maximum nesting depth.
    ///
    /// # Panics
    ///
    /// Panics if `depth` is 0.
    #[must_use]
    pub fn max_nesting_depth(mut self, depth: usize) -> Self {
        assert!(depth > 0, "max_nesting_depth must be greater than 0");
        self.max_nesting_depth = depth;
        self
    }

    /// Set the maximum collection length.
    ///
    /// # Panics
    ///
    /// Panics if `length` is 0.
    #[must_use]
    pub fn max_collection_length(mut self, length: usize) -> Self {
        assert!(length > 0, "max_collection_length must be greater than 0");
        self.max_collection_length = length;
        self
    }

    /// Build the security configuration.
    pub fn build(self) -> SecurityConfig {
        SecurityConfig {
            max_file_size: self.max_file_size,
            max_nesting_depth: self.max_nesting_depth,
            max_collection_length: self.max_collection_length,
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    unused_must_use
)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SecurityConfig::new();
        assert_eq!(
            config.max_file_size(),
            SecurityConfig::DEFAULT_MAX_FILE_SIZE
        );
        assert_eq!(
            config.max_nesting_depth(),
            SecurityConfig::DEFAULT_MAX_NESTING_DEPTH
        );
        assert_eq!(
            config.max_collection_length(),
            SecurityConfig::DEFAULT_MAX_COLLECTION_LENGTH
        );
    }

    #[test]
    fn test_validate_file_size_within_limit() {
        let config = SecurityConfig::new();
        assert!(config.validate_file_size(1024).is_ok());
        assert!(config.validate_file_size(5_000_000).is_ok());
    }

    #[test]
    fn test_validate_file_size_exceeds_limit() {
        let config = SecurityConfig::new();
        let result = config.validate_file_size(20_000_000);
        assert!(matches!(result, Err(Error::FileSizeExceeded { .. })));
    }

    #[test]
    fn test_validate_nesting_depth_within_limit() {
        let config = SecurityConfig::new();
        assert!(config.validate_nesting_depth(10).is_ok());
        assert!(config.validate_nesting_depth(64).is_ok());
    }

    #[test]
    fn test_validate_nesting_depth_exceeds_limit() {
        let config = SecurityConfig::new();
        let result = config.validate_nesting_depth(100);
        assert!(matches!(result, Err(Error::NestingDepthExceeded { .. })));
    }

    #[test]
    fn test_validate_collection_length_within_limit() {
        let config = SecurityConfig::new();
        assert!(config.validate_collection_length(100).is_ok());
        assert!(config.validate_collection_length(10_000).is_ok());
    }

    #[test]
    fn test_validate_collection_length_exceeds_limit() {
        let config = SecurityConfig::new();
        let result = config.validate_collection_length(15_000);
        assert!(matches!(
            result,
            Err(Error::CollectionLengthExceeded { .. })
        ));
    }

    #[test]
    fn test_builder_custom_values() {
        let config = SecurityConfig::builder()
            .max_file_size(5_000_000)
            .max_nesting_depth(32)
            .max_collection_length(5_000)
            .build();

        assert_eq!(config.max_file_size(), 5_000_000);
        assert_eq!(config.max_nesting_depth(), 32);
        assert_eq!(config.max_collection_length(), 5_000);
    }

    #[test]
    #[should_panic(expected = "max_file_size must be greater than 0")]
    fn test_builder_zero_file_size() {
        let _ = SecurityConfig::builder().max_file_size(0);
    }

    #[test]
    #[should_panic(expected = "max_nesting_depth must be greater than 0")]
    fn test_builder_zero_nesting_depth() {
        let _ = SecurityConfig::builder().max_nesting_depth(0);
    }

    #[test]
    #[should_panic(expected = "max_collection_length must be greater than 0")]
    fn test_builder_zero_collection_length() {
        let _ = SecurityConfig::builder().max_collection_length(0);
    }
}
