# Compiler Pipeline — Tasks

## Task 1: Crate wiring, lints, error enum
**Status:** completed
**Depends on:** none

Add the `hpdl-ron` dependency edge and establish the crate's module skeleton,
lints, and the typed `Error` enum.

**Acceptance Criteria:**
- [ ] `Cargo.toml` adds `hpdl-ron` (and keeps `hpdl-sema`, `hpdl-pir`).
- [ ] `lib.rs` keeps crate-level lints (`forbid(unsafe_code)`, `deny(missing_docs,
      unwrap_used, expect_used, indexing_slicing, float_arithmetic)`).
- [ ] `Error` enum with variants: `Frontend`, `Validation`, `UnresolvedModule`,
      `ModuleVersionMismatch`, `LockParse`, `Lowering`, `Emission`, `Limit`.
- [ ] Re-export `hpdl_pir::{PirEnvelope, PirVersion, CanonicalHash}` and
      `hpdl_sema::{Diagnostic, DiagnosticCode, Severity, SourceSpan}`.
- [ ] `cargo build -p hpdl-compiler` succeeds.

**Files:**
- `crates/hpdl-compiler/Cargo.toml`
- `crates/hpdl-compiler/src/lib.rs`

---

## Task 2: CompileOptions and ResourceLimits
**Status:** completed
**Depends on:** Task 1

Define run configuration with a strict, deterministic `Default`.

**Acceptance Criteria:**
- [ ] `ResourceLimits { max_source_bytes, max_modules }` with documented defaults
      (`10 MiB`, `1024`).
- [ ] `CompileOptions { validator, lock, limits }`.
- [ ] `Default` for both; no ambient configuration read.
- [ ] Unit tests for default values.

**Files:**
- `crates/hpdl-compiler/src/options.rs` (new)
- `crates/hpdl-compiler/src/lib.rs` (module + re-export)

---

## Task 3: BuildLock parsing and resolution types
**Status:** completed
**Depends on:** Task 2

Implement the deterministic lock format reader and the resolution output types.

**Acceptance Criteria:**
- [ ] `LockedModule`, `ResolvedModule`, `ResolvedModules` types.
- [ ] `BuildLock` backed by `BTreeMap<String, LockedModule>`.
- [ ] `BuildLock::parse` handles comments/blank lines; requires exactly three
      fields; rejects wrong arity and duplicate names with `Error::LockParse { line,
      reason }`.
- [ ] `ResolvedModules` keeps modules sorted by name.
- [ ] Unit tests: valid lock, comment/blank handling, wrong arity, duplicate name.
- [ ] No panic on malformed input; no `unwrap`/`indexing`.

**Files:**
- `crates/hpdl-compiler/src/resolve.rs` (new)
- `crates/hpdl-compiler/src/lib.rs` (module + re-exports)

---

## Task 4: Module resolution stage
**Status:** completed
**Depends on:** Task 3

Resolve a definition's `ModuleRef`s against the lock, deterministically.

**Acceptance Criteria:**
- [ ] `resolve_modules(def, lock, limits) -> Result<ResolvedModules, Error>`.
- [ ] Unlocked / missing-lock reference ⇒ `Error::UnresolvedModule`.
- [ ] Version mismatch ⇒ `Error::ModuleVersionMismatch`.
- [ ] Output sorted by name; `max_modules` enforced (`Error::Limit`).
- [ ] Empty module set ⇒ empty `ResolvedModules` with or without a lock.
- [ ] Unit tests for every branch above.

**Files:**
- `crates/hpdl-compiler/src/resolve.rs` (update)

---

## Task 5: Frontend decode seam
**Status:** completed
**Depends on:** Task 1

Wire RON decode through `hpdl-ron` behind a frontend-neutral function.

**Acceptance Criteria:**
- [ ] `decode_ron(name, source) -> Result<PlanetDefinition, Error>` using
      `SourceDocument::from_string` + `RonLoader`.
- [ ] `hpdl_ron::Error` mapped to `Error::Frontend { source, reason }`.
- [ ] Returns a bare `PlanetDefinition` so the core is frontend-neutral.
- [ ] Unit tests: valid RON decodes; malformed RON ⇒ `Error::Frontend`.

**Files:**
- `crates/hpdl-compiler/src/frontend.rs` (new)
- `crates/hpdl-compiler/src/lib.rs` (module)

---

## Task 6: Lowering / representability guard
**Status:** completed
**Depends on:** Task 1

Verify numeric representability with checked arithmetic; no floats.

**Acceptance Criteria:**
- [ ] `check_representable(def) -> Result<(), Error>` validating physical numeric
      fields against documented storage widths with `checked_*`.
- [ ] Overflow / unrepresentable magnitude ⇒ `Error::Lowering { field, reason }`.
- [ ] Pure function; no ambient state; no float arithmetic.
- [ ] Unit tests: representable values pass; an out-of-range case fails typed.

**Files:**
- `crates/hpdl-compiler/src/lower.rs` (new)
- `crates/hpdl-compiler/src/lib.rs` (module)

---

## Task 7: Canonical emission stage
**Status:** completed
**Depends on:** Task 1

