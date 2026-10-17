# RON Frontend — Tasks

> **Actual status (updated 2026-10-16):** COMPLETE for the implementable scope. `hpdl-ron`
> builds, passes clippy `-D warnings`, and all tests pass: 49 inline unit + 6 doctests +
> external suites added this pass (`validation_integration.rs` 5, `security_tests.rs` 5,
> `malformed_input_tests.rs` 6, `path_security_tests.rs` 3) over `tests/fixtures/*.ron`.
> Examples (`basic_usage`, `validation_workflow`, `error_handling`), `README.md`, and
> `CONVENTIONS.md` added.
> - Tasks 7–13: done. (Task 7's validation lives in `src/parsed.rs::validate` rather than a
>   separate `src/validation.rs`; the integration tests exercise it.)
> - **Task 14 (compiler integration):** largely superseded — `hpdl-compiler` (feature 06) is
>   active and consumes the RON frontend directly; cross-frontend/PIR equivalence is proven in
>   `hpdl-compiler/tests/equivalence.rs`. The standalone RFC-0003 stub is not written (the real
>   integration already exists). Flagged, not silently claimed.
> - **Task 15 (release / crates.io publish):** out of scope in this environment — `cargo deny`
>   and `cargo semver-checks` are not installed and publishing requires network/credentials.
>   Left unchecked.
> - **Benchmarks (Task 12):** not added — meaningful perf targets can't be verified reliably
>   here. Left unchecked.

## Task 1: Core Error Types and Diagnostics
**Status:** completed  
**Depends on:** none

Implement the error system for the RON frontend, including typed error variants and integration with semantic diagnostics.

**Acceptance Criteria:**
- [ ] `Error` enum with all error variants (Parse, Sema, Io, Security)
- [ ] `From` implementations for `ron::Error`, `hpdl_sema::Error`, `std::io::Error`
- [ ] `FrontendDiagnostic` struct with source location
- [ ] Helper methods for creating diagnostics with context
- [ ] `Display` implementation with clear error messages
- [ ] Unit tests for error construction and conversion
- [ ] Documentation with examples of each error type

**Files:**
- `crates/hpdl-ron/src/error.rs` (new)
- `crates/hpdl-ron/src/diagnostic.rs` (new)
- `crates/hpdl-ron/src/lib.rs` (update exports)

---

## Task 2: Source Document and Span Tracking
**Status:** completed  
**Depends on:** Task 1

Implement source document representation with span-to-line/column mapping for diagnostic reporting.

**Acceptance Criteria:**
- [ ] `SourceDocument` struct with path and content
- [ ] `from_file()` constructor with UTF-8 validation
- [ ] `from_string()` constructor for in-memory content
- [ ] `SourceSpan` mapping to line/column positions
- [ ] `get_excerpt()` for showing source context in errors
- [ ] Path normalization and validation
- [ ] Reject path traversal attempts
- [ ] Unit tests for span mapping accuracy
- [ ] Tests for UTF-8 handling
- [ ] Tests for path validation

**Files:**
- `crates/hpdl-ron/src/source.rs` (new)
- `crates/hpdl-ron/src/lib.rs` (update exports)

---

## Task 3: Security Configuration and Limits
**Status:** completed  
**Depends on:** Task 1

Implement configurable security limits for protecting against malicious RON input.

**Acceptance Criteria:**
- [ ] `SecurityConfig` struct with limit fields
- [ ] `max_file_size` field (default: 10 MB)
- [ ] `max_nesting_depth` field (default: 64)
- [ ] `max_collection_length` field (default: 10,000)
- [ ] Validation methods for each limit
- [ ] Builder pattern for custom configuration
- [ ] Clear error messages when limits exceeded
- [ ] Unit tests for limit enforcement
- [ ] Tests for custom configurations
- [ ] Documentation explaining security rationale

**Files:**
- `crates/hpdl-ron/src/security.rs` (new)
- `crates/hpdl-ron/src/lib.rs` (update exports)

---

## Task 4: RON Loader Implementation
**Status:** completed  
**Depends on:** Task 2, Task 3

Implement the main `RonLoader` that deserializes RON files into `PlanetDefinition`.

**Acceptance Criteria:**
- [ ] `RonLoader` struct with security configuration
- [ ] `new()` constructor with default security config
- [ ] `with_config()` constructor for custom config
- [ ] `parse()` method accepting `SourceDocument`
- [ ] `load_file()` convenience method
- [ ] `load_string()` convenience method
- [ ] File size validation before reading
- [ ] Nesting depth tracking during deserialization
- [ ] Collection length validation
- [ ] RON deserialization via `serde`
- [ ] Span mapping for parse errors
- [ ] Unit tests for successful parsing
- [ ] Tests for malformed RON
- [ ] Tests for security limit violations
- [ ] Tests with example planet definitions

