//! Resource limits for parsing untrusted `.hpdl` source.
//!
//! The parser treats source text as untrusted input. These limits bound the
//! work the lexer/parser will attempt before failing closed with a typed error.

use crate::Error;

/// Configurable resource limits applied before/while parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    /// Maximum source size in bytes (default: 10 MiB).
    pub max_source_bytes: usize,
    /// Maximum identifier length in characters (default: 256).
    pub max_identifier_len: usize,
    /// Maximum value-nesting depth the parser will descend before failing
    /// closed (default: 128).
    ///
    /// The parser is recursive descent, so unbounded nesting (`[[[[…]]]]`,
    /// `Variant(Variant(…))`, nested objects) would recurse until the native
    /// stack overflows and the process aborts. This bound turns that into a
    /// typed [`Error::Security`] long before the stack is exhausted.
    pub max_nesting_depth: usize,
}

impl ParseLimits {
    /// Default maximum source size: 10 MiB.
    pub const DEFAULT_MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;
    /// Default maximum identifier length: 256 characters.
    pub const DEFAULT_MAX_IDENTIFIER_LEN: usize = 256;
    /// Default maximum value-nesting depth: 128 levels.
    ///
    /// Comfortably deeper than any legitimate planet definition while far below
    /// the point at which recursive-descent parsing risks a stack overflow on
    /// supported platforms.
    pub const DEFAULT_MAX_NESTING_DEPTH: usize = 128;

    /// Verify a source string is within the size limit.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Security`] if the source exceeds `max_source_bytes`.
    pub fn check_source_size(&self, source: &str) -> Result<(), Error> {
        if source.len() > self.max_source_bytes {
            return Err(Error::security(format!(
                "source size {} bytes exceeds maximum {}",
                source.len(),
                self.max_source_bytes
            )));
        }
        Ok(())
    }

    /// Verify an identifier is within the length limit.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Security`] if the identifier exceeds `max_identifier_len`.
    pub fn check_identifier(&self, ident: &str) -> Result<(), Error> {
        if ident.chars().count() > self.max_identifier_len {
            return Err(Error::security(format!(
                "identifier length {} exceeds maximum {}",
                ident.chars().count(),
                self.max_identifier_len
            )));
        }
        Ok(())
    }

    /// Verify a nesting depth is within the limit.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Security`] if `depth` exceeds `max_nesting_depth`.
    pub fn check_depth(&self, depth: usize) -> Result<(), Error> {
        if depth > self.max_nesting_depth {
            return Err(Error::security(format!(
                "nesting depth {} exceeds maximum {}",
                depth, self.max_nesting_depth
            )));
        }
        Ok(())
    }
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: Self::DEFAULT_MAX_SOURCE_BYTES,
            max_identifier_len: Self::DEFAULT_MAX_IDENTIFIER_LEN,
            max_nesting_depth: Self::DEFAULT_MAX_NESTING_DEPTH,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn default_limits() {
        let l = ParseLimits::default();
        assert_eq!(l.max_source_bytes, ParseLimits::DEFAULT_MAX_SOURCE_BYTES);
        assert_eq!(
            l.max_identifier_len,
            ParseLimits::DEFAULT_MAX_IDENTIFIER_LEN
        );
        assert_eq!(l.max_nesting_depth, ParseLimits::DEFAULT_MAX_NESTING_DEPTH);
    }

    #[test]
    fn source_within_limit_ok() {
        let l = ParseLimits::default();
        assert!(l.check_source_size("language hpdl \"1.0\";").is_ok());
    }

    #[test]
    fn oversized_source_rejected() {
        let l = ParseLimits {
            max_source_bytes: 8,
            ..ParseLimits::default()
        };
        let err = l.check_source_size("this is longer than eight bytes");
        assert!(matches!(err, Err(Error::Security(_))));
    }

    #[test]
    fn overlong_identifier_rejected() {
        let l = ParseLimits {
            max_identifier_len: 4,
            ..ParseLimits::default()
        };
        assert!(l.check_identifier("ok").is_ok());
        assert!(matches!(
            l.check_identifier("toolong"),
            Err(Error::Security(_))
        ));
    }

    #[test]
    fn depth_within_limit_ok() {
        let l = ParseLimits {
            max_nesting_depth: 4,
            ..ParseLimits::default()
        };
        assert!(l.check_depth(0).is_ok());
        assert!(l.check_depth(4).is_ok());
    }

    #[test]
    fn excessive_depth_rejected() {
        let l = ParseLimits {
            max_nesting_depth: 4,
            ..ParseLimits::default()
        };
        assert!(matches!(l.check_depth(5), Err(Error::Security(_))));
    }
}
