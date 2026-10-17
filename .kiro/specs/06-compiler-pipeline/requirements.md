# Compiler Pipeline — Requirements

## Overview

The compiler pipeline (`hpdl-compiler`) is the orchestration layer that turns an
authored planet definition (currently via the RON frontend) into a canonical,
content-addressed PIR artifact plus a reproducible build manifest. It owns no
semantic rules of its own: it sequences the existing stages — frontend decode →
shared semantic validation (`hpdl-sema`) → module/package resolution → fixed-point
lowering / unit normalization → canonical PIR emission (`hpdl-pir`) — and reports
structured, typed diagnostics.

This crate sits above `hpdl-ron`, `hpdl-sema`, and `hpdl-pir` in the dependency
graph. `hpdl-pir` and `hpdl-runtime-api` must never depend on it.

## Goals

1. **Deterministic compilation**: Same locked inputs + compiler semantics version
   + PIR format version ⇒ byte-identical PIR on Linux, macOS, and Windows.
2. **Single pipeline**: One orchestrated path shared by every frontend; equivalent
   semantics yield identical PIR bytes regardless of frontend.
3. **Explicit failure**: Unsupported or unresolved declarations fail with typed
   errors; nothing silently produces plausible-looking output.
4. **Reproducible manifests**: Every build emits a manifest recording the compiler
   semantics version, PIR format version, content hash, and the resolved module
   closure.
5. **Safety on untrusted input**: Enforce limits before allocation; never panic on
   malformed source, locks, or references.

## Scope boundaries

- The compiler **does not** execute, simulate, or evolve planetary state. It emits
  immutable artifacts and runtime contracts only.
- The compiler **does not** reimplement validation. All semantic rules live in
  `hpdl-sema`.
- Native `.hpdl` parsing lives in `hpdl-syntax`; this feature wires the RON frontend
  now and leaves a frontend-neutral seam for `hpdl-syntax` to plug in later without
  changing the pipeline contract.
- Package registry / archive fetching and content-hash verification of fetched
  archives are owned by the stdlib feature (14). This feature resolves declared
  module references against a provided lockfile and fails closed on anything
  unresolved — it does not perform network fetches.

## Functional Requirements

### FR1: Compile Options and Entry Point

**FR1.1**: `CompileOptions` configures a compilation run.
- Carries the semantic validator configuration (`hpdl_sema::ValidatorConfig`).
- Carries an optional lockfile (`BuildLock`) describing allowed module versions and
  their content hashes.
- Carries resource limits (max source size, max module-closure size).
- Has a `Default` that is strict and deterministic (no ambient configuration).

**FR1.2**: `Compiler` is the public entry point.
- `Compiler::new(options)` constructs an instance.
- `compile_ron_str(name, source)` compiles RON source text to a `CompileResult`.
- `compile_validated(validated)` compiles an already-validated definition (shared
  path used by every frontend and by equivalence tests).
- All entry points return `Result<CompileResult, Error>`; none panic on input.

**FR1.3**: `CompileResult` is the output of a successful compile.
- Exposes the canonical `PirEnvelope` (version, content hash, payload).
- Exposes the `BuildManifest`.
- Provides a convenience accessor for the canonical hash hex string.

### FR2: Frontend Decode Stage

**FR2.1**: RON source is decoded through `hpdl-ron` (`RonLoader` + `SourceDocument`);
the compiler does not re-parse RON itself.

**FR2.2**: Decode failures surface as `Error::Frontend` carrying the frontend's
diagnostic text and the logical source name; the pipeline stops at the first stage
that fails.

**FR2.3**: The decode stage is frontend-neutral at the type level: the pipeline's
core operates on `PlanetDefinition` / `ValidatedDefinition`, so adding `hpdl-syntax`
later requires no change to resolution, lowering, or emission.

### FR3: Semantic Validation Stage

**FR3.1**: Every definition is routed through `hpdl_sema::Validator` exactly once.

