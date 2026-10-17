# RON Frontend — Requirements

## Overview

The RON frontend provides a Serde-based RON (Rusty Object Notation) deserialization path for HPDL planet definitions. It enables authoring in RON format while ensuring identical semantics to native HPDL syntax by routing all deserialized content through the shared `hpdl-sema` validation pipeline.

## Goals

1. **Semantic Equivalence**: RON and HPDL syntax produce identical PIR for equivalent definitions
2. **Single Validation Pipeline**: All semantic rules live in `hpdl-sema`, not duplicated in RON frontend
3. **Syntactic Convenience**: Leverage Serde for straightforward RON parsing
4. **Security**: Handle untrusted RON input safely with bounds checking
5. **Diagnostics**: Provide clear error messages with source location context

## Functional Requirements

### FR1: RON Deserialization Infrastructure

**FR1.1**: Provide `RonLoader` for loading RON files
- Must accept file paths or string content
- Must deserialize into `hpdl_model::PlanetDefinition`
- Must return `Result<ParsedDefinition, Error>`
- Must track source location for diagnostics

**FR1.2**: Define `SourceDocument` type
- Must wrap the source text
- Must provide path/identifier for the document
- Must support span-to-line/column mapping
- Must be suitable for diagnostic reporting

**FR1.3**: Define `ParsedDefinition` wrapper
- Must contain the deserialized `PlanetDefinition`
- Must preserve source document reference
- Must be ready for semantic validation

**FR1.4**: Support RON-specific extensions
- Must allow namespaced extension fields (e.g., `_ext::custom`)
- Must reject unknown fields without extension namespace
- Must document extension mechanism

### FR2: Error Handling and Diagnostics

**FR2.1**: Define `FrontendDiagnostic` type
- Must distinguish parse errors from semantic errors
- Must include source location (span, line, column)
- Must provide clear error messages
- Must integrate with `hpdl_sema::Diagnostic`

**FR2.2**: Implement typed error variants
- Parse errors (RON syntax, malformed structure)
- Validation errors (from `hpdl-sema`)
- I/O errors (file not found, permission denied)
- Security errors (size limits, nesting depth)

**FR2.3**: Provide context-rich error messages
- Must show relevant source excerpt
- Must highlight error location
- Must suggest corrections when possible
- Must reference diagnostic codes

### FR3: Integration with Semantic Validator

**FR3.1**: Route all deserialized definitions through `hpdl-sema`
- Must call `hpdl_sema::Validator::validate()`
- Must not implement semantic rules in RON frontend
- Must preserve all validation diagnostics
- Must fail on any semantic error

**FR3.2**: Map RON spans to validation diagnostics
- Must translate RON parse positions to `SourceSpan`
- Must maintain accurate line/column information
- Must support nested structure paths

**FR3.3**: Ensure deterministic validation order
- Must validate in consistent order regardless of RON field order
- Must produce same diagnostics as native HPDL for equivalent input
- Must use ordered collections (BTreeMap) where necessary

### FR4: Security and Resource Limits

**FR4.1**: Enforce input size limits
- Must validate file size before reading entire content
- Must reject files exceeding configured maximum (default: 10 MB)
- Must provide clear error message for oversized files

**FR4.2**: Enforce nesting depth limits
- Must track nesting depth during deserialization
- Must reject deeply nested structures (default: 64 levels)
- Must prevent stack overflow attacks

**FR4.3**: Enforce collection length limits
- Must validate array/map lengths before allocation
- Must reject excessively large collections (default: 10,000 items)
- Must prevent memory exhaustion attacks

**FR4.4**: Handle malformed input safely
- Must never panic on invalid RON syntax
- Must never panic on unexpected field types
- Must use checked arithmetic for size calculations
- Must treat all RON input as untrusted

### FR5: File Loading and Path Resolution

**FR5.1**: Support file-based loading
- Must load from absolute paths
- Must load from workspace-relative paths
- Must handle I/O errors gracefully
- Must validate UTF-8 encoding

**FR5.2**: Support in-memory loading
- Must accept RON string content directly
- Must support testing without filesystem
- Must allow synthetic source identifiers

**FR5.3**: Path handling
- Must normalize paths for consistent diagnostics
- Must never follow symlinks outside workspace
- Must reject path traversal attempts (e.g., `../../etc/passwd`)

### FR6: Canonical Equivalence

