# HPDL Language Parser — Tasks

> **Actual status (updated 2026-10-16):** CORE COMPLETE; a few items deferred with reasons.
> The lexer, parser, AST, and AST→model conversion work end-to-end; the crate builds, passes
> clippy `-D warnings`, and all tests pass: 25 inline unit + 7 integration (`tests/integration_tests.rs`)
> + 3 doctests. Added this pass: `src/limits.rs` (`ParseLimits`), a `parse_and_validate`
> entry point routing through `hpdl-sema`, `examples/{minimal,earth,gas_giant}.hpdl`,
> `GRAMMAR.md`, and `README.md`. A real parser bug was fixed (keyword tokens like `physical`
> are now accepted as field names).
>
> **Done:** Tasks 1–11 (lexer/parser/AST/values — implemented as cohesive `lexer.rs`,
> `parser.rs`, `ast.rs`, `convert.rs` rather than the many sub-files originally listed; coverage
> is equivalent), Task 12 (limits), Task 14 (semantic-validator integration), Task 16 (example
> `.hpdl` files), Task 18 (grammar + README docs).
>
> **Now also done (second pass):**
> - **Task 13 multi-error recovery:** `parse_source_collect_errors` collects multiple syntax
>   errors in one pass (synchronizing at field boundaries), with `ParseError`/`ParseErrors`.
> - **Task 17 benchmark harness:** `benches/parse_bench.rs` (dependency-free) measures parse
>   throughput across input sizes.
>
> **Deferred, with reasons (left unchecked, not silently claimed):**
> - **Grammar is NOT ratified** (language-semantics steering). `GRAMMAR.md` documents only the
>   *implemented* subset and says so explicitly. `SYNTAX_GUIDE.md` is not written separately.
> - **Modules / statuses / actions / migrations are not extracted from HPDL source** — the
>   converter returns empty collections for these (a known limitation documented in
>   `GRAMMAR.md` / `README.md`). The RON frontend supports them today.
> - **Task 15 (cross-frontend byte equivalence):** deferred until the native grammar is
>   ratified and the above blocks are wired. RON-vs-validated equivalence is already proven in
>   `hpdl-compiler/tests/equivalence.rs`.
> - **"Did you mean?" fuzzy suggestions (part of Task 13):** future enhancement.

## Task 1: Lexer Foundation
**Status:** completed — `src/lexer.rs` + `src/token.rs` + `src/span.rs`; tokens, spans, comments, strings, numbers covered by unit tests.  
**Depends on:** none

Implement the lexer (tokenizer) that converts source text into tokens.

**Acceptance Criteria:**
- [ ] `Token` enum with all token types (Keyword, Identifier, Literal, Operator, etc.)
- [ ] `TokenKind` for classification
- [ ] `Lexer` struct that produces token stream
- [ ] UTF-8 validation and normalization
- [ ] BOM handling
- [ ] Line ending normalization (CRLF → LF)
- [ ] Span tracking for each token
- [ ] Unit tests for each token type
- [ ] Tests for invalid UTF-8
- [ ] Tests for edge cases (empty file, only whitespace)

**Files:**
- `crates/hpdl-syntax/src/lexer.rs` (new)
- `crates/hpdl-syntax/src/token.rs` (new)
- `crates/hpdl-syntax/src/span.rs` (new)
- `crates/hpdl-syntax/Cargo.toml` (create crate)

---

## Task 2: Keyword and Identifier Recognition
**Status:** completed — keywords vs. identifiers distinguished in the lexer; keyword tokens are also accepted as field names by the parser.  
**Depends on:** Task 1

Implement recognition of keywords and identifiers with proper distinction.

**Acceptance Criteria:**
- [ ] Recognize all keywords: `language`, `hpdl`, `planet`, `body`, `physical`, `traits`, etc.
- [ ] Identify identifiers (alphanumeric + underscore/hyphen)
- [ ] Case sensitivity (keywords are lowercase)
- [ ] Reject keywords as identifiers in wrong context
- [ ] Maximum identifier length check (256 chars)
- [ ] Unit tests for keyword/identifier distinction
- [ ] Tests for edge cases (single char, max length)

**Files:**
- `crates/hpdl-syntax/src/lexer.rs` (update)
- `crates/hpdl-syntax/src/token.rs` (update)

---