**FR3.2**: Validation diagnostics are collected and returned as
`Error::Validation(Vec<Diagnostic>)`; a definition that fails validation is never
lowered or emitted.

**FR3.3**: The compiler never bypasses or duplicates validation, even on the
`compile_validated` path (that path accepts a `ValidatedDefinition`, which can only
be produced by the validator).

### FR4: Module / Package Resolution Stage

**FR4.1**: `BuildLock` pins, for each module name, an exact version string and a
content hash, plus the transitive closure order.
- Parsed from a deterministic, line-oriented lock format (sorted, no ambient data).
- Rejects duplicate module names and malformed lines with typed errors.

**FR4.2**: Resolution walks the declared `ModuleRef`s of the definition and matches
each against the lockfile.
- An unlocked module reference ⇒ `Error::UnresolvedModule`.
- A version mismatch between the `ModuleRef` and the lock ⇒ `Error::ModuleVersionMismatch`.
- Resolution output is a `ResolvedModules` set ordered deterministically
  (sorted by module name) for inclusion in the manifest and hash inputs.

**FR4.3**: Resolution never resolves a mutable branch/label and never performs
network or filesystem access during this feature; a missing lockfile with any
declared module reference fails closed.

**FR4.4**: A definition with no module references compiles successfully with an
empty resolved-module set, with or without a lockfile.

### FR5: Lowering / Normalization Stage

**FR5.1**: The model already stores canonical scaled integers (e.g. `mass_kg: u128`,
`surface_gravity_mms2: milli-m/s²`). The lowering stage verifies representability of
every numeric field against its PIR storage width using checked arithmetic and fails
with `Error::Lowering` on overflow or unrepresentable magnitude.

**FR5.2**: No floating-point arithmetic occurs on any canonical path
(`#![deny(clippy::float_arithmetic)]` enforced crate-wide).

**FR5.3**: Lowering is a pure function of the validated model; it introduces no
ambient nondeterminism (no clock, locale, RNG, HashMap iteration, or filesystem
order).

### FR6: Canonical PIR Emission Stage

**FR6.1**: Emission calls `hpdl_pir::encode_planet_definition` to produce the
canonical `PirEnvelope`; the compiler does not re-implement encoding or hashing.

**FR6.2**: The emitted envelope's content hash is verified (`verify_hash`) before the
result is returned; a verification failure is an internal error, surfaced typed.

**FR6.3**: Equivalent definitions (same validated model) always emit byte-identical
payloads and identical content hashes.

### FR7: Build Manifest

**FR7.1**: `BuildManifest` records, deterministically:
- compiler semantics version (crate version),
- PIR format version,
- the canonical content hash (hex),
- the ordered resolved-module closure (name, version, content hash),
- the logical source name.

**FR7.2**: The manifest serializes to a stable, sorted, line-oriented text form so it
is diff-friendly and reproducible; serialization never depends on HashMap order.

**FR7.3**: Two compiles of the same locked input produce byte-identical manifests.

### FR8: Diagnostics and Error Model

**FR8.1**: All errors are typed (`thiserror`) and non-destructive. The `Error` enum
distinguishes frontend, validation, resolution, lowering, emission, and limit
failures.

**FR8.2**: Each error carries enough context (source name, module name/version, field
name) to be actionable, with stable wording suitable for CLI presentation.

**FR8.3**: Unrecognized or unsupported executable declarations fail rather than
silently producing output. (No simulation is ever performed here.)

## Non-Functional Requirements

### NFR1: Determinism
- No wall-clock, locale, filesystem order, network, or unseeded randomness on any
  compile path.
- `BTreeMap`/`BTreeSet` or explicit sorting wherever order can reach the manifest or
  hash inputs.

### NFR2: Security / untrusted input
- Enforce `max_source_bytes` before decoding; reject oversized input with a typed
  limit error.
