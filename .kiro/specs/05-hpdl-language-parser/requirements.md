# HPDL Language Parser — Requirements

## Overview

The HPDL language parser (`hpdl-syntax`) provides a native UTF-8 lexer and parser for `.hpdl` files. It transforms HPDL source text into the same `PlanetDefinition` model used by the RON frontend, ensuring semantic equivalence between both input formats.

## Goals

1. **Native Syntax**: Support a clean, declarative syntax for defining planets
2. **Semantic Equivalence**: RON and HPDL produce identical PIR for equivalent definitions
3. **Error Recovery**: Provide helpful diagnostics with source location context
4. **Security**: Handle untrusted input safely with resource limits
5. **Deterministic**: Same input always produces same output

## Functional Requirements

### FR1: Lexer (Tokenization)

**FR1.1**: UTF-8 Source Processing
- Must accept UTF-8 encoded source files
- Must reject invalid UTF-8 with clear error messages
- Must handle BOM (Byte Order Mark) if present
- Must normalize line endings (CRLF → LF)

**FR1.2**: Token Recognition
- **Keywords**: `language`, `planet`, `body`, `physical`, `traits`, `status`, `action`, `module`, `migration`
- **Literals**: integers, floats (with explicit units), strings, booleans
- **Identifiers**: alphanumeric with underscores, hyphens
- **Operators**: `=`, `:`, `,`, `{`, `}`, `[`, `]`, `(`, `)`
- **Comments**: Line comments (`//`) and block comments (`/* */`)
- **Whitespace**: Spaces, tabs, newlines (significant for parsing)

**FR1.3**: Token Stream
- Must produce a stream of tokens with:
  - Token type (keyword, identifier, literal, etc.)
  - Span (start/end byte positions)
  - Source text slice
- Must preserve token order
- Must include EOF (end-of-file) token

**FR1.4**: Lexer Error Handling
- Invalid characters → specific error
- Unclosed string literals → error with suggestion
- Unclosed block comments → error with location
- Invalid number formats → error with expected format

### FR2: Parser (Syntax Analysis)

**FR2.1**: Grammar Structure
```
file ::= language_decl body_decl

language_decl ::= "language" "hpdl" STRING ";"

body_decl ::= "planet" IDENT "{" field_list "}"

field_list ::= field ("," field)* ","?

field ::= IDENT ":" value

value ::= 
    | STRING
    | NUMBER unit?
    | BOOL
    | IDENT
    | list
    | object

list ::= "[" (value ("," value)* ","?)? "]"

object ::= "{" (field ("," field)* ","?)? "}"
```

**FR2.2**: AST Construction
- Must build Abstract Syntax Tree (AST) from tokens
- Must preserve source spans for all nodes
- Must support nested structures
- Must handle optional trailing commas

**FR2.3**: Parser Error Recovery
- Must attempt to recover from errors and continue parsing
- Must report all errors found (not just first)
- Must provide helpful error messages with context
- Must suggest fixes where applicable

**FR2.4**: Validation Rules (Syntax Only)
- Language version must be present and first
- Planet declaration must have unique identifier
- Field names must be valid identifiers
- Nesting depth must not exceed 64 levels
- Collection sizes must not exceed 10,000 items

### FR3: AST to Model Conversion

**FR3.1**: Type Mapping
- AST nodes → `hpdl_model` types
- String literals → String fields
- Number literals with units → PhysicalProperties fields
- Lists → Vec<T>
- Objects → Structs

**FR3.2**: Unit Parsing
- Parse unit suffixes (e.g., `6371000 m`, `5.972e24 kg`)
- Convert to canonical integer representation
- Support scientific notation
- Validate unit compatibility

**FR3.3**: Identifier Resolution
- Map field names to struct fields
- Handle nested structures
- Support optional fields (None if absent)
- Reject unknown field names

**FR3.4**: Error Handling
- Type mismatches → clear error with expected type
- Missing required fields → error listing missing fields
- Unknown fields → error with did-you-mean suggestion
- Invalid units → error with valid unit list

### FR4: Source Location Tracking

**FR4.1**: Span Management
- Track byte offset for every token
- Support span union for multi-token constructs
- Provide line/column conversion
- Support source excerpt extraction

**FR4.2**: Diagnostic Reporting
- Associate errors with source locations
- Show relevant source lines
- Highlight error span
- Provide notes with additional context

**FR4.3**: Multi-file Support (Future)
- Track file name/path for each span
- Support relative imports
- Handle circular dependencies