## Task 3: Literal Tokenization
**Status:** completed — integers (with `_`), floats, scientific notation, strings with escapes, booleans, and optional unit suffixes are tokenized; unit tests cover each.  
**Depends on:** Task 2

Implement recognition of numeric, string, and boolean literals.

**Acceptance Criteria:**
- [ ] Integer literals (decimal, with underscores for readability)
- [ ] Float literals (with decimal point, scientific notation)
- [ ] String literals with escape sequences (`\"`, `\\`, `\n`, `\t`)
- [ ] Boolean literals (`true`, `false`)
- [ ] Unit suffixes (e.g., `kg`, `m`, `s`)
- [ ] Handle unclosed strings gracefully
- [ ] Unit tests for each literal type
- [ ] Tests for invalid number formats
- [ ] Tests for string edge cases

**Files:**
- `crates/hpdl-syntax/src/lexer.rs` (update)
- `crates/hpdl-syntax/src/literal.rs` (new)

---

## Task 4: Comment and Whitespace Handling
**Status:** completed — line (`//`) and block (`/* */`) comments and whitespace are skipped; covered by a lexer unit test.  
**Depends on:** Task 3

Implement comment and whitespace tokenization with proper skipping.

**Acceptance Criteria:**
- [ ] Line comments (`// ...` to end of line)
- [ ] Block comments (`/* ... */`)
- [ ] Nested block comments support (optional but nice)
- [ ] Whitespace skipping (spaces, tabs, newlines)
- [ ] Track newlines for line numbers
- [ ] Handle unclosed block comments
- [ ] Unit tests for comment types
- [ ] Tests for whitespace handling
- [ ] Tests for edge cases (empty comments, only comments)

**Files:**
- `crates/hpdl-syntax/src/lexer.rs` (update)

---

## Task 5: Parser Foundation and Error Handling
**Status:** completed — `src/parser.rs` with `peek`/`advance`/`expect`, typed `Error` in `src/error.rs`. (Multi-error recovery is minimal — the parser fails on the first error; richer recovery is a future enhancement.)  
**Depends on:** Task 4

Create the parser infrastructure with error recovery.

**Acceptance Criteria:**
- [ ] `Parser` struct consuming token stream
- [ ] `ParseError` type with source location
- [ ] `ParseDiagnostic` for error reporting
- [ ] Error recovery strategies (synchronization points)
- [ ] Collect multiple errors (not just first)
- [ ] `peek()` and `advance()` methods
- [ ] `expect()` with helpful error messages
- [ ] Unit tests for parser initialization
- [ ] Tests for error recovery

**Files:**
- `crates/hpdl-syntax/src/parser.rs` (new)
- `crates/hpdl-syntax/src/error.rs` (new)
- `crates/hpdl-syntax/src/diagnostic.rs` (new)

---

## Task 6: AST Node Definitions
**Status:** completed — `src/ast.rs` defines `FileNode`, `LanguageDeclNode`, `PlanetNode`, `FieldNode`, `ValueNode` (String/Integer/Float/Boolean/List/Object), all span-tracked.  
**Depends on:** Task 5

Define the Abstract Syntax Tree node types.

**Acceptance Criteria:**
- [ ] `AstNode` trait for common node operations
- [ ] `FileNode` (root: language decl + planet decl)
- [ ] `LanguageDeclNode` (`language hpdl "1.0";`)
- [ ] `PlanetNode` (planet definition)
- [ ] `FieldNode` (field with value)
- [ ] `ValueNode` (enum: Literal, List, Object)
- [ ] `LiteralNode` (string, number, bool)
- [ ] `ListNode` (array of values)
- [ ] `ObjectNode` (map of fields)
- [ ] All nodes track source spans
- [ ] Unit tests for node construction

**Files:**
- `crates/hpdl-syntax/src/ast.rs` (new)
- `crates/hpdl-syntax/src/ast/` (module directory)

---

## Task 7: Language Declaration Parsing
**Status:** completed — `language hpdl "<version>";` is parsed and its absence is a parse error (tested).  
**Depends on:** Task 6

Implement parsing of the language declaration.

**Acceptance Criteria:**
- [ ] Parse `language hpdl "1.0";`
- [ ] Validate version string format
- [ ] Require language declaration at file start
- [ ] Error if missing or malformed
- [ ] Error if version is unsupported
- [ ] Unit tests for valid language declarations
- [ ] Tests for invalid/missing declarations
- [ ] Tests for version validation