- Enforce `max_modules` on the resolved closure to bound work.
- No `unwrap`/`expect`/`panic`/panicking-index on any input path; checked arithmetic
  in lowering.

### NFR3: Reliability
- `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]` on public items.
- `#![deny(clippy::unwrap_used, expect_used, indexing_slicing, float_arithmetic)]`.

### NFR4: Documentation
- Rustdoc on every public item with at least one example on the primary entry points.
- CHANGELOG updated for the new public surface.

### NFR5: Cross-platform reproducibility
- Golden fixtures under `tests/golden/` freeze the canonical hash and manifest for a
  known definition; the same bytes are expected on Linux, macOS, and Windows.

## Constraints

- Depends on `hpdl-ron`, `hpdl-sema`, `hpdl-pir`, `hpdl-model` (via re-exports),
  `thiserror`.
- Must NOT be depended upon by `hpdl-pir` or `hpdl-runtime-api` (no reverse edges).
- Must not expose frontend AST or compiler-internal structures to runtime consumers.
- Must not change canonical PIR bytes; it only orchestrates the existing encoder.

## Dependency & versioning implications (surfaced up front)

- **New dependency edge**: `hpdl-compiler → hpdl-ron`. This is consistent with the
  architecture steering (`hpdl-cli → hpdl-compiler → { hpdl-ron, hpdl-syntax }`) but
  is a new edge versus the current skeleton `Cargo.toml`, which only lists
  `hpdl-sema` and `hpdl-pir`. Added in Task 1.
- **No PIR format change**: this feature does not alter canonical PIR bytes or the
  PIR format version; it consumes `hpdl-pir` as-is. No golden-vector regeneration in
  `hpdl-pir` is required or permitted here.
- **SemVer**: `hpdl-compiler` is pre-1.0 (0.1.x) and not a published contract crate,
  so its public API may evolve; additions here are new surface, not breaking changes.

## Open decisions / approvals required

- **OD-06.1 Lock format**: this spec defines a minimal, deterministic line-oriented
  lock format sufficient for resolution. The authoritative `hpdl.lock` format is
  owned by feature 14 (stdlib packages). If/when 14 lands, this reader must align or
  be replaced; recorded as an ADR rather than silently diverging.
- **OD-06.2 Numeric storage widths**: the representability checks assume the model's
  current widths (`u128`/`u64`/`i64`). Any future per-field quantum/overflow contract
  (Amendment A2) refines these checks; flagged, not resolved in code.
- **OD-06.3 MSRV**: inherits the workspace open decision (OD-02) on MSRV.

## Success Criteria

1. ✅ RON source compiles end-to-end to a verified `PirEnvelope` + `BuildManifest`.
2. ✅ The `compile_validated` path and the RON path produce identical PIR for the
   same semantics (equivalence seam proven).
3. ✅ Deterministic: repeated compiles produce byte-identical PIR and manifest; a
   golden fixture pins the hash.
4. ✅ Module resolution succeeds against a matching lock and fails closed
   (typed errors) on unresolved / mismatched / unlocked modules.
5. ✅ Lowering rejects unrepresentable numerics with typed errors; no floats.
6. ✅ No panics on malformed source / lock / references; limits enforced.
7. ✅ CI passes: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
   -- -D warnings`, `cargo test --workspace`, `cargo deny check`.
8. ✅ Rustdoc complete; CHANGELOG updated.

## Out of Scope

- Native `.hpdl` parsing (feature 05 `hpdl-syntax`).
- Package registry, archive download, archive hash verification (feature 14).
- CLI subcommand presentation (feature 15 `hpdl-cli`).
- Runtime execution / simulation (external HORIZON engine).
- Schema migration execution (feature 13; declarations only pass through).

## References

- Design Doc: `docs/design/06-compiler-pipeline.md`
- PIR: `.kiro/specs/02-canonical-pir/`
- Semantic Validation: `.kiro/specs/03-semantic-validation/`
- RON Frontend: `.kiro/specs/04-ron-frontend/`
