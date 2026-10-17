# RON Frontend — Design Summary

## Purpose

The RON frontend (`hpdl-ron`) provides a Serde-based deserialization path for HPDL planet definitions authored in RON (Rusty Object Notation) format. It ensures semantic equivalence with native HPDL syntax by routing all validation through the shared `hpdl-sema` pipeline.

## Architecture

```
┌─────────────────┐
│  .ron file      │
│  (UTF-8 text)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  SourceDocument │ ← File path, content, span mapping
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   RonLoader     │ ← Security limits, Serde deserialization
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ PlanetDefinition│ ← hpdl-model types (deserialized)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ ParsedDefinition│ ← Wrapper with source context
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  hpdl-sema      │ ← Shared semantic validation
│  Validator      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Validated      │ ← Ready for PIR compilation
│  Definition     │
└─────────────────┘
```

## Key Components

### RonLoader
- Orchestrates RON deserialization
- Enforces security limits (file size, nesting depth, collection length)
- Maps parse errors to source locations
- Produces `ParsedDefinition`

### SourceDocument
- Wraps source text with path/identifier
- Provides span-to-line/column mapping
- Supports diagnostic error reporting
- Validates UTF-8 encoding

### ParsedDefinition
- Holds deserialized `PlanetDefinition`
- Preserves source document reference
- Provides `validate()` method for semantic checking
- Bridges parse and validation phases

### Error Types
- `Error::Parse`: RON syntax or structure errors
- `Error::Sema`: Semantic validation errors (from `hpdl-sema`)
- `Error::Io`: File system errors
- `Error::Security`: Resource limit violations

### FrontendDiagnostic
- Rich error messages with source context
- Span highlighting in error output
- Integration with `hpdl_sema::Diagnostic`
- Stable diagnostic codes

## Security Model

### Input Validation
- File size checked before reading entire content
- Nesting depth tracked during deserialization
- Collection lengths validated before allocation
- All input treated as untrusted

### Limits (Configurable via SecurityConfig)
- `max_file_size`: 10 MB (default)
- `max_nesting_depth`: 64 levels (default)
- `max_collection_length`: 10,000 items (default)

### Safe Patterns
- No `unsafe` code
- No `unwrap()`/`expect()` on user input paths
- Checked arithmetic for size calculations
- Path traversal prevention

## Semantic Equivalence

### Shared Validation
- All semantic rules in `hpdl-sema`, not duplicated in RON frontend
- RON frontend is purely syntactic adapter
- Same `Validator` instance used regardless of source format

### PIR Equivalence
- Equivalent RON and HPDL must produce identical PIR bytes
- Same planet definition must hash identically
- Cross-frontend equivalence tests verify parity

### Deterministic Ordering
- Use `BTreeMap` where order affects output
- Sort diagnostics by span before returning
- Consistent validation order regardless of field order

## Extension Mechanism

### Namespaced Extensions
- Extension fields use namespace prefix (e.g., `_ext::custom`)
- Unknown fields without namespace are rejected
- Extension namespace configurable
- Documented in RFC

### Example
```ron
PlanetDefinition(
    id: BodyId("earth"),
    physical: PhysicalProperties(
        mass_kg: 5972000000000000000000000,
        // Standard field
    ),
    _ext::custom_metadata: "user-defined",  // Extension field
)
```

## Integration Points

### With hpdl-model
- Deserializes directly into `PlanetDefinition`
- No transformation or adaptation layer
- Serde derives on model types

### With hpdl-sema
- Calls `Validator::validate()` on parsed definitions
- Maps semantic diagnostics to RON source spans
- Preserves all validation errors

### With hpdl-compiler (future)
- Provides validated definitions for PIR compilation
- Compiler handles package resolution and lowering
- No direct PIR generation in RON frontend

## Error Handling Philosophy

### Parse Errors
- Syntax errors in RON format
- Malformed structure (missing fields, wrong types)
- Clear indication of location and expected format

### Semantic Errors
- From `hpdl-sema`, not RON frontend
- Physical constraint violations
- Status threshold errors
- Action declaration errors

### Security Errors
- Resource limit exceeded
- Path traversal attempt
- Invalid UTF-8 encoding

### User Experience
- Show source excerpt with error location
- Highlight exact error position
- Provide suggestions when possible
- Reference diagnostic codes for stable lookup

## Performance Characteristics

### Targets
- Parse typical planet definition: < 50ms
- Validate typical definition: < 100ms (hpdl-sema)
- Memory efficient (no unbounded allocation)

### Optimization Strategy
- Stream large files when possible
- Minimize allocation and copying
- Efficient span tracking
- Benchmark-driven optimization

## Testing Strategy

### Unit Tests
- Each component in isolation
- Error path coverage
- Boundary conditions

### Security Tests
- Resource limit enforcement
- Malformed input handling
- Path security

### Integration Tests
- End-to-end parse and validate
- Example planet definitions
- Semantic error propagation

### Equivalence Tests
- RON vs HPDL frontend parity (when native syntax available)
- PIR hash equality
- Diagnostic equivalence

### Golden Tests
- Frozen expected outputs
- Parse results
- Error messages
- PIR bytes

## Versioning Strategy

### Crate Version (SemVer)
- API changes follow Rust SemVer
- Breaking API changes require major version bump

### RON Schema Version
- Independent of crate version
- Embedded in RON format (future)
- Migration support for old schemas

### PIR Format Version
- Independent of RON schema
- Handled by `hpdl-pir` crate
- Multiple versions supported

## Documentation Requirements

### Rustdoc
- All public types and functions
- Module-level architecture docs
- Examples for common patterns

### Examples
- Basic usage: load and validate
- Error handling patterns
- Security configuration
- Advanced features

### Guides
- RON format conventions
- Extension field usage
- Integration with compiler
- Migration from other formats

## Open Design Decisions

### File Extensions
- `.ron` or `.hpdl.ron`?
- Document in RFC after decision

### Extension Namespace Syntax
- `_ext::field` or `@ext/field`?
- Document in RFC after decision

### Numeric Precision
- How to handle RON numeric formats vs canonical SI?
- Validation vs automatic conversion?

### Comment Support
- Standard RON supports comments
- Should HPDL RON format allow comments?
- How to preserve for round-tripping?

These decisions should be documented in `docs/rfcs/` with:
- Problem statement
- Alternatives considered
- Chosen solution and rationale
- Compatibility impact
- Test requirements

## Success Criteria

1. ✅ RON files deserialize to `PlanetDefinition`
2. ✅ All semantic validation via `hpdl-sema` (no duplication)
3. ✅ Security limits enforced and tested
4. ✅ Clear error messages with source context
5. ✅ Cross-frontend PIR equivalence verified
6. ✅ All tests pass (unit, integration, security, equivalence)
7. ✅ Documentation complete
8. ✅ No unsafe code or panics on untrusted input
9. ✅ CI passing (fmt, clippy, test, deny)
10. ✅ Ready for stable release

## References

- Design Document: `docs/design/04-ron-frontend.md`
- Requirements: `.kiro/specs/04-ron-frontend/requirements.md`
- Tasks: `.kiro/specs/04-ron-frontend/tasks.md`
- HPDL Spec: (normative, take precedence)
- Steering: `.kiro/steering/` (workflow, product, architecture)
