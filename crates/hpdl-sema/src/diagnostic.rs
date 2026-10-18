//! Diagnostic system for semantic validation errors, warnings, and info messages.

use std::fmt;

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational message.
    Info,
    /// Warning that doesn't prevent compilation.
    Warning,
    /// Error that prevents successful validation.
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Source location information for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceSpan {
    /// Byte offset of the start of the span.
    pub start: usize,
    /// Byte offset of the end of the span (exclusive).
    pub end: usize,
    /// Optional source identifier (file path, module name, etc.).
    pub source: Option<String>,
}

impl SourceSpan {
    /// Create a new source span.
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            source: None,
        }
    }

    /// Create a source span with a source identifier.
    pub fn with_source(start: usize, end: usize, source: String) -> Self {
        Self {
            start,
            end,
            source: Some(source),
        }
    }
}

/// Stable diagnostic codes for semantic validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    // Type validation (E1000 series)
    /// A required field is missing.
    MissingRequiredField,
    /// A field has an invalid type.
    InvalidFieldType,
    /// An enum variant is not recognized.
    InvalidEnumVariant,

    // Constraint validation (E2000 series)
    /// Mass value is negative or zero.
    NegativeMass,
    /// Temperature range is invalid or non-physical.
    InvalidTemperatureRange,
    /// Orbital parameters are inconsistent.
    InconsistentOrbitalParameters,
    /// Constraints contradict each other.
    ContradictoryConstraints,

    // Unit validation (E3000 series)
    /// A quantity is missing its unit.
    MissingUnit,
    /// Units have incompatible dimensions.
    IncompatibleDimensions,
    /// Temperature used where TemperatureDelta expected, or vice versa.
    InvalidTemperatureDelta,
    /// Custom dimension is not registered.
    UnregisteredCustomDimension,
    /// Numeric value is invalid (NaN, infinity).
    InvalidNumericValue,
    /// Numeric operation would overflow.
    NumericOverflow,

    // Status validation (E4000 series)
    /// Status thresholds are not sorted.
    UnsortedThresholds,
    /// Status threshold appears more than once.
    DuplicateThreshold,
    /// Status threshold is outside the scale range.
    ThresholdOutOfRange,
    /// Status scale bounds are invalid (min >= max).
    InvalidScaleBounds,
    /// Action attempts to write to a status field.
    StatusInWriteList,
    /// First threshold does not equal scale minimum.
    FirstThresholdMismatch,

    // Action validation (E5000 series)
    /// Action references an invalid target.
    InvalidActionReference,
    /// Action writes to a field it didn't declare.
    UndeclaredWrite,

    // Module validation (E6000 series)
    /// Module identifier is malformed.
    MalformedModuleId,
    /// Multiple modules have conflicting version requirements.
    ConflictingModuleVersions,
    /// Multiple world laws claim the same exclusive domain.
    MultipleWorldLawsForDomain,
    /// Magic system lacks required physics bridge.
    MissingMagicBridge,
    /// A declared world-law / magic module has no registered implementation.
    /// Accepted as a declaration, never simulated.
    UnsupportedWorldLaw,
    /// A declared magic-bridge module has no registered implementation of the
    /// correct kind. Accepted as a declaration, never executed.
    UnsupportedMagicBridge,

    // Security and limits (E7000 series)
    /// Input exceeds maximum nesting depth.
    MaxDepthExceeded,
    /// Collection exceeds maximum size.
    MaxCollectionSizeExceeded,
    /// String exceeds maximum length.
    MaxStringLengthExceeded,

    // Orbital / celestial validation (E8000 series)
    /// A body's declared parent chain contains a cycle (including self-parent).
    ParentCycle,
    /// An orbit's frame is inconsistent with its parent reference's frame.
    ReferenceFrameMismatch,
    /// A Keplerian element is outside its valid range.
    InvalidOrbitalElement,
    /// An orbital declaration references an unsupported (non-executable) model,
    /// such as a custom reference frame.
    UnsupportedOrbitalModel,

    // Environment validation (E9100 series)
    /// A gas composition's fractions are out of bounds or do not sum within tolerance.
    CompositionOutOfBounds,
    /// An environment numeric value is invalid (zero pressure, inverted temperature,
    /// or out-of-range surface coverage).
    InvalidEnvironmentValue,
    /// An environment declaration references an unsupported (non-executable) module,
    /// such as a climate model with no registered implementation.
    UnsupportedEnvironmentModule,
    /// An asset reference has a missing or malformed content digest.
    MissingAssetDigest,

    // Status / vitality validation (E9200 series)
    /// A status declares an evaluator but is missing a window or cut points.
    IncompleteEvaluatorDeclaration,
    /// A status references an evaluator with no registered implementation.
    UnsupportedEvaluator,
    /// A status references a baseline artifact with no registered content.
    UnsupportedBaseline,

    // Action capability validation (E9300 series)
    /// An action read/write field path is malformed.
    MalformedFieldPath,
    /// An action declares a mutating write not covered by any declared capability.
    UncoveredWrite,

    // Generic (E9000 series)
    /// Generic validation error.
    Generic,
}

