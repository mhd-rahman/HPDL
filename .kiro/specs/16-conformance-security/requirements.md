# Conformance & Security — Requirements

> **Feature:** `16-conformance-security`
> **Design doc:** `docs/design/16-conformance-security.md`
> **Crates / surfaces:** workspace tests, `fuzz/`, `.github/workflows/`, `deny.toml`, `SECURITY.md`
> **Status:** 🚧 Consolidation / gap-closing

## Overview

This feature covers **domain-16 work: cross-platform deterministic behavior and
hostile-input defenses** — the conformance and security suite that proves HPDL's
determinism, canonical-format stability, and panic-free handling of untrusted input,
plus the supply-chain and API-stability gates.

**Critical framing — this is a consolidation feature, not greenfield.** A large part of
the conformance/security surface already exists from features 01–15 and the recent CI/CD
work: the 3-OS CI matrix, cross-platform PIR digest parity, `cargo-deny` + `cargo-audit`,
the semver gate on both contract crates, the MSRV job, `SECURITY.md`, the PIR-decoder
fuzz harness, and extensive malformed-input / panic-free tests in `hpdl-pir` and
`hpdl-ron`. This feature **references and extends** that work and closes the specific
gaps the determinism/testing-ci steering requires. It does **not** rebuild what exists.

The six "interfaces" named in the design doc — `GoldenVectors`, `CrossPlatformSuite`,
`ReplayVectors`, `FuzzCorpus`, `DependencyAudit`, `SemverGate` — are **conceptual labels**
for test/fixture/workflow surfaces, **not** Rust types. This feature does not introduce
Rust newtypes by those names (OD-16.1); it realizes each concept as tests, frozen
fixtures, a fuzz crate, and CI configuration.

## What already exists (referenced, NOT re-built)

- **CI matrix** (`.github/workflows/ci.yml`): `check` on `ubuntu/macos/windows` (fmt,
  clippy `-D warnings`, test, `cargo deny check`); `msrv` (build+test on 1.85.0);
  `digest-parity-emit` + `digest-parity-compare` (PIR content-hash parity across the 3
  OSes); `semver` (`hpdl-pir`, `hpdl-runtime-api`); `docs` (rustdoc `-D missing_docs`).
- **Supply chain**: `deny.toml` (licenses/advisories/bans/sources, one documented
  `RUSTSEC-2025-0141` exception); `.github/workflows/security-audit.yml` (`cargo-deny` +
  independent `cargo-audit`, scheduled + on-change); `.github/dependabot.yml`.
- **Fuzzing**: `fuzz/` crate with the `decode_pir` target over
  `hpdl_pir::decode_planet_definition`, one seed, nightly `fuzz.yml` workflow.
- **Golden fixtures**: `hpdl-compiler` (`minimal_build.txt`, `stdlib.lock`), `hpdl-model`
  (`*.json` roundtrip), `hpdl-sema` (`status_thresholds_*.json`), `hpdl-cli`
  (`inspect_minimal.txt`).
- **Panic-free / malformed-input**: `hpdl-pir` decoder size caps + typed errors;
  `hpdl-ron` malformed/security/path-traversal tests; `hpdl-sema` limits.
- **Determinism**: `emit_digests` example; unit determinism tests; `SECURITY.md`;
  `rust-toolchain.toml` (1.99.1) + MSRV 1.85.0.
- **Coverage**: `.github/workflows/coverage.yml` (`cargo-llvm-cov`).

## Functional Requirements (the gaps this feature closes)

### REQ-01 — Frozen exact PIR golden digest vectors

Today `hpdl-pir/tests/golden_hash.rs` **self-computes** the content hash and asserts only
structural invariants (length 64, non-zero, cross-encoding determinism). The determinism
steering requires golden vectors to store the **exact** expected digest + format version.

**Acceptance criteria:**
- A frozen golden fixture under `crates/hpdl-pir/tests/golden/` records, for a fixed set
  of definitions, the input semantics, the `PirVersion`, and the **exact 64-hex content
  hash** (and, where practical, the exact payload bytes).
- A test asserts the freshly-encoded digest **equals the frozen digest** for each vector.
- The fixture header documents that a changed digest requires a deliberate PIR
  format-version decision + ADR + reviewed vector update — never a silent regeneration.
- Differing-ID and determinism tests are retained.

### REQ-02 — Replay conformance vectors (contract level)

No frozen `action → event → state-hash` / `process → receipt` / `migration → receipt`
vectors exist; only mock-executor contract tests do. The real executor is external, so
replay conformance is assertable here only at the **contract** level.

**Acceptance criteria:**
- A frozen vector set drives the `hpdl-runtime-api` receipt/binding contracts
  (`EventEnvelope`, `ProcessReceipt`, `MigrationReceipt`) with fixed inputs and asserts
  the **exact** binding hashes and deterministic ordering keys.
- Documented explicitly as *contract-level* replay conformance — the authoritative
  state-transition replay lives in the external HORIZON engine (out of scope here).

### REQ-03 — Fuzz the frontend decoders

Only the PIR decoder is fuzzed. The RON frontend (`hpdl-ron`) parses untrusted `.ron`
input and must be equally total.

**Acceptance criteria:**
- A `decode_ron` fuzz target over the `hpdl-ron` loader entry point exists in `fuzz/`,
  asserting totality (no panic/hang/over-allocation) on arbitrary input.
- At least one seed per target under `fuzz/seeds/`; the fuzz workflow runs each target.
- The native parser (`hpdl-syntax`) fuzzing is explicitly deferred while the grammar is
  unratified (OD-16.3) — stated, not silently skipped.

### REQ-04 — Property-based conformance tests

No `proptest`/`quickcheck`/`arbitrary` usage exists. Example-based tests miss generated
edge cases.