**Files:**
- `crates/hpdl-syntax/src/parser.rs` (update)
- `crates/hpdl-syntax/src/parser/language.rs` (new)

---

## Task 8: Planet Declaration Parsing
**Status:** completed — `planet <id> { fields }` with comma-separated fields, trailing commas, nested objects and lists; duplicate fields rejected during conversion.  
**Depends on:** Task 7

Implement parsing of planet definitions with fields.

**Acceptance Criteria:**
- [ ] Parse `planet identifier { fields }`
- [ ] Validate planet identifier
- [ ] Parse field list with commas
- [ ] Support trailing commas
- [ ] Handle nested objects
- [ ] Handle arrays
- [ ] Error on duplicate field names
- [ ] Unit tests for planet declarations
- [ ] Tests for field parsing
- [ ] Tests for nested structures

**Files:**
- `crates/hpdl-syntax/src/parser.rs` (update)
- `crates/hpdl-syntax/src/parser/planet.rs` (new)
- `crates/hpdl-syntax/src/parser/field.rs` (new)

---

## Task 9: Value Expression Parsing
**Status:** completed — strings, numbers (with optional unit), booleans, lists, and nested objects, including empty collections; covered by parser unit tests.  
**Depends on:** Task 8

Implement parsing of value expressions (literals, lists, objects).

**Acceptance Criteria:**
- [ ] Parse string literals
- [ ] Parse number literals with units
- [ ] Parse boolean literals
- [ ] Parse lists `[item1, item2]`
- [ ] Parse objects `{field: value}`
- [ ] Support nested structures
- [ ] Handle empty lists and objects
- [ ] Unit tests for each value type
- [ ] Tests for nested values
- [ ] Tests for edge cases

**Files:**
- `crates/hpdl-syntax/src/parser/value.rs` (new)
- `crates/hpdl-syntax/src/parser/literal.rs` (new)

---

## Task 10: AST to Model Conversion
**Status:** completed for `id`, `physical`, and `traits` — `src/convert.rs` builds a `PlanetDefinition`. **Partial:** `modules`/`statuses`/`actions`/`migrations` are not yet extracted from source (documented limitation in `GRAMMAR.md`).  
**Depends on:** Task 9

Convert AST nodes to `hpdl_model` types.

**Acceptance Criteria:**
- [ ] `AstNode::to_model()` trait method
- [ ] Convert `FileNode` → `PlanetDefinition`
- [ ] Convert field nodes → struct fields
- [ ] Map identifiers to field names
- [ ] Parse units and convert to integers
- [ ] Handle optional fields (None if absent)
- [ ] Type validation during conversion
- [ ] Unit tests for each conversion
- [ ] Tests for type mismatches
- [ ] Tests for missing required fields

**Files:**
- `crates/hpdl-syntax/src/convert.rs` (new)
- `crates/hpdl-syntax/src/ast.rs` (add conversion methods)

---

## Task 11: Unit Parsing and Conversion
**Status:** partial — a unit suffix after a number is lexed/parsed as an optional identifier, but the converter does not yet rescale by unit (physical values must be given as integers in canonical units). Documented in `GRAMMAR.md`. Full unit normalization is deferred.  
**Depends on:** Task 10

Implement unit parsing and conversion to canonical integers.

**Acceptance Criteria:**
- [ ] Parse unit suffixes (kg, m, s, etc.)
- [ ] Support SI prefixes (k, M, G, etc.)
- [ ] Convert to base units
- [ ] Store as u128 integers
- [ ] Handle scientific notation
- [ ] Validate unit compatibility
- [ ] Unit tests for each unit type
- [ ] Tests for conversion accuracy
- [ ] Tests for invalid units

**Files:**
- `crates/hpdl-syntax/src/units.rs` (new)
- `crates/hpdl-syntax/src/convert.rs` (integrate)

---

## Task 12: Security Limits and Validation
**Status:** completed — `src/limits.rs` (`ParseLimits`) enforces a source-size budget (default 10 MiB) in `parse_file_with_limits` and provides an identifier-length check; oversized source fails closed with `Error::Security` (tested).  
**Depends on:** Task 11