Delegate canonical encoding to `hpdl-pir` and verify the hash.

**Acceptance Criteria:**
- [ ] `emit(def) -> Result<PirEnvelope, Error>` calling
      `encode_planet_definition`.
- [ ] `hpdl_pir::Error` mapped to `Error::Emission`.
- [ ] `verify_hash()` checked; `false` ⇒ `Error::Emission`.
- [ ] Unit test: emission of a minimal definition verifies.

**Files:**
- `crates/hpdl-compiler/src/emit.rs` (new)
- `crates/hpdl-compiler/src/lib.rs` (module)

---

## Task 8: BuildManifest and deterministic serialization
**Status:** completed
**Depends on:** Task 4, Task 7

Assemble and serialize the reproducible build manifest.

**Acceptance Criteria:**
- [ ] `BuildManifest` with source name, compiler semantics version, PIR version,
      content hash hex, sorted module closure.
- [ ] `to_text()` emits stable, sorted, line-oriented output; no HashMap order.
- [ ] Unit test: two manifests from identical inputs produce identical `to_text()`.

**Files:**
- `crates/hpdl-compiler/src/manifest.rs` (new)
- `crates/hpdl-compiler/src/lib.rs` (module + re-export)

---

## Task 9: CompileResult and Compiler entry point
**Status:** completed
**Depends on:** Task 2, Task 4, Task 5, Task 6, Task 7, Task 8

Compose the stages into the public `Compiler` with `CompileResult`.

**Acceptance Criteria:**
- [ ] `CompileResult` with `envelope()`, `manifest()`, `content_hash_hex()`,
      `into_parts()`.
- [ ] `Compiler::new(options)`.
- [ ] `compile_validated(name, validated)`: resolve → lower → emit → manifest.
- [ ] `compile_ron_str(name, source)`: limit check → decode → validate →
      `compile_validated` tail.
- [ ] Validation failure ⇒ `Error::Validation`; nothing emitted.
- [ ] Rustdoc with a runnable example on `compile_ron_str`.

**Files:**
- `crates/hpdl-compiler/src/result.rs` (new)
- `crates/hpdl-compiler/src/lib.rs` (Compiler impl + re-exports)

---

## Task 10: End-to-end pipeline tests (positive + negative)
**Status:** completed
**Depends on:** Task 9

Integration tests over the full pipeline.

**Acceptance Criteria:**
- [ ] Minimal RON compiles; `verify_hash()` true; manifest well-formed.
- [ ] Oversized source ⇒ `Error::Limit`.
- [ ] Malformed RON ⇒ `Error::Frontend`.
- [ ] Sema failure ⇒ `Error::Validation`.
- [ ] Lock parse errors (arity, duplicate) ⇒ `Error::LockParse`.
- [ ] Unlocked module ⇒ `Error::UnresolvedModule`.
- [ ] Version mismatch ⇒ `Error::ModuleVersionMismatch`.
- [ ] `max_modules` exceeded ⇒ `Error::Limit`.

**Files:**
- `crates/hpdl-compiler/tests/pipeline.rs` (new)

---

## Task 11: Cross-frontend equivalence test
**Status:** completed
**Depends on:** Task 9

Prove the shared tail yields identical bytes regardless of entry path.

**Acceptance Criteria:**
- [ ] Same semantics built via `compile_validated` and `compile_ron_str`.
- [ ] Assert identical `envelope.payload` and `content_hash`.

**Files:**
- `crates/hpdl-compiler/tests/equivalence.rs` (new)

---

## Task 12: Golden build fixture (reproducibility)
**Status:** completed
**Depends on:** Task 9

Freeze canonical hash + manifest for a known planet.

**Acceptance Criteria:**
- [ ] `tests/golden/minimal_build.txt` holds the frozen manifest text and hash.
- [ ] `tests/golden_build.rs` compiles the fixed planet and asserts an exact match.
- [ ] Comment documents that updates require a deliberate, reviewed change (and a PIR
      format-version decision if PIR bytes change — out of scope here).

**Files:**
- `crates/hpdl-compiler/tests/golden_build.rs` (new)
- `crates/hpdl-compiler/tests/golden/minimal_build.txt` (new)

---

## Task 13: Docs and CHANGELOG
**Status:** completed
**Depends on:** Task 12

Finish documentation and record the change.

**Acceptance Criteria:**
- [ ] Rustdoc on every public item; module-level docs for each stage.
- [ ] `cargo doc -p hpdl-compiler` builds without warnings.
- [ ] CHANGELOG updated with the new compiler surface and the `hpdl-ron` edge.

**Files:**
- `crates/hpdl-compiler/src/*.rs`
- `CHANGELOG.md`

---

## Definition of Done (All Tasks)

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo deny check`
- [ ] No `unwrap()`/`expect()`/panic on input paths; checked arithmetic in lowering.
- [ ] No unsafe code; no floats on canonical paths.
- [ ] No unsupported feature silently claims simulation or success.

## Task Execution Order

1. Foundation: Tasks 1–2.
2. Resolution: Tasks 3–4.
3. Stages: Tasks 5–8.
4. Composition: Task 9.
5. Verification: Tasks 10–12.
6. Docs: Task 13.

