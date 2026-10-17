//! Frontend diagnostic types for RON parsing errors.

use crate::source::{SourceDocument, SourceSpan};
use std::fmt;

/// A diagnostic message from the RON frontend with source location context.
#[derive(Debug, Clone)]
pub struct FrontendDiagnostic {
    /// The diagnostic severity level.
    pub severity: Severity,

    /// The diagnostic message.
    pub message: String,

    /// Source span where the diagnostic applies.
    pub span: Option<SourceSpan>,

    /// Additional notes or context.
    pub notes: Vec<String>,

    /// Diagnostic code for stable reference.
    pub code: Option<String>,
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Error that prevents successful parsing/validation.
    Error,

    /// Warning about potentially problematic input.
    Warning,

    /// Informational message.
    Info,
}

impl FrontendDiagnostic {
    /// Create a new error diagnostic.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            notes: Vec::new(),
            code: None,
        }
    }

    /// Create a new warning diagnostic.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            notes: Vec::new(),
            code: None,
        }
    }

    /// Create a new info diagnostic.
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
            span: None,
            notes: Vec::new(),
            code: None,
        }
    }

    /// Add a source span to this diagnostic.
    #[must_use]
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    /// Add a note to this diagnostic.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add a diagnostic code.
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Format this diagnostic with source context from the document.
    pub fn format_with_source(&self, source: &SourceDocument) -> String {
        let mut output = String::new();

        // Severity and code
        let severity_str = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };

        if let Some(code) = &self.code {
            output.push_str(&format!("{severity_str}[{code}]"));
        } else {
            output.push_str(severity_str);
        }

        // Location
        if let Some(span) = &self.span {
            let (line, col) = source.line_col(span.start);
            output.push_str(&format!(": {}:{}:{}", source.path().display(), line, col));
        }

        // Message
        output.push_str(&format!("\n  {}\n", self.message));

        // Source excerpt
        if let Some(span) = &self.span {
            if let Some(excerpt) = source.get_excerpt(*span) {
                output.push_str(&format!("\n{excerpt}\n"));
            }
        }

        // Notes
        for note in &self.notes {
            output.push_str(&format!("  note: {note}\n"));
        }

        output
    }
}

impl fmt::Display for FrontendDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity_str = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };

        if let Some(code) = &self.code {
            write!(f, "{severity_str}[{code}]: {}", self.message)?;
        } else {
            write!(f, "{severity_str}: {}", self.message)?;
        }

        for note in &self.notes {
            write!(f, "\n  note: {note}")?;
        }

        Ok(())
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_error_diagnostic() {
        let diag = FrontendDiagnostic::error("Test error message");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "Test error message");
        assert!(diag.span.is_none());
        assert!(diag.notes.is_empty());
    }

    #[test]
    fn test_diagnostic_with_code() {
        let diag = FrontendDiagnostic::error("Test error").with_code("E001");
        assert_eq!(diag.code, Some("E001".to_string()));
        assert_eq!(diag.to_string(), "error[E001]: Test error");
    }

    #[test]
    fn test_diagnostic_with_note() {
        let diag = FrontendDiagnostic::warning("Test warning").with_note("Consider fixing this");
        assert_eq!(diag.notes.len(), 1);
        assert_eq!(diag.notes[0], "Consider fixing this");
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = FrontendDiagnostic::error("Unexpected token")
            .with_code("E002")
            .with_note("Expected a closing brace");

        let output = diag.to_string();
        assert!(output.contains("error[E002]"));
        assert!(output.contains("Unexpected token"));
        assert!(output.contains("note: Expected a closing brace"));
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Info.to_string(), "info");
    }
}