impl DiagnosticCode {
    /// Get the stable error code identifier.
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingRequiredField => "E1001",
            Self::InvalidFieldType => "E1002",
            Self::InvalidEnumVariant => "E1003",
            Self::NegativeMass => "E2001",
            Self::InvalidTemperatureRange => "E2002",
            Self::InconsistentOrbitalParameters => "E2003",
            Self::ContradictoryConstraints => "E2004",
            Self::MissingUnit => "E3001",
            Self::IncompatibleDimensions => "E3002",
            Self::InvalidTemperatureDelta => "E3003",
            Self::UnregisteredCustomDimension => "E3004",
            Self::InvalidNumericValue => "E3005",
            Self::NumericOverflow => "E3006",
            Self::UnsortedThresholds => "E4001",
            Self::DuplicateThreshold => "E4002",
            Self::ThresholdOutOfRange => "E4003",
            Self::InvalidScaleBounds => "E4004",
            Self::StatusInWriteList => "E4005",
            Self::FirstThresholdMismatch => "E4006",
            Self::InvalidActionReference => "E5001",
            Self::UndeclaredWrite => "E5002",
            Self::MalformedModuleId => "E6001",
            Self::ConflictingModuleVersions => "E6002",
            Self::MultipleWorldLawsForDomain => "E6003",
            Self::MissingMagicBridge => "E6004",
            Self::UnsupportedWorldLaw => "E6005",
            Self::UnsupportedMagicBridge => "E6006",
            Self::MaxDepthExceeded => "E7001",
            Self::MaxCollectionSizeExceeded => "E7002",
            Self::MaxStringLengthExceeded => "E7003",
            Self::ParentCycle => "E8001",
            Self::ReferenceFrameMismatch => "E8002",
            Self::InvalidOrbitalElement => "E8003",
            Self::UnsupportedOrbitalModel => "E8004",
            Self::CompositionOutOfBounds => "E9101",
            Self::InvalidEnvironmentValue => "E9102",
            Self::UnsupportedEnvironmentModule => "E9103",
            Self::MissingAssetDigest => "E9104",
            Self::IncompleteEvaluatorDeclaration => "E9201",
            Self::UnsupportedEvaluator => "E9202",
            Self::UnsupportedBaseline => "E9203",
            Self::MalformedFieldPath => "E9301",
            Self::UncoveredWrite => "E9302",
            Self::Generic => "E9999",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Structured diagnostic message for validation results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: DiagnosticCode,
    /// Severity level.
    pub severity: Severity,
    /// Primary diagnostic message.
    pub message: String,
    /// Optional source location.
    pub span: Option<SourceSpan>,
    /// Additional notes and suggestions.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Create a new diagnostic.
    pub fn new(code: DiagnosticCode, severity: Severity, message: String) -> Self {
        Self {
            code,
            severity,
            message,
            span: None,
            notes: Vec::new(),
        }
    }