**FR6.1**: Ensure PIR equivalence
- Equivalent RON and HPDL must produce identical PIR bytes
- Same planet definition must hash identically
- Must pass cross-frontend equivalence tests

**FR6.2**: Document semantic parity
- Must list any intentional RON-specific behaviors
- Must document extension namespace mechanism
- Must explain how RON maps to HPDL concepts

**FR6.3**: Maintain compatibility
- Must version RON schema separately from compiler
- Must support migration of old RON definitions
- Must preserve backward compatibility within major versions

## Non-Functional Requirements

### NFR1: Performance
- Must deserialize typical planet definitions in < 50ms
- Must not allocate unbounded memory during parsing
- Must stream large files when possible

### NFR2: Security
- Must enforce all resource limits without exceptions
- Must validate input before allocation
- Must use safe Rust patterns (no `unsafe` code)
- Must pass security audit checks (`cargo deny`)

### NFR3: Reliability
- Must use `Result` for all fallible operations
- Must not use `unwrap()` or `expect()` on user input
- Must use checked arithmetic for size calculations
- Must provide typed errors via `thiserror`

### NFR4: Documentation
- Must document public API with rustdoc
- Must provide examples of valid RON definitions
- Must document error handling patterns
- Must explain integration with semantic validator

### NFR5: Testing
- Must include unit tests for deserialization
- Must include negative tests for parse errors
- Must include security tests for resource limits
- Must include cross-frontend equivalence tests
- Must maintain golden test fixtures

## Constraints

- Must depend on `hpdl-model` and `hpdl-sema`
- Must use `ron` crate for deserialization
- Must NOT implement semantic validation rules
- Must NOT depend on `hpdl-pir` or `hpdl-compiler`
- Must follow Rust standards: `#![forbid(unsafe_code)]`
- Must work on Linux, macOS, and Windows
- Must be suitable for publishing as a stable crate

## Success Criteria

1. RON definitions successfully deserialize to `PlanetDefinition`
2. All semantic validation goes through `hpdl-sema`
3. Equivalent RON/HPDL produce identical PIR and hashes
4. All security limits enforced and tested
5. Clear error messages with source locations
6. All tests pass including cross-platform checks
7. CI checks pass (fmt, clippy, test, deny)
8. Documentation complete with examples
9. No unsafe code or panics on untrusted input

## Out of Scope

- Native HPDL syntax parsing (handled by `hpdl-syntax`, deferred)
- PIR compilation (handled by `hpdl-compiler`)
- Package resolution (handled by `hpdl-compiler`)
- Lockfile management (handled by `hpdl-compiler`)
- Runtime execution (handled by HORIZON engine)
- LSP support (deferred to `hpdl-lsp`)
- Syntax highlighting (editor-specific)

## Dependencies

- `hpdl-model`: Provides `PlanetDefinition` and domain types
- `hpdl-sema`: Provides `Validator` and semantic checks
- `ron`: RON deserialization library
- `thiserror`: Error type definitions
- `serde`: Serialization framework

## Example Usage

```rust
use hpdl_ron::{RonLoader, SourceDocument};
use hpdl_sema::Validator;

// Load and parse RON file
let source = SourceDocument::from_file("planets/earth.ron")?;
let parsed = RonLoader::new().parse(&source)?;

// Validate through shared pipeline
let validator = Validator::new();
let validated = validator.validate(parsed.definition)?;

// Pass to compiler for PIR generation
// (handled by hpdl-compiler, not this crate)
```

## Integration Points

### With `hpdl-model`
- Deserializes into `PlanetDefinition`
- Uses all domain types unchanged
- Respects type constraints

### With `hpdl-sema`
- Calls `Validator::validate()` on all parsed definitions
- Maps parse spans to `SourceSpan` for diagnostics
- Propagates semantic errors unchanged

### With `hpdl-compiler` (future)
- Provides `ParsedDefinition` for compilation
- Compiler handles PIR generation
- Compiler orchestrates package resolution

## Versioning Strategy

- RON schema version independent of compiler version
- Schema changes require version bump in RON format
- Old RON schemas supported via migration
- PIR format version independent of RON schema version

## Open Questions

1. Exact RON file extension convention (`.ron` or `.hpdl.ron`)?
2. Extension namespace syntax (`_ext::` or `@ext/`)?
3. Maximum file size limit (10 MB reasonable default)?
4. Should RON support comments (standard RON does)?
5. Handling of RON-specific numeric formats vs canonical SI?

These decisions should be recorded as ADRs in `docs/rfcs/`.