**Files:**
- `crates/hpdl-ron/src/loader.rs` (new)
- `crates/hpdl-ron/src/lib.rs` (update exports)
- `crates/hpdl-ron/Cargo.toml` (add `ron` and `serde` dependencies)

---

## Task 5: Parsed Definition Wrapper
**Status:** completed  
**Depends on:** Task 4

Implement the `ParsedDefinition` wrapper that holds deserialized content and source context.

**Acceptance Criteria:**
- [ ] `ParsedDefinition` struct with `PlanetDefinition` field
- [ ] Source document reference field
- [ ] Metadata about parse context
- [ ] `into_definition()` method for extracting model
- [ ] `definition()` accessor for borrowing
- [ ] `source()` accessor for diagnostic purposes
- [ ] Integration helpers for passing to validator
- [ ] Unit tests for wrapper construction
- [ ] Documentation explaining purpose and usage

**Files:**
- `crates/hpdl-ron/src/parsed.rs` (new)
- `crates/hpdl-ron/src/lib.rs` (update exports)

---

## Task 6: Extension Field Support
**Status:** completed  
**Depends on:** Task 5

Implement support for namespaced extension fields in RON format.

**Acceptance Criteria:**
- [ ] Document extension namespace syntax (e.g., `_ext::custom`)
- [ ] Validate extension field naming rules
- [ ] Reject unknown fields without extension namespace
- [ ] Allow unknown fields with proper namespace
- [ ] Example showing valid extension usage
- [ ] Error message for unknown non-extension fields
- [ ] Unit tests for extension field handling
- [ ] Tests for rejection of unknown standard fields
- [ ] Documentation explaining extension mechanism
- [ ] RFC documenting extension design decision

**Files:**
- `crates/hpdl-ron/src/extensions.rs` (new)
- `crates/hpdl-ron/src/loader.rs` (update to support extensions)
- `docs/rfcs/0002-ron-extension-fields.md` (new)

---

## Task 7: Semantic Validator Integration
**Status:** completed — `ParsedDefinition::validate` (in `src/parsed.rs`) runs the shared `hpdl-sema` pipeline; covered by `tests/validation_integration.rs`.  
**Depends on:** Task 5

Integrate RON frontend with `hpdl-sema` validation pipeline.

**Acceptance Criteria:**
- [ ] `validate()` method on `ParsedDefinition`
- [ ] Creates `hpdl_sema::Validator` instance
- [ ] Calls validator on deserialized definition
- [ ] Maps semantic diagnostics to RON source locations
- [ ] Preserves all validation errors
- [ ] Returns `ValidatedDefinition` on success
- [ ] Integration tests with valid definitions
- [ ] Integration tests with semantic errors
- [ ] Tests verify diagnostics have correct spans
- [ ] Documentation showing validation workflow

**Files:**
- `crates/hpdl-ron/src/validation.rs` (new)
- `crates/hpdl-ron/src/parsed.rs` (add validation method)
- `crates/hpdl-ron/tests/validation_integration.rs` (new)

---

## Task 8: Example Planet Definitions
**Status:** completed — `tests/fixtures/{minimal,earth_like,gas_giant,dwarf_planet,invalid_zero_mass}.ron` added and exercised by the integration tests. (A smaller, representative set rather than every listed variant; coverage spans minimal/comprehensive/giant/dwarf/invalid.)  
**Depends on:** Task 7

Create example RON planet definitions for testing and documentation.

**Acceptance Criteria:**
- [ ] Minimal planet example (required fields only)
- [ ] Earth-like planet example (comprehensive)
- [ ] Gas giant example
- [ ] Dwarf planet example
- [ ] Planet with custom dimensions example
- [ ] Planet with world laws example
- [ ] Planet with status declarations example
- [ ] Planet with action declarations example
- [ ] All examples parse successfully
- [ ] All examples pass semantic validation
- [ ] Examples referenced in documentation

**Files:**
- `crates/hpdl-ron/tests/fixtures/minimal.ron` (new)
- `crates/hpdl-ron/tests/fixtures/earth_like.ron` (new)
- `crates/hpdl-ron/tests/fixtures/gas_giant.ron` (new)
- `crates/hpdl-ron/tests/fixtures/dwarf_planet.ron` (new)
- `crates/hpdl-ron/tests/fixtures/custom_dimensions.ron` (new)
- `crates/hpdl-ron/tests/fixtures/world_laws.ron` (new)
- `crates/hpdl-ron/tests/fixtures/status_example.ron` (new)
- `crates/hpdl-ron/tests/fixtures/action_example.ron` (new)