    /// Create an error diagnostic.
    pub fn error(code: DiagnosticCode, message: String) -> Self {
        Self::new(code, Severity::Error, message)
    }

    /// Create a warning diagnostic.
    pub fn warning(code: DiagnosticCode, message: String) -> Self {
        Self::new(code, Severity::Warning, message)
    }

    /// Create an info diagnostic.
    pub fn info(code: DiagnosticCode, message: String) -> Self {
        Self::new(code, Severity::Info, message)
    }

    /// Add a source span to this diagnostic.
    #[must_use]
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    /// Add a note to this diagnostic.
    #[must_use]
    pub fn with_note(mut self, note: String) -> Self {
        self.notes.push(note);
        self
    }

    /// Add multiple notes to this diagnostic.
    #[must_use]
    pub fn with_notes(mut self, notes: Vec<String>) -> Self {
        self.notes.extend(notes);
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: [{}] {}", self.severity, self.code, self.message)?;

        if let Some(span) = &self.span {
            if let Some(source) = &span.source {
                write!(f, " ({}:{}:{})", source, span.start, span.end)?;
            } else {
                write!(f, " (@{}:{})", span.start, span.end)?;
            }
        }

        for note in &self.notes {
            write!(f, "\n  note: {}", note)?;
        }

        Ok(())
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by span first (for determinism), then by code, then by message
        self.span
            .cmp(&other.span)
            .then_with(|| self.code.code().cmp(other.code.code()))
            .then_with(|| self.message.cmp(&other.message))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Info.to_string(), "info");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Error.to_string(), "error");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }

    #[test]
    fn test_diagnostic_code_stable() {
        assert_eq!(DiagnosticCode::MissingRequiredField.code(), "E1001");
        assert_eq!(DiagnosticCode::UnsortedThresholds.code(), "E4001");
        assert_eq!(DiagnosticCode::Generic.code(), "E9999");
    }

    #[test]
    fn test_orbital_diagnostic_codes_stable() {
        assert_eq!(DiagnosticCode::ParentCycle.code(), "E8001");
        assert_eq!(DiagnosticCode::ReferenceFrameMismatch.code(), "E8002");
        assert_eq!(DiagnosticCode::InvalidOrbitalElement.code(), "E8003");
        assert_eq!(DiagnosticCode::UnsupportedOrbitalModel.code(), "E8004");
    }

    #[test]
    fn test_environment_diagnostic_codes_stable() {
        assert_eq!(DiagnosticCode::CompositionOutOfBounds.code(), "E9101");
        assert_eq!(DiagnosticCode::InvalidEnvironmentValue.code(), "E9102");
        assert_eq!(DiagnosticCode::UnsupportedEnvironmentModule.code(), "E9103");
        assert_eq!(DiagnosticCode::MissingAssetDigest.code(), "E9104");
    }

    #[test]
    fn test_status_vitality_diagnostic_codes_stable() {
        assert_eq!(
            DiagnosticCode::IncompleteEvaluatorDeclaration.code(),
            "E9201"
        );
        assert_eq!(DiagnosticCode::UnsupportedEvaluator.code(), "E9202");
    }

    #[test]
    fn test_action_capability_diagnostic_codes_stable() {
        assert_eq!(DiagnosticCode::MalformedFieldPath.code(), "E9301");
        assert_eq!(DiagnosticCode::UncoveredWrite.code(), "E9302");
    }

    #[test]
    fn test_world_law_diagnostic_codes_stable() {
        assert_eq!(DiagnosticCode::MultipleWorldLawsForDomain.code(), "E6003");
        assert_eq!(DiagnosticCode::MissingMagicBridge.code(), "E6004");
        assert_eq!(DiagnosticCode::UnsupportedWorldLaw.code(), "E6005");
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error(
            DiagnosticCode::MissingRequiredField,
            "Field 'name' is required".to_string(),
        );

        assert_eq!(diag.code, DiagnosticCode::MissingRequiredField);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "Field 'name' is required");
        assert!(diag.span.is_none());
        assert!(diag.notes.is_empty());
    }

    #[test]
    fn test_diagnostic_with_span() {
        let span = SourceSpan::with_source(10, 20, "test.hpdl".to_string());
        let diag = Diagnostic::error(DiagnosticCode::InvalidFieldType, "Invalid type".to_string())
            .with_span(span.clone());

        assert_eq!(diag.span, Some(span));
    }

    #[test]
    fn test_diagnostic_with_notes() {
        let diag = Diagnostic::error(DiagnosticCode::Generic, "Error".to_string())
            .with_note("First note".to_string())
            .with_note("Second note".to_string());

        assert_eq!(diag.notes.len(), 2);
        assert_eq!(diag.notes[0], "First note");
        assert_eq!(diag.notes[1], "Second note");
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::error(
            DiagnosticCode::MissingRequiredField,
            "Field 'name' is required".to_string(),
        );

        let display = diag.to_string();
        assert!(display.contains("error"));
        assert!(display.contains("E1001"));
        assert!(display.contains("Field 'name' is required"));
    }

    #[test]
    fn test_diagnostic_display_with_span() {
        let span = SourceSpan::with_source(10, 20, "test.hpdl".to_string());
        let diag = Diagnostic::error(DiagnosticCode::InvalidFieldType, "Invalid type".to_string())
            .with_span(span);

        let display = diag.to_string();
        assert!(display.contains("test.hpdl"));
        assert!(display.contains("10"));
        assert!(display.contains("20"));
    }

    #[test]
    fn test_diagnostic_display_with_notes() {
        let diag = Diagnostic::error(DiagnosticCode::Generic, "Error".to_string())
            .with_note("Try this instead".to_string());

        let display = diag.to_string();
        assert!(display.contains("note: Try this instead"));
    }

    #[test]
    fn test_diagnostic_ordering() {
        let diag1 = Diagnostic::error(DiagnosticCode::MissingRequiredField, "Error 1".to_string())
            .with_span(SourceSpan::new(0, 10));

        let diag2 = Diagnostic::error(DiagnosticCode::MissingRequiredField, "Error 2".to_string())
            .with_span(SourceSpan::new(20, 30));

        assert!(diag1 < diag2);
    }

    #[test]
    fn test_source_span_creation() {
        let span = SourceSpan::new(5, 15);
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 15);
        assert!(span.source.is_none());

        let span_with_source = SourceSpan::with_source(5, 15, "file.hpdl".to_string());
        assert_eq!(span_with_source.source, Some("file.hpdl".to_string()));
    }

    /// Every variant maps to its stable code and the mapping is a bijection.
    /// This exercises all arms of `DiagnosticCode::code()` (previously several
    /// arms had no coverage) and guards against accidental code collisions.
    #[test]
    fn test_all_diagnostic_codes_are_stable_and_unique() {
        use std::collections::BTreeSet;
        let all = [
            (DiagnosticCode::MissingRequiredField, "E1001"),
            (DiagnosticCode::InvalidFieldType, "E1002"),
            (DiagnosticCode::InvalidEnumVariant, "E1003"),
            (DiagnosticCode::NegativeMass, "E2001"),
            (DiagnosticCode::InvalidTemperatureRange, "E2002"),
            (DiagnosticCode::InconsistentOrbitalParameters, "E2003"),
            (DiagnosticCode::ContradictoryConstraints, "E2004"),
            (DiagnosticCode::MissingUnit, "E3001"),
            (DiagnosticCode::IncompatibleDimensions, "E3002"),
            (DiagnosticCode::InvalidTemperatureDelta, "E3003"),
            (DiagnosticCode::UnregisteredCustomDimension, "E3004"),
            (DiagnosticCode::InvalidNumericValue, "E3005"),
            (DiagnosticCode::NumericOverflow, "E3006"),
            (DiagnosticCode::UnsortedThresholds, "E4001"),
            (DiagnosticCode::DuplicateThreshold, "E4002"),
            (DiagnosticCode::ThresholdOutOfRange, "E4003"),
            (DiagnosticCode::InvalidScaleBounds, "E4004"),
            (DiagnosticCode::StatusInWriteList, "E4005"),
            (DiagnosticCode::FirstThresholdMismatch, "E4006"),
            (DiagnosticCode::InvalidActionReference, "E5001"),
            (DiagnosticCode::UndeclaredWrite, "E5002"),
            (DiagnosticCode::MalformedModuleId, "E6001"),
            (DiagnosticCode::ConflictingModuleVersions, "E6002"),
            (DiagnosticCode::MultipleWorldLawsForDomain, "E6003"),
            (DiagnosticCode::MissingMagicBridge, "E6004"),
            (DiagnosticCode::UnsupportedWorldLaw, "E6005"),
            (DiagnosticCode::UnsupportedMagicBridge, "E6006"),
            (DiagnosticCode::MaxDepthExceeded, "E7001"),
            (DiagnosticCode::MaxCollectionSizeExceeded, "E7002"),
            (DiagnosticCode::MaxStringLengthExceeded, "E7003"),
            (DiagnosticCode::ParentCycle, "E8001"),
            (DiagnosticCode::ReferenceFrameMismatch, "E8002"),
            (DiagnosticCode::InvalidOrbitalElement, "E8003"),
            (DiagnosticCode::UnsupportedOrbitalModel, "E8004"),
            (DiagnosticCode::CompositionOutOfBounds, "E9101"),
            (DiagnosticCode::InvalidEnvironmentValue, "E9102"),
            (DiagnosticCode::UnsupportedEnvironmentModule, "E9103"),
            (DiagnosticCode::MissingAssetDigest, "E9104"),
            (DiagnosticCode::IncompleteEvaluatorDeclaration, "E9201"),
            (DiagnosticCode::UnsupportedEvaluator, "E9202"),
            (DiagnosticCode::UnsupportedBaseline, "E9203"),
            (DiagnosticCode::MalformedFieldPath, "E9301"),
            (DiagnosticCode::UncoveredWrite, "E9302"),
            (DiagnosticCode::Generic, "E9999"),
        ];
        let mut seen = BTreeSet::new();
        for (code, expected) in all {
            assert_eq!(code.code(), expected, "{code:?} has an unexpected code");
            // Display delegates to code().
            assert_eq!(code.to_string(), expected);
            assert!(seen.insert(expected), "duplicate code {expected}");
        }
        assert_eq!(seen.len(), 44, "all variants must be covered");
    }

    #[test]
    fn test_warning_and_info_constructors() {
        let w = Diagnostic::warning(DiagnosticCode::UnsupportedWorldLaw, "w".to_string());
        assert_eq!(w.severity, Severity::Warning);
        let i = Diagnostic::info(DiagnosticCode::Generic, "i".to_string());
        assert_eq!(i.severity, Severity::Info);
        let n = Diagnostic::new(DiagnosticCode::Generic, Severity::Info, "n".to_string());
        assert_eq!(n.severity, Severity::Info);
    }

    #[test]
    fn test_with_notes_bulk_and_display_without_source() {
        let diag = Diagnostic::warning(DiagnosticCode::Generic, "msg".to_string())
            .with_notes(vec!["one".to_string(), "two".to_string()])
            .with_span(SourceSpan::new(3, 7));
        assert_eq!(diag.notes.len(), 2);
        let display = diag.to_string();
        // Span with no source uses the "(@start:end)" form.
        assert!(display.contains("(@3:7)"), "got: {display}");
        assert!(display.contains("note: one"));
        assert!(display.contains("note: two"));
    }

    #[test]
    fn test_partial_cmp_matches_ord() {
        let a = Diagnostic::error(DiagnosticCode::Generic, "a".to_string());
        let b = Diagnostic::error(DiagnosticCode::Generic, "b".to_string());
        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less));
    }
}