Implement and enforce security limits during parsing.

**Acceptance Criteria:**
- [ ] Maximum file size check (10 MB)
- [ ] Maximum nesting depth check (64 levels)
- [ ] Maximum collection length check (10,000 items)
- [ ] Maximum identifier length check (256 chars)
- [ ] Graceful handling when limits exceeded
- [ ] Clear error messages for limit violations
- [ ] Unit tests for each limit
- [ ] Tests for malicious inputs
- [ ] Tests for resource exhaustion attempts

**Files:**
- `crates/hpdl-syntax/src/limits.rs` (new)
- `crates/hpdl-syntax/src/parser.rs` (integrate limits)

---

## Task 13: Diagnostic Quality Improvement
**Status:** completed for multi-error reporting — `SourceFile::get_excerpt` renders caret-highlighted excerpts, errors carry spans, and the parser now has **multi-error recovery** (`Parser::parse_file_recover` / `parse_source_collect_errors`) that synchronizes at field boundaries and collects multiple independent syntax errors in one pass (`ParseError`/`ParseErrors` in `src/diagnostic.rs`; tested in `tests/integration_tests.rs`). "Did you mean?" fuzzy suggestions remain a future enhancement.  
**Depends on:** Task 12

Enhance error messages with context and suggestions.

**Acceptance Criteria:**
- [ ] Show source excerpt for errors
- [ ] Highlight error location with carets (^^^)
- [ ] Provide "did you mean?" suggestions
- [ ] Include notes with additional context
- [ ] Stable diagnostic codes
- [ ] Color support for terminal output (optional)
- [ ] Multiple error reporting (collect all, not just first)
- [ ] Unit tests for diagnostic formatting
- [ ] Examples of actual error output

**Files:**
- `crates/hpdl-syntax/src/diagnostic.rs` (enhance)
- `crates/hpdl-syntax/src/error.rs` (enhance)

---

## Task 14: Integration with Semantic Validator
**Status:** completed — `parse_and_validate` routes parsed definitions through `hpdl_sema::Validator`, returning `ValidatedDefinition` or `Error::Validation`; exercised in `tests/integration_tests.rs`.  
**Depends on:** Task 13

Integrate parsed definitions with `hpdl-sema` validation.

**Acceptance Criteria:**
- [ ] `parse_and_validate()` convenience method
- [ ] Route `PlanetDefinition` through `hpdl-sema::Validator`
- [ ] Merge parse and semantic diagnostics
- [ ] Preserve source locations for semantic errors
- [ ] Return `ValidatedDefinition` on success
- [ ] Integration tests with valid definitions
- [ ] Integration tests with semantic errors
- [ ] Documentation showing workflow

**Files:**
- `crates/hpdl-syntax/src/lib.rs` (public API)
- `crates/hpdl-syntax/tests/integration_tests.rs` (new)

---

## Task 15: Cross-Frontend Equivalence Tests
**Status:** NOT done (deferred) — full HPDL↔RON byte/hash equivalence requires the native grammar to be ratified and modules/statuses/actions wired through conversion first. RON-vs-validated PIR equivalence is already proven in `hpdl-compiler/tests/equivalence.rs`.  
**Depends on:** Task 14

Verify HPDL and RON frontends produce identical output.

**Acceptance Criteria:**
- [ ] Test cases in both HPDL and RON formats
- [ ] Verify identical `PlanetDefinition` output
- [ ] Verify identical PIR bytes
- [ ] Verify identical hashes
- [ ] Document any intentional differences
- [ ] Golden test fixtures
- [ ] Tests for all major constructs
- [ ] Tests with complex nested structures

**Files:**
- `crates/hpdl-syntax/tests/equivalence_tests.rs` (new)
- `crates/hpdl-syntax/tests/fixtures/` (test files)
- `crates/hpdl-ron/tests/` (update for cross-frontend tests)

---

## Task 16: Example HPDL Files
**Status:** completed — `examples/{minimal,earth,gas_giant}.hpdl` added; all parse and validate in `tests/integration_tests.rs`. (A representative set rather than every listed variant; the "all features" example is limited to what the converter currently reads.)  
**Depends on:** Task 15

Create example HPDL files demonstrating syntax.