### FR5: Integration with Semantic Validator

**FR5.1**: Model Production
- Produce `PlanetDefinition` identical to RON frontend
- Preserve all semantic information
- Include metadata for diagnostics

**FR5.2**: Validation Pipeline
- Parse → AST → Model → Semantic Validation
- Route through `hpdl-sema` (same as RON)
- Collect and merge parse + semantic diagnostics
- Fail if any errors present

**FR5.3**: Equivalence Guarantee
- Same planet in HPDL and RON → identical model
- Identical model → identical PIR bytes
- Identical PIR → identical hash

### FR6: Security and Resource Limits

**FR6.1**: Input Validation
- Maximum file size: 10 MB
- Maximum nesting depth: 64 levels
- Maximum collection length: 10,000 items
- Maximum identifier length: 256 characters

**FR6.2**: Attack Prevention
- No recursive descent stack overflow
- No exponential backtracking
- No unbounded memory allocation
- Timeout for pathological inputs (optional)

**FR6.3**: Safe Parsing
- No `unwrap()` or `panic()` on user input
- Use checked arithmetic
- Validate indices before access
- Handle malformed UTF-8 gracefully

## Non-Functional Requirements

### NFR1: Performance
- Parse typical planet file (< 100 KB) in < 50ms
- Lexer should be zero-copy where possible
- AST construction should be efficient
- Memory usage proportional to file size

### NFR2: Error Quality
- Errors must be actionable
- Include source location (file:line:col)
- Show source excerpt with highlighting
- Suggest fixes when possible
- Use stable error codes

### NFR3: Maintainability
- Clean separation: Lexer → Parser → AST → Model
- Testable components
- Well-documented grammar
- Examples for each construct

### NFR4: Determinism
- Same input always produces same output
- No randomness or non-deterministic behavior
- Cross-platform consistency
- Stable error order

### NFR5: Documentation
- Complete rustdoc for public API
- Grammar specification document
- Syntax examples
- Migration guide from RON

## Example HPDL Syntax

```hpdl
language hpdl "1.0";

planet kepler_186f {
    id: "kepler-186f",
    
    physical: {
        mass_kg: 6500000000000000000000000,
        radius_m: 7100000,
        surface_gravity_mms2: 11500,
    },
    
    traits: {
        tags: ["terrestrial", "habitable", "exoplanet"],
    },
    
    modules: [],
    statuses: [],
    actions: [],
    migrations: [],
}
```

## Constraints

- Must depend on `hpdl-model` and `hpdl-sema`
- Must NOT depend on `hpdl-pir` or `hpdl-compiler`
- Must follow Rust standards: `#![forbid(unsafe_code)]`
- Must work on Linux, macOS, and Windows
- Must produce identical output to RON frontend for equivalent inputs

## Success Criteria

1. ✅ HPDL files parse to `PlanetDefinition`
2. ✅ Equivalent HPDL/RON produce identical PIR and hashes
3. ✅ Clear error messages with source locations
4. ✅ Security limits enforced
5. ✅ All tests pass (unit, integration, equivalence)
6. ✅ CI checks pass (fmt, clippy, test, deny)
7. ✅ Documentation complete
8. ✅ No unsafe code or panics on untrusted input

## Out of Scope

- Formatter (pretty-printer for HPDL)
- LSP (language server) support
- Syntax highlighting
- Auto-completion
- Refactoring tools
- PIR compilation (handled by `hpdl-compiler`)
- Package imports (handled by `hpdl-compiler`)

## Open Questions

1. **Exact syntax for units**: `6371000m` or `6371000 m` or `6371_000 m`?
2. **Comments in expressions**: Allow `/* comment */` anywhere?
3. **String escape sequences**: Support `\n`, `\t`, `\uXXXX`?
4. **Number formats**: Hex (`0x`), binary (`0b`), octal (`0o`)?
5. **Trailing commas**: Required, optional, or forbidden?

These should be documented in an RFC after decision.

## Dependencies

- `hpdl-model`: Target model types
- `hpdl-sema`: Semantic validation
- `thiserror`: Error types
- `logos` or custom lexer: Tokenization
- Consider `chumsky` or `nom` for parsing (or hand-written recursive descent)

## References

- Design Doc: `docs/design/05-hpdl-language-parser.md`
- RON Frontend: `.kiro/specs/04-ron-frontend/`
- Semantic Validation: `.kiro/specs/03-semantic-validation/`
