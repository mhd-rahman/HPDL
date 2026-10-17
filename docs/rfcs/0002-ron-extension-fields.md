# RFC 0002: RON Extension Fields

**Status:** Proposed  
**Date:** 2026-10-15  
**Author:** HPDL Team

## Summary

This RFC proposes a mechanism for including custom metadata in RON-format HPDL definitions through namespaced extension fields, allowing tools and workflows to attach metadata without affecting semantic validation.

## Motivation

Users may want to include metadata in planet definitions for:
- **Authorship**: Who created or last modified the definition
- **Timestamps**: When the definition was created or updated
- **Tooling**: Build system metadata, version control info
- **Documentation**: Internal notes, references to external docs
- **Workflow**: CI/CD tags, deployment metadata

However, adding arbitrary fields to HPDL definitions could:
- Break semantic validation
- Create ambiguity about standard vs. custom fields
- Cause compatibility issues with different tools

## Proposed Solution

### Extension Namespace Prefix

Extension fields must use a namespace prefix to clearly distinguish them from standard HPDL fields. The default prefix is `_ext::`.

**Example:**
```ron
PlanetDefinition(
    language_version: "1.0",
    body: CelestialBody(
        id: BodyId("kepler-186f"),
        physical: PhysicalProperties(
            mass_kg: 6500000000000000000000000,
            radius_m: 7100000,
        ),
        traits: TraitSet(tags: ["terrestrial", "habitable"]),
        modules: [],
        statuses: [],
        actions: [],
        migrations: [],
        // Extension fields below
        _ext::author: "Dr. Jane Smith",
        _ext::created: "2026-01-15",
        _ext::project: "exoplanet-catalog",
        _ext::notes: "Candidate for biosignature study",
    )
)
```

### Validation Rules

1. **Unknown standard fields are rejected**: Fields without the extension prefix that aren't part of the HPDL schema cause validation errors.

2. **Extension fields are allowed but ignored**: Fields with the extension prefix are accepted but do not participate in semantic validation or PIR generation.

3. **Extension field naming rules**:
   - Must start with the configured namespace prefix (default: `_ext::`)
   - Name after prefix must be non-empty
   - Name may contain: alphanumeric characters, underscores, hyphens
   - Name may NOT contain: dots, spaces, special characters

4. **Configuration**: The extension namespace prefix is configurable per `RonLoader` instance to support different conventions.

### Behavior

- **Parsing**: Extension fields are deserialized from RON but stored separately or discarded
- **Validation**: Extension fields do not affect semantic validation (they are not passed to `hpdl-sema`)
- **PIR Generation**: Extension fields are NOT included in canonical PIR output
- **Preservation**: Extension fields MAY be preserved in round-trip scenarios (future enhancement)

## Alternatives Considered

### 1. Comments for metadata
**Approach**: Use RON comments for metadata instead of fields.

**Pros**:
- Simple, no validation needed
- Already supported by RON format

**Cons**:
- Not machine-readable
- Lost during deserialization
- Cannot be programmatically accessed or validated

**Decision**: Comments are complementary but insufficient for structured metadata.

### 2. Separate metadata files
**Approach**: Store metadata in separate files alongside planet definitions.

**Pros**:
- Complete separation of concerns
- No impact on definition files

**Cons**:
- Requires managing multiple files
- Metadata can drift from definition
- More complex for simple use cases

**Decision**: This approach works for complex metadata but is overkill for simple annotations.

### 3. Standard metadata block
**Approach**: Define a standard `metadata` field in HPDL schema.

**Pros**:
- First-class support
- Included in schema

**Cons**:
- Every tool must handle it
- Becomes part of semantic validation
- Included in PIR (increases size)
- Less flexible for tool-specific needs

**Decision**: Too heavyweight; extension fields provide better separation.

### 4. Different namespace syntaxes

Considered alternatives to `_ext::`:
- `@ext/field` (inspired by npm scopes)
- `x-field` (HTTP header convention)
- `__field` (double underscore)
- `ext.field` (dot notation)

**Decision**: `_ext::` chosen because:
- Follows Rust's `::` namespace convention
- Less likely to conflict with existing conventions
- Clearly distinguishable from standard fields
- Configurable for users with other preferences

## Compatibility Impact

### Forward Compatibility
- Older parsers that don't recognize extension fields will reject unknown fields (safe failure)
- Extension fields are opt-in; existing definitions are unaffected

### Backward Compatibility
- No impact on existing definitions without extension fields
- Extension field support is additive

### PIR Format
- **No impact**: Extension fields are not included in PIR
- PIR hashes remain identical regardless of extension field presence

### Cross-Frontend Equivalence
- Native HPDL syntax (when implemented) must support same extension mechanism
- Semantic equivalence maintained: same definition yields same PIR

## Implementation Plan

### Phase 1: Core Support (Current)
- ✅ `ExtensionConfig` type for configuration
- ✅ Extension field validation
- ✅ Configurable namespace prefix
- ✅ Documentation

### Phase 2: Integration (Next)
- Integrate with `RonLoader` to filter extension fields during parsing
- Update error messages to suggest extension prefix for unknown fields
- Add examples showing extension field usage

### Phase 3: Enhancement (Future)
- Extension field preservation in round-trip scenarios
- Tool API for accessing extension metadata
- Standard extension field registry (optional conventions)

## Open Questions

### Q1: Should we have a registry of "known" extension fields?

**Proposed Answer**: No, not initially. Extension fields are explicitly tool-specific. However, we may document common conventions (e.g., `_ext::author`, `_ext::created`) for interoperability.

### Q2: Should extension fields be included in any output format?

**Proposed Answer**: Not in PIR (canonical format). However, we could provide a debug/development mode that preserves them in non-canonical output for tooling purposes.

### Q3: What about nested extension fields?

**Example:**
```ron
_ext::metadata: (
    author: "Jane",
    created: "2026-01-15",
)
```

**Proposed Answer**: Supported. The validation only checks the top-level field name. The structure within an extension field is unrestricted.

### Q4: Should we validate extension field values?

**Proposed Answer**: No. Extension fields are opaque to HPDL. Tools may implement their own validation if needed.

## Test Requirements

1. ✅ Valid extension field names accepted
2. ✅ Invalid extension field names rejected
3. Unknown standard fields rejected (to be implemented in RonLoader integration)
4. Extension fields do not affect PIR output (to be verified in integration)
5. Custom namespace prefixes work correctly
6. Disabling extension support causes appropriate errors

## Documentation Requirements

- ✅ RFC documenting design and rationale
- User guide showing extension field usage examples
- API documentation for `ExtensionConfig`
- Migration guide for adding extension fields to existing definitions

## Success Criteria

1. Users can add custom metadata to RON definitions without validation errors
2. Extension fields have no impact on PIR generation or semantic validation
3. Clear error messages guide users when they accidentally use non-standard fields
4. Configuration allows teams to use preferred namespace conventions
5. Design is extensible to native HPDL syntax (when implemented)

## References

- HTTP X- Header Convention: RFC 6648
- NPM Package Scoping: https://docs.npmjs.com/cli/v7/using-npm/scope
- Rust Naming Conventions: https://rust-lang.github.io/api-guidelines/naming.html