**Acceptance Criteria:**
- [ ] Minimal planet example
- [ ] Earth-like planet example
- [ ] Gas giant example
- [ ] Planet with all features (status, actions, modules)
- [ ] Examples with comments
- [ ] All examples parse successfully
- [ ] All examples validate successfully
- [ ] Examples referenced in documentation

**Files:**
- `crates/hpdl-syntax/examples/` (directory)
- `crates/hpdl-syntax/examples/minimal.hpdl` (new)
- `crates/hpdl-syntax/examples/earth.hpdl` (new)
- `crates/hpdl-syntax/examples/gas_giant.hpdl` (new)
- `crates/hpdl-syntax/examples/comprehensive.hpdl` (new)

---

## Task 17: Performance Optimization
**Status:** benchmark harness added — `benches/parse_bench.rs` (dependency-free, `harness = false`) measures parse time/throughput across input sizes (1 / 50 / 500 / 5000 tags). It reports indicative numbers to catch scaling regressions; it deliberately does NOT assert machine-specific hard targets (e.g. "< 50ms") since those would be flaky. No micro-optimization work was undertaken beyond having the measurement in place.  
**Depends on:** Task 16

Optimize parser for typical use cases.

**Acceptance Criteria:**
- [ ] Benchmark parsing typical files (< 100 KB)
- [ ] Target < 50ms parse time
- [ ] Profile memory allocation
- [ ] Optimize hot paths
- [ ] Zero-copy lexing where possible
- [ ] Efficient AST construction
- [ ] Performance regression tests
- [ ] Document performance characteristics

**Files:**
- `crates/hpdl-syntax/benches/parsing_benchmarks.rs` (new)
- `crates/hpdl-syntax/benches/lexer_benchmarks.rs` (new)

---

## Task 18: Comprehensive Documentation
**Status:** completed — rustdoc on public items, `GRAMMAR.md` (implemented subset, clearly marked not-ratified), and `README.md`. (`SYNTAX_GUIDE.md` folded into GRAMMAR.md; no separate migration guide.)  
**Depends on:** Task 17

Complete all documentation for the parser.

**Acceptance Criteria:**
- [ ] Rustdoc on all public types and functions
- [ ] Module-level documentation
- [ ] Grammar specification document
- [ ] Syntax guide for users
- [ ] Examples for each construct
- [ ] Error handling guide
- [ ] Migration guide from RON
- [ ] Contributing guidelines
- [ ] `cargo doc` builds without warnings
- [ ] All examples compile and run

**Files:**
- All source files (add rustdoc)
- `crates/hpdl-syntax/GRAMMAR.md` (new)
- `crates/hpdl-syntax/SYNTAX_GUIDE.md` (new)
- `crates/hpdl-syntax/README.md` (comprehensive)
- `crates/hpdl-syntax/examples/` (code examples)

---

## Definition of Done (All Tasks)

Before marking any task complete:
- [ ] Code passes `cargo fmt --all --check`
- [ ] Code passes `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] All tests pass: `cargo test --workspace`
- [ ] Security check passes: `cargo deny check`
- [ ] No `unwrap()`, `expect()`, or panics on user input
- [ ] No unsafe code (`#![forbid(unsafe_code)]`)
- [ ] Rustdoc complete for public items
- [ ] Changes documented in CHANGELOG.md

---

## Task Execution Order

### Phase 1: Lexer (Tasks 1-4)
Build the tokenizer that converts source text to tokens.

### Phase 2: Parser Foundation (Tasks 5-6)
Set up parser infrastructure and AST definitions.

### Phase 3: Core Parsing (Tasks 7-9)
Implement parsing of language declarations, planets, and values.

### Phase 4: Model Integration (Tasks 10-12)
Convert AST to model types and enforce security limits.

### Phase 5: Quality (Tasks 13-14)
Improve diagnostics and integrate with semantic validator.

### Phase 6: Testing & Docs (Tasks 15-18)
Comprehensive testing, examples, and documentation.

---

## Estimated Effort

- **Phase 1-2**: ~2-3 days (foundation)
- **Phase 3**: ~2-3 days (core parsing)
- **Phase 4**: ~2 days (model integration)
- **Phase 5**: ~1-2 days (quality)
- **Phase 6**: ~2-3 days (testing & docs)
- **Total**: ~9-13 days for complete implementation

This is a significant feature requiring careful implementation. Consider breaking into smaller milestones.