**Acceptance criteria:**
- A property-test suite (using `proptest`) asserts, over generated inputs:
  (a) the PIR decoder never panics on arbitrary bytes; (b) encode→decode round-trips a
  generated `PlanetDefinition` to an equal value; (c) encoding is deterministic (same
  input ⇒ same bytes/hash).
- Runs inside `cargo test` on **every** platform (stable toolchain — no nightly), so it
  complements (does not replace) the nightly fuzz job.

### REQ-05 — Broaden cross-platform digest parity

The parity example covers only two tiny planets and PIR content hashes.

**Acceptance criteria:**
- `emit_digests` is extended to cover a broader, representative fixture set (environment,
  status, orbital, world-law-referencing definitions) in a fixed order.
- The parity CI job continues to diff the emitted digests across the 3 OSes and fail on
  any divergence.
- Contract-level replay binding hashes (REQ-02) are included in the parity emission so
  replay hashes are compared across OSes, not just PIR content hashes.

### REQ-06 — Reconcile the security surface

Documentation/config drift found during inventory.

**Acceptance criteria:**
- `SECURITY.md`'s in-scope crate list is reconciled to include **all** active crates
  (notably `hpdl-syntax` and `hpdl-runtime-api`).
- Every workspace crate declares a consistent `rust-version` matching the ratified MSRV
  (1.85.0, ADR 0001). A check (CI or documented) catches drift.

### REQ-07 — Cross-quantum / fixed-point arithmetic conformance (surface, do not resolve)

The determinism steering lists cross-quantum arithmetic (Pa vs kPa, m vs mm,
`ties_to_even`) as a required conformance case. `hpdl-compiler/src/lower.rs` is only a
representability guard; the per-field quantum / rounding / overflow contracts of Amendment
A2 are **unresolved** (OD-06.2).

**Acceptance criteria:**
- The feature **surfaces** OD-06.2 as the blocker: cross-quantum conformance tests cannot
  be written until the fixed-point lowering contract is decided.
- An ADR records that cross-quantum arithmetic conformance is **deferred** pending OD-06.2;
  it is **not** faked with placeholder tests that assert nothing.

## Non-Functional Requirements

- **NFR-01 Determinism:** no wall-clock, locale, filesystem order, network, unseeded
  randomness, or float arithmetic reaches any canonical byte, hash, or test fixture.
  Property tests use a fixed `proptest` seed/config for reproducibility.
- **NFR-02 No new runtime dependencies in contract crates:** `proptest` and any fuzz
  dependency are `dev`/fuzz-only; `hpdl-pir` and `hpdl-runtime-api` keep their minimal
  dependency sets. The fuzz crate stays excluded from the workspace.
- **NFR-03 Frozen-fixture discipline:** every golden/replay vector is a committed, frozen
  file; changing one requires a reviewed, deliberate update tied to a format/version
  decision — never silent regeneration (determinism steering).
- **NFR-04 Panic-free on untrusted input:** malformed PIR, RON, and generated inputs fail
  closed with typed errors; no panic/overflow/unbounded work. CI gates this.
- **NFR-05 CI gating:** new tests run in the existing `check` matrix; new fuzz targets run
  in `fuzz.yml`; parity additions run in the `digest-parity` jobs. No leg uses
  `continue-on-error` without approval.

## Open decisions / approvals required

- **OD-16.1 — Design-doc "interfaces" as Rust types? — PROPOSED: no.** `GoldenVectors`,
  `CrossPlatformSuite`, `ReplayVectors`, `FuzzCorpus`, `DependencyAudit`, `SemverGate`
  remain conceptual; the feature is realized as tests/fixtures/workflows/config, not new
  public Rust newtypes. Confirm.
- **OD-16.2 — Property-test dependency — PROPOSED: `proptest`** (stable, widely used,
  shrinking). Dev-only. Confirm the crate choice.
- **OD-16.3 — Fuzz the native parser now? — PROPOSED: defer.** `hpdl-syntax`'s grammar is
  unratified and its converter is a stub; fuzzing it now tests throwaway code. Fuzz
  `hpdl-ron` now; defer `hpdl-syntax` until the grammar is ratified.
- **OD-16.4 — Cross-quantum conformance — BLOCKED on OD-06.2.** Deferred via ADR; not
  faked.
- **OD-16.5 — Exact PIR byte vector vs digest-only.** Freeze the exact digest + format
  version for sure; freezing the full raw payload bytes is optional (larger, bincode-
  dependent). Confirm whether to freeze bytes or digest-only.

## Out of Scope

- The authoritative state-transition **replay engine** (external HORIZON engine); only
  contract-level replay vectors are in scope.
- Resolving the fixed-point lowering contract (OD-06.2) — surfaced, deferred.
- Fuzzing the native `.hpdl` parser (OD-16.3 — deferred).
- Rebuilding any already-existing CI job, `deny.toml`, `SECURITY.md`, or malformed-input
  test — those are referenced, not re-created.
- Any PIR / canonical-bytes change (a changed digest is a *failure signal* here, not a
  deliverable).

## References

- Steering: `determinism.md` (golden vectors = exact bytes + format version + digest;
  cross-platform parity; no floats; frozen fixtures), `testing-ci.md` (CI gates, suites,
  required boundary cases, MSRV, fuzz, cross-platform digest parity), `rust-standards.md`
  (untrusted input, no panics, verify hashes), `architecture.md` (contract crates).
- Builds on / references: features 02 (PIR), 06 (pipeline), 10/12/13 (runtime contracts),
  14 (lockfile), 15 (CLI), and ADR 0001 (toolchain/MSRV), ADR 0003 (PIR v2).
- Design: `docs/design/16-conformance-security.md`.
