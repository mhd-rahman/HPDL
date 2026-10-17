# Semantic Validation — Design

## Architecture Overview

The semantic validation system is implemented in the `hpdl-sema` crate and provides a single validation pipeline used by all HPDL frontends. It validates the domain model from `hpdl-model` and produces either a `ValidatedDefinition` or a collection of structured `Diagnostic` messages.

```
┌─────────────┐     ┌─────────────┐
│  hpdl-ron   │────>│             │
└─────────────┘     │             │
                    │  hpdl-sema  │───> ValidatedDefinition
┌─────────────┐     │             │      or
│ hpdl-syntax │────>│             │      Vec<Diagnostic>
└─────────────┘     └─────────────┘
                           │
                           v
                    ┌─────────────┐
                    │ hpdl-model  │
                    └─────────────┘
```

## Core Types

### Diagnostic System

```rust
/// Diagnostic severity level
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Source location information
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
    pub source: Option<String>, // file path or identifier
}

/// Structured diagnostic message
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub notes: Vec<String>,
}

/// Stable diagnostic codes
pub enum DiagnosticCode {
    // Type validation
    MissingRequiredField,
    InvalidFieldType,
    InvalidEnumVariant,
    
    // Constraint validation
    NegativeMass,
    InvalidTemperatureRange,
    InconsistentOrbitalParameters,
    ContradictoryConstraints,
    
    // Unit validation
    MissingUnit,
    IncompatibleDimensions,
    InvalidTemperatureDelta,
    UnregisteredCustomDimension,
    InvalidNumericValue,
    NumericOverflow,
    
    // Status validation
    UnsortedThresholds,
    DuplicateThreshold,
    ThresholdOutOfRange,
    InvalidScaleBounds,
    StatusInWriteList,
    
    // Action validation
    InvalidActionReference,
    UndeclaredWrite,
    
    // Module validation
    MalformedModuleId,
    ConflictingModuleVersions,
    MultipleWorldLawsForDomain,
    MissingMagicBridge,
    
    // Generic
    Generic,
}
```

### Validated Definition

```rust
/// Wrapper for a successfully validated planet definition
pub struct ValidatedDefinition {
    definition: PlanetDefinition,
    validation_metadata: ValidationMetadata,
}

struct ValidationMetadata {
    validator_version: String,
    validation_timestamp: String,
    applied_rules: Vec<String>,
}
```

## Validation Pipeline

### Main Validator

```rust
pub struct Validator {
    type_checker: TypeChecker,
    constraint_solver: ConstraintSolver,
    unit_registry: UnitRegistry,
    config: ValidatorConfig,
}

pub struct ValidatorConfig {
    pub max_nesting_depth: usize,
    pub max_collection_size: usize,
    pub strict_mode: bool,
}

impl Validator {
    pub fn new(config: ValidatorConfig) -> Self;
    
    pub fn validate(
        &self,
        definition: PlanetDefinition
    ) -> Result<ValidatedDefinition, Vec<Diagnostic>>;
    
    // Internal validation passes
    fn validate_structure(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    fn validate_types(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    fn validate_constraints(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    fn validate_units(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    fn validate_statuses(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    fn validate_actions(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    fn validate_modules(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
}
```

### Type Checker

```rust
pub struct TypeChecker {
    strict: bool,
}

impl TypeChecker {
    pub fn check_field_types(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    pub fn check_required_fields(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    pub fn check_enum_variants(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    
    fn validate_body_id(&self, id: &BodyId) -> Option<Diagnostic>;
    fn validate_physical_properties(&self, props: &PhysicalProperties) -> Vec<Diagnostic>;
    fn validate_trait_set(&self, traits: &TraitSet) -> Vec<Diagnostic>;
}
```

### Constraint Solver

```rust
pub struct ConstraintSolver {
    tolerance: f64,
}

impl ConstraintSolver {
    pub fn check_physical_constraints(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    pub fn check_cross_field_consistency(&self, def: &PlanetDefinition) -> Vec<Diagnostic>;
    
    fn validate_mass_positive(&self, mass: Option<&Mass>) -> Option<Diagnostic>;
    fn validate_temperature_range(&self, temp: &Temperature) -> Option<Diagnostic>;
    fn validate_orbital_parameters(&self, orbital: &OrbitalParams) -> Vec<Diagnostic>;
    fn detect_contradictions(&self, constraints: &[Constraint]) -> Vec<Diagnostic>;
}
```

### Unit Registry

```rust
pub struct UnitRegistry {
    custom_dimensions: BTreeMap<String, DimensionSpec>,
}

pub struct DimensionSpec {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub base_unit: String,
}

impl UnitRegistry {
    pub fn new() -> Self;
    
    pub fn register_custom_dimension(&mut self, spec: DimensionSpec) -> Result<(), Error>;
    
    pub fn validate_unit(&self, value: &Quantity) -> Option<Diagnostic>;
    pub fn validate_dimension(&self, value: &Quantity, expected: Dimension) -> Option<Diagnostic>;
    pub fn validate_numeric(&self, value: f64) -> Option<Diagnostic>;
    
    fn check_temperature_vs_delta(&self, value: &Quantity, context: Context) -> Option<Diagnostic>;
    fn check_custom_dimension(&self, dimension: &str) -> Option<Diagnostic>;
    fn check_no_nan_inf(&self, value: f64) -> Option<Diagnostic>;
}
```

## Validation Rules

### Status Threshold Validation (Amendment A1)

The status threshold validation is critical and follows these rules:

```rust
fn validate_status_thresholds(
    &self,
    scale_min: i32,
    scale_max: i32,
    thresholds: &BTreeMap<StatusBand, i32>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    // Rule 1: Scale bounds must be valid
    if scale_min >= scale_max {
        diagnostics.push(Diagnostic::new(
            DiagnosticCode::InvalidScaleBounds,
            Severity::Error,
            format!("Scale min ({}) must be less than max ({})", scale_min, scale_max),
        ));
        return diagnostics;
    }
    
    // Extract threshold values in declaration order
    let mut values: Vec<i32> = thresholds.values().copied().collect();
    
    // Rule 2: First threshold must equal scale minimum
    if let Some(&first) = values.first() {
        if first != scale_min {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::ThresholdOutOfRange,
                Severity::Error,
                format!("First threshold ({}) must equal scale minimum ({})", first, scale_min),
            ));
        }
    }
    
    // Rule 3: All thresholds must be within [scale_min, scale_max]
    for &value in &values {
        if value < scale_min || value > scale_max {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::ThresholdOutOfRange,
                Severity::Error,
                format!("Threshold {} is outside scale range [{}, {}]", value, scale_min, scale_max),
            ));
        }
    }
    
    // Rule 4: Thresholds must be strictly increasing (sorted and unique)
    for window in values.windows(2) {
        if window[0] >= window[1] {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::UnsortedThresholds,
                Severity::Error,
                format!("Thresholds must be strictly increasing: {} >= {}", window[0], window[1]),
            ));
        }
    }
    
    diagnostics
}
```

### Action Status Write Validation

```rust
fn validate_action_writes(&self, action: &ActionDeclaration) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    for write_target in &action.writes {
        if write_target.starts_with("status.") {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::StatusInWriteList,
                Severity::Error,
                format!("Action '{}' attempts to write to status '{}'. Statuses are derived and cannot be written.", 
                    action.name, write_target),
            ).with_note("Statuses are computed from state by versioned evaluators."));
        }
    }
    
    diagnostics
}
```

## Security and Bounds Checking

All validation operations must enforce limits:

```rust
const DEFAULT_MAX_NESTING_DEPTH: usize = 32;
const DEFAULT_MAX_COLLECTION_SIZE: usize = 10_000;
const DEFAULT_MAX_STRING_LENGTH: usize = 1_000_000;

fn check_depth_limit(&self, depth: usize, span: SourceSpan) -> Option<Diagnostic> {
    if depth > self.config.max_nesting_depth {
        Some(Diagnostic::new(
            DiagnosticCode::Generic,
            Severity::Error,
            format!("Nesting depth {} exceeds limit {}", depth, self.config.max_nesting_depth),
        ).with_span(span))
    } else {
        None
    }
}

fn check_collection_limit(&self, size: usize, span: SourceSpan) -> Option<Diagnostic> {
    if size > self.config.max_collection_size {
        Some(Diagnostic::new(
            DiagnosticCode::Generic,
            Severity::Error,
            format!("Collection size {} exceeds limit {}", size, self.config.max_collection_size),
        ).with_span(span))
    } else {
        None
    }
}
```

## Determinism Guarantees

To ensure deterministic validation across platforms:

1. Use `BTreeMap` and `BTreeSet` for all ordered collections
2. Never iterate `HashMap` when order affects diagnostics
3. Sort diagnostic lists by span location before returning
4. Use explicit comparison for floating-point (with epsilon)
5. Never use locale-dependent string operations
6. Never use system time or random values in validation logic

## Error Handling

All validation functions follow this pattern:

```rust
// Return Vec<Diagnostic> - empty means success
fn validate_component(&self, component: &Component) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    // Check each rule
    if let Some(diag) = self.check_rule_1(component) {
        diagnostics.push(diag);
    }
    
    if let Some(diag) = self.check_rule_2(component) {
        diagnostics.push(diag);
    }
    
    diagnostics
}

// Main validator accumulates diagnostics
pub fn validate(&self, def: PlanetDefinition) -> Result<ValidatedDefinition, Vec<Diagnostic>> {
    let mut all_diagnostics = Vec::new();
    
    all_diagnostics.extend(self.validate_structure(&def));
    all_diagnostics.extend(self.validate_types(&def));
    all_diagnostics.extend(self.validate_constraints(&def));
    // ... more validation passes
    
    if all_diagnostics.iter().any(|d| matches!(d.severity, Severity::Error)) {
        Err(all_diagnostics)
    } else {
        Ok(ValidatedDefinition::new(def, ValidationMetadata::new()))
    }
}
```

## Testing Strategy

### Unit Tests

Each validation rule gets:
- Positive test (valid input passes)
- Negative test (invalid input fails with correct diagnostic)
- Boundary tests (edge cases)

### Integration Tests

- Cross-frontend equivalence (same RON and HPDL produce same diagnostics)
- Complex planet definitions with multiple validation issues
- Performance tests with large inputs

### Golden Tests

Store expected diagnostics for:
- Standard valid planets
- Common error patterns
- Threshold validation cases (0-100 scale with 24.5, 25, 49.999 cut points)

### Property Tests

Use `proptest` or similar for:
- Threshold ordering properties
- Numeric overflow detection
- Constraint satisfaction

## Implementation Order

1. **Phase 1: Foundation** (Task 1-3)
   - Error types and diagnostic system
   - Validator skeleton and config
   - Basic structure validation

2. **Phase 2: Core Validation** (Task 4-6)
   - Type checker implementation
   - Constraint solver implementation
   - Unit registry implementation

3. **Phase 3: Specialized Rules** (Task 7-9)
   - Status threshold validation (Amendment A1)
   - Action validation
   - Module and world law validation

4. **Phase 4: Testing & Polish** (Task 10-12)
   - Comprehensive test suite
   - Golden test fixtures
   - Documentation and examples

## Future Enhancements

- Incremental validation for editor support
- Diagnostic quick-fixes and suggestions
- Performance profiling and optimization
- Parallel validation of independent components
- Validation plugin system for custom rules