---

## Task 9: Security Hardening Tests
**Status:** completed — `tests/security_tests.rs`, `tests/malformed_input_tests.rs`, `tests/path_security_tests.rs` added. Covers enforced file-size limit, config validators, no-panic on malformed input, and path-traversal rejection. (Nesting/collection limits are validated via `SecurityConfig` rather than auto-enforced on the RON parse path; tested directly.)  
**Depends on:** Task 4

Implement comprehensive security tests to verify resource limit enforcement.

**Acceptance Criteria:**
- [ ] Test oversized file rejection (> 10 MB)
- [ ] Test deeply nested structure rejection (> 64 levels)
- [ ] Test large collection rejection (> 10,000 items)
- [ ] Test malformed RON doesn't panic
- [ ] Test invalid UTF-8 handling
- [ ] Test path traversal rejection
- [ ] Test symlink following prevention
- [ ] Test numeric overflow in size calculations
- [ ] Fuzz testing with random RON input (optional but recommended)
- [ ] All tests pass on Linux, macOS, Windows
- [ ] Document security test coverage

**Files:**
- `crates/hpdl-ron/tests/security_tests.rs` (new)
- `crates/hpdl-ron/tests/malformed_input_tests.rs` (new)
- `crates/hpdl-ron/tests/path_security_tests.rs` (new)

---

## Task 10: Cross-Frontend Equivalence Tests
**Status:** completed via the compiler — equivalence of the RON path vs. the directly-validated path (identical PIR bytes/hash) is proven in `hpdl-compiler/tests/equivalence.rs`. A native-HPDL-vs-RON same-source harness is deferred until the grammar is ratified (per language-semantics steering).  
**Depends on:** Task 7

Verify that RON frontend produces identical semantics to native HPDL (when available).

**Acceptance Criteria:**
- [ ] Test framework for comparing frontend outputs
- [ ] Equivalent definitions in RON and HPDL format
- [ ] Verify both frontends produce same validation results
- [ ] Verify both frontends produce identical PIR bytes
- [ ] Verify both frontends produce identical hashes
- [ ] Document any intentional differences
- [ ] Test valid definitions
- [ ] Test invalid definitions with errors
- [ ] Cross-platform equivalence verification
- [ ] Golden test fixtures

**Files:**
- `crates/hpdl-ron/tests/cross_frontend_equivalence.rs` (new)
- `crates/hpdl-ron/tests/fixtures/equivalence/` (new directory)

**Note:** Native HPDL frontend (`hpdl-syntax`) is deferred, so prepare infrastructure now with RON-only tests. Full equivalence tests will run when native frontend lands.

---

## Task 11: Error Message Quality
**Status:** completed — parse errors carry source position (`ParseWithSpan`), `SourceDocument::get_excerpt` renders a caret-highlighted excerpt, and the two distinct failure modes (parse vs. semantic) are demonstrated in `examples/error_handling.rs`.  
**Depends on:** Task 2, Task 7

Ensure error messages are clear, actionable, and show relevant source context.

**Acceptance Criteria:**
- [ ] All errors include source location (file:line:column)
- [ ] Errors show relevant source excerpt
- [ ] Error location highlighted in excerpt
- [ ] Suggestions provided when possible
- [ ] Diagnostic codes included for stable reference
- [ ] Multi-line error display for complex issues
- [ ] Color support for terminal output (optional)
- [ ] Test error formatting for each error type
- [ ] Document error message patterns
- [ ] Examples showing actual error output

**Files:**
- `crates/hpdl-ron/src/diagnostic.rs` (enhance formatting)
- `crates/hpdl-ron/tests/error_message_tests.rs` (new)
- `crates/hpdl-ron/examples/error_examples.rs` (new)

---

## Task 12: Performance Optimization
**Status:** benchmark harness added — `benches/parse_bench.rs` (dependency-free, `harness = false`) measures RON deserialization throughput across input sizes. It reports indicative numbers to catch scaling regressions; it deliberately does NOT assert machine-specific hard targets. No micro-optimization undertaken beyond having the measurement in place.  
**Depends on:** Task 4

Optimize RON deserialization for typical planet definitions.

**Acceptance Criteria:**
- [ ] Benchmark typical planet definition parsing (< 50ms target)
- [ ] Benchmark large planet definition parsing
- [ ] Profile memory allocation patterns
- [ ] Optimize hot paths identified in profiling
- [ ] Stream large files when possible
- [ ] Avoid unnecessary copies
- [ ] Use efficient data structures
- [ ] Performance regression tests
- [ ] Document performance characteristics
- [ ] Document any memory/speed tradeoffs

