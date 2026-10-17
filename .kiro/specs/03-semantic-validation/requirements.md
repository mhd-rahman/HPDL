# Semantic Validation — Requirements

## Overview

The semantic validation system provides a single, shared validation pipeline that ensures all HPDL definitions are correct, consistent, and complete before they are lowered to PIR. This validator must work identically across all frontends (RON, native HPDL syntax) to guarantee semantic equivalence.

## Goals

1. **Single Source of Truth**: One validation pipeline used by all frontends
2. **Type Safety**: Strong typing with explicit units and dimensions
3. **Deterministic**: Same input produces identical diagnostics
4. **Secure**: Handle untrusted input safely with bounds checking
5. **Informative**: Clear, actionable error messages with source locations

## Functional Requirements

### FR1: Core Validation Infrastructure

**FR1.1**: Provide a `Validator` struct that coordinates all validation passes
- Must accept `PlanetDefinition` from `hpdl-model`
- Must return `Result<ValidatedDefinition, Vec<Diagnostic>>`
- Must be frontend-agnostic

**FR1.2**: Define structured diagnostic types
- Must include severity (error, warning, info)
- Must include diagnostic codes for stable error identification
- Must track source location via `SourceSpan`
- Must provide human-readable messages

**FR1.3**: Support validated definition wrapper
- Must contain the validated model
- Must preserve all validation metadata
- Must be suitable for PIR compilation

### FR2: Type and Constraint Validation

**FR2.1**: Implement `TypeChecker` for domain model validation
- Must validate all field types are correctly structured
- Must verify required fields are present
- Must check field value ranges are valid
- Must validate enum variants are known

**FR2.2**: Implement `ConstraintSolver` for cross-field validation
- Must validate physical constraints (e.g., mass > 0)
- Must check temperature ranges are physically valid
- Must verify orbital parameters are consistent
- Must detect contradictory constraints

**FR2.3**: Validate absence semantics
- Must distinguish between `Absent`, `Unknown`, and `Generated`
- Must never apply Earth defaults implicitly
- Must error on required fields that are absent without declaration

### FR3: Unit and Dimension Validation

**FR3.1**: Implement `UnitRegistry` for unit validation
- Must verify all quantities have explicit units
- Must validate unit dimensions match expected types
- Must distinguish `Temperature` from `TemperatureDelta`
- Must support custom dimensions (e.g., `arcane::Aether`)

**FR3.2**: Validate numeric values
- Must reject NaN and infinities
- Must use checked arithmetic
- Must validate ranges before conversion
- Must detect overflow/underflow

**FR3.3**: Validate unit compatibility
- Must prevent implicit conversions between incompatible dimensions
- Must require explicit world-law bridges for custom dimension conversions
- Must validate SI as canonical representation

### FR4: Status Declaration Validation (Amendment A1)

**FR4.1**: Validate status threshold structure
- Must verify thresholds are ordered (strictly increasing)
- Must verify thresholds are unique (no duplicates)
- Must verify first threshold equals scale minimum
- Must verify last threshold is within scale maximum
- Must validate scale bounds are valid (min < max)

**FR4.2**: Validate status bands
- Must correctly interpret bands as `[lower, next)` with last band including maximum
- Must validate example: `0..=100` with `{0, 25, 50, 75}` produces 4 bands
- Must reject out-of-range scores
- Must never clamp scores silently

**FR4.3**: Validate status declarations are read-only
- Must verify statuses are never in action `writes` lists
- Must reject any attempt to assign status values
- Must validate statuses are derived from state

### FR5: Action Declaration Validation

**FR5.1**: Validate action declarations
- Must verify `reads` and `writes` lists are valid references
- Must validate module references exist
- Must enforce declared access patterns

**FR5.2**: Enforce status immutability
- Must reject actions that write to status fields
- Must produce compile-time error with clear message
- Must provide example of correct pattern

### FR6: Module and World Law Validation

**FR6.1**: Validate module references
- Must verify module identifiers are well-formed
- Must validate version constraints
- Must check for conflicting module requirements

**FR6.2**: Validate world law declarations
- Must verify world law mode is valid (`additive` or replacement)
- Must validate only one authoritative law per exclusive domain
- Must verify magic-to-physics bridges are declared
- Must validate deterministic module properties

### FR7: Cross-Frontend Equivalence

**FR7.1**: Ensure diagnostic equivalence
- Must produce identical diagnostics for equivalent RON and HPDL inputs
- Must use same error codes across frontends
- Must maintain same validation order

**FR7.2**: Ensure deterministic validation
- Must not depend on HashMap iteration order
- Must not use locale-dependent operations
- Must not use floating-point where precision matters
- Must use BTreeMap/BTreeSet for ordered data

## Non-Functional Requirements

### NFR1: Performance
- Must validate typical planet definitions in < 100ms
- Must handle deeply nested structures efficiently
- Must limit memory allocation for untrusted inputs

### NFR2: Security
- Must validate input sizes before allocation
- Must enforce nesting depth limits
- Must check collection length limits
- Must handle malformed input without panicking
- Must treat all input as untrusted

### NFR3: Reliability
- Must use `Result` types for all fallible operations
- Must not use `unwrap()`, `expect()`, or panic on user input
- Must use checked arithmetic for numeric operations
- Must provide typed errors via `thiserror`

### NFR4: Documentation
- Must document all public types and functions with rustdoc
- Must provide examples of valid and invalid inputs
- Must document all diagnostic codes
- Must explain validation semantics

### NFR5: Testing
- Must include unit tests for each validation rule
- Must include negative tests (invalid inputs)
- Must include boundary cases (e.g., 24.5, 25, 49.999 bands)
- Must include cross-frontend equivalence tests
- Must maintain golden test fixtures

## Constraints

- Must depend only on `hpdl-model`
- Must NOT depend on `hpdl-pir`, `hpdl-compiler`, or `hpdl-cli`
- Must follow Rust standards: `#![forbid(unsafe_code)]`
- Must work on Linux, macOS, and Windows
- Must be suitable for publishing as a stable crate

## Success Criteria

1. All validation rules from design document are implemented
2. All tests pass including golden fixtures
3. CI checks pass (fmt, clippy, test, deny)
4. Cross-platform compatibility verified
5. Documentation is complete
6. No unsafe code or panics on untrusted input
7. Diagnostic codes are stable and documented

## Out of Scope

- PIR compilation (handled by `hpdl-compiler`)
- Source parsing (handled by `hpdl-ron`, `hpdl-syntax`)
- Runtime execution (handled by HORIZON engine)
- Package resolution (handled by `hpdl-compiler`)
- Exact numeric lowering to fixed-point (handled by `hpdl-compiler`)
