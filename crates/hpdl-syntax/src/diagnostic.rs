//! Parse diagnostics for multi-error recovery.
//!
//! The recovering parser ([`crate::parse_file_recover`]) collects every
//! independent syntax error it can find in a single pass rather than stopping at
//! the first. Each error is a [`ParseError`] carrying the offending span so a
//! caller can render a source excerpt.

use crate::SourceSpan;

/// A single parse error with its source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable message describing the error.
    pub message: String,
    /// The source span the error applies to.
    pub span: SourceSpan,
}

impl ParseError {
    /// Create a new parse error.
    pub fn new(message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "parse error at {}..{}: {}",
            self.span.start, self.span.end, self.message
        )
    }
}

/// The result of a recovering parse: either a complete AST, or one or more
/// collected [`ParseError`]s.
///
/// Even when errors are returned, the parser has attempted to recover at
/// field/synchronization boundaries so that multiple independent problems are
/// reported together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseErrors {
    /// All collected errors, in source order.
    pub errors: Vec<ParseError>,
}

impl ParseErrors {
    /// Create from a vector of errors.
    pub fn new(errors: Vec<ParseError>) -> Self {
        Self { errors }
    }

    /// Number of collected errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Whether there are no errors.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl std::fmt::Display for ParseErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} parse error(s):", self.errors.len())?;
        for e in &self.errors {
            writeln!(f, "  {e}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display() {
        let e = ParseError::new("boom", SourceSpan::new(3, 7));
        assert!(e.to_string().contains("3..7"));
        assert!(e.to_string().contains("boom"));
    }

    #[test]
    fn parse_errors_len_and_empty() {
        assert!(ParseErrors::new(vec![]).is_empty());
        let errs = ParseErrors::new(vec![ParseError::new("x", SourceSpan::new(0, 1))]);
        assert_eq!(errs.len(), 1);
        assert!(!errs.is_empty());
    }
}