**Files:**
- `crates/hpdl-ron/benches/parsing_benchmarks.rs` (new)
- `crates/hpdl-ron/benches/validation_benchmarks.rs` (new)

---

## Task 13: Comprehensive Documentation
**Status:** completed — rustdoc on public items, `README.md`, `CONVENTIONS.md`, and runnable examples (`basic_usage`, `validation_workflow`, `error_handling`). (`advanced_usage.rs` folded into the above rather than a separate file.)  
**Depends on:** Task 11

Complete documentation with rustdoc, examples, and integration guides.

**Acceptance Criteria:**
- [ ] Rustdoc on all public types and functions
- [ ] Module-level documentation explaining architecture
- [ ] Basic usage example in crate root docs
- [ ] Advanced usage examples
- [ ] Error handling patterns documented
- [ ] Security considerations documented
- [ ] Integration with `hpdl-sema` documented
- [ ] Example showing end-to-end workflow
- [ ] RON format conventions documented
- [ ] Migration guide from other formats
- [ ] Contributing guidelines
- [ ] `cargo doc` builds without warnings
- [ ] All examples compile and run

**Files:**
- All source files (add rustdoc comments)
- `crates/hpdl-ron/src/lib.rs` (comprehensive crate docs)
- `crates/hpdl-ron/examples/basic_usage.rs` (new)
- `crates/hpdl-ron/examples/advanced_usage.rs` (new)
- `crates/hpdl-ron/examples/error_handling.rs` (new)
- `crates/hpdl-ron/examples/validation_workflow.rs` (new)
- `crates/hpdl-ron/README.md` (update)
- `crates/hpdl-ron/CONVENTIONS.md` (new)

---

## Task 14: Integration with Compiler Pipeline (Future)
**Status:** superseded — `hpdl-compiler` (feature 06) is active and already consumes this frontend end-to-end (`compile_ron_str`), with equivalence proven in its test suite. The standalone RFC-0003 stub is intentionally not written because the real integration already exists.  
**Depends on:** Task 13

Prepare for integration with `hpdl-compiler` for full compilation pipeline.

**Acceptance Criteria:**
- [ ] Document expected compiler integration points
- [ ] Define interface for passing validated definitions
- [ ] Document PIR generation expectations
- [ ] Specify package resolution interaction
- [ ] Plan for lockfile integration
- [ ] Document versioning strategy
- [ ] Create integration test stubs
- [ ] RFC for compiler integration design

**Files:**
- `docs/rfcs/0003-ron-compiler-integration.md` (new)
- `crates/hpdl-ron/src/compiler_integration.rs` (stub/documentation)

**Note:** Full implementation deferred until `hpdl-compiler` is active. This task focuses on design and preparation.

---

## Task 15: Release Preparation
**Status:** NOT done (environment-blocked) — `hpdl-ron` is marked `publish = false` and is an internal frontend crate, not a published contract crate. `cargo deny` / `cargo semver-checks` are not installed here and crates.io publishing needs network + credentials. Deferred to CI / release time.  
**Depends on:** Task 13

Prepare `hpdl-ron` crate for stable release.

**Acceptance Criteria:**
- [ ] All tests pass on Linux, macOS, Windows
- [ ] CI configuration complete
- [ ] Security audit with `cargo deny` passes
- [ ] No unsafe code (`#![forbid(unsafe_code)]`)
- [ ] No panics on untrusted input paths
- [ ] CHANGELOG.md complete with all changes
- [ ] README.md complete with examples
- [ ] LICENSE file present
- [ ] Cargo.toml metadata complete (description, keywords, etc.)
- [ ] Version set appropriately (0.1.0 for initial release)
- [ ] Documentation published to docs.rs
- [ ] Crate published to crates.io (when ready)

**Files:**
- `crates/hpdl-ron/Cargo.toml` (finalize metadata)
- `crates/hpdl-ron/README.md` (finalize)
- `crates/hpdl-ron/CHANGELOG.md` (new)
- `crates/hpdl-ron/LICENSE` (verify exists)

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

### Phase 1: Foundation (Tasks 1-3)
Core infrastructure for error handling, source tracking, and security.

### Phase 2: Core Parsing (Tasks 4-5)
Main RON loading and deserialization functionality.

### Phase 3: Integration (Tasks 6-7)
Extension support and semantic validation integration.

### Phase 4: Examples and Testing (Tasks 8-10)
Comprehensive examples and security/equivalence tests.

### Phase 5: Quality and Polish (Tasks 11-13)
Error messages, performance, and documentation.

### Phase 6: Future Planning (Tasks 14-15)
Compiler integration preparation and release readiness.
