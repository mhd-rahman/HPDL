# Conformance & Security — Design

## 1. Scope & stance

A consolidation/gap-closing feature. The conformance/security surface is largely built
(CI matrix, digest parity, cargo-deny/audit, semver gate, MSRV, SECURITY.md, PIR fuzz,
malformed-input tests). This design specifies only the **deltas** that close the
determinism/testing-ci steering gaps, and references the existing surfaces rather than
restating them. No new public Rust types; the design-doc "interfaces" stay conceptual
(OD-16.1).

## 2. Delta map (concept → where it lands)

| Concept | Exists today | Delta this feature adds |
|---|---|---|
| GoldenVectors | `hpdl-pir` self-computed hash; compiler/model/sema frozen fixtures | **Frozen exact PIR digest vector** (REQ-01) |
| CrossPlatformSuite | 3-OS PIR digest parity | **Broader fixtures + replay binding hashes in parity** (REQ-05) |
| ReplayVectors | mock-executor contract tests | **Frozen contract-level binding-hash vectors** (REQ-02) |
| FuzzCorpus | `decode_pir` target, 1 seed | **`decode_ron` target + seeds** (REQ-03); syntax deferred |
| DependencyAudit | `deny.toml`, `cargo-audit`, dependabot | referenced; no change |
| SemverGate | `semver` job on both contract crates | referenced; no change |
| (new) PropertySuite | none | **`proptest` decode/round-trip/determinism** (REQ-04) |
| SecuritySurface | `SECURITY.md`, per-crate `rust-version` | **reconcile scope + MSRV consistency** (REQ-06) |
| Cross-quantum | representability guard only | **surfaced, deferred to OD-06.2** (REQ-07) |

## 3. REQ-01 — Frozen PIR digest vectors

`crates/hpdl-pir/tests/golden/pir_vectors.json` (committed): an ordered list of records
`{ name, language_version, body_id, pir_version, content_hash_hex }` for a fixed fixture
set (minimal, id-variant, and a richer body). `golden_hash.rs` gains a test that, for each
record, reconstructs the definition, encodes it, and asserts `envelope.version` and
`envelope.content_hash.to_hex()` **equal** the frozen values. Header documents the
no-silent-regeneration rule (ties to ADR 0003 / PIR format version). Existing structural
tests stay. OD-16.5: freeze the digest for certain; freezing raw payload bytes is optional
and, if chosen, stored as a `.bin` fixture compared with `assert_eq!`.

> First landing: the frozen digests are captured once from a verified run on the pinned
> toolchain, reviewed, and committed. They are **not** auto-updated by the test.

## 4. REQ-02 — Contract-level replay vectors

`crates/hpdl-runtime-api/tests/replay_vectors.rs` + a frozen
`tests/golden/replay_vectors.json`: fixed inputs for `EventEnvelope`, `ProcessReceipt`,
and `MigrationReceipt` constructors, each with its expected **binding hash** (hex) and
`order_key`. The test builds each contract from the fixed inputs and asserts the binding
equals the frozen value and `is_consistent()` holds, and that a known ordering input sorts
to the frozen key order. Clearly labelled contract-level (the real engine is external).
Reuses the existing mock state-hash helper pattern for deterministic `StateHash` inputs.

## 5. REQ-03 — RON frontend fuzz target

`fuzz/fuzz_targets/decode_ron.rs`: `fuzz_target!(|data: &[u8]| { if let Ok(s) =
std::str::from_utf8(data) { let _ = <hpdl-ron loader>.parse_str("fuzz", s); } })` — totality
only (Ok/Err both fine; panic/hang is the bug). Add `[[bin]] name = "decode_ron"` to
`fuzz/Cargo.toml` with `hpdl-ron` as a fuzz dependency. Add `fuzz/seeds/valid.ron` (reuse
the minimal RON fixture). `fuzz.yml` runs both targets (loop over a target list, or a
second step). `hpdl-syntax` fuzzing deferred (OD-16.3), noted in `fuzz/README.md`.

> The exact `hpdl-ron` entry point is confirmed against the crate's public API when the
> target is written (loader + `SourceDocument`); the target calls the same path the
> compiler frontend uses.

## 6. REQ-04 — Property-based suite

`crates/hpdl-pir/tests/property.rs`, `proptest` as a dev-dependency of `hpdl-pir`
(OD-16.2). Properties:
- **decode-never-panics:** `proptest!(|(data: Vec<u8>)| { let _ = decode_planet_definition(&data); })`
  (bounded length via a `proptest` strategy, e.g. `0..=4096` bytes) — asserts no panic.
- **round-trip:** generate a `PlanetDefinition` from a strategy over valid field values
  (bounded ids/traits/numeric ranges), encode, decode, assert equal.
- **deterministic-encode:** encode the generated definition twice, assert identical bytes
  and identical content hash.

Runs on the pinned **stable** toolchain inside `cargo test` on all three OSes — no nightly,
complementing the nightly fuzz job. A fixed `ProptestConfig` (cases count, RNG seed via
`PROPTEST_` env default) keeps runs reproducible. No property test writes a fixture or
reaches a canonical byte path with randomness — generation feeds *inputs*, not canonical
output.

## 7. REQ-05 — Broaden digest parity

Extend `hpdl-compiler/examples/emit_digests.rs` with additional fixed fixtures
(environment-bearing, status-bearing, orbital, and a world-law-referencing definition),
emitted in a fixed, documented order as `name=<hash>`. Append the REQ-02 contract binding
hashes (a stable, hard-coded set reconstructed from the same frozen inputs) so the parity
job compares **replay/binding hashes across OSes**, not only PIR content hashes. The
`digest-parity-emit`/`-compare` jobs are unchanged structurally — they already diff the
emitted file across the matrix.

## 8. REQ-06 — Security surface reconciliation

- `SECURITY.md`: expand the in-scope list to every active crate (`hpdl-model`, `hpdl-sema`,
  `hpdl-ron`, `hpdl-syntax`, `hpdl-pir`, `hpdl-compiler`, `hpdl-cli`, `hpdl-runtime-api`),
  explicitly naming untrusted-input surfaces (`.hpdl`, `.ron`, PIR blobs, lockfiles,
  migration/package payloads).
- MSRV consistency: confirm every crate's `rust-version = "1.85.0"`. Optionally a tiny CI
  step greps each `Cargo.toml` for the expected value; otherwise a documented manual check
  in the spec's DoD. (ADR 0001 already ratified the value; this just prevents drift.)

## 9. REQ-07 — Cross-quantum conformance (deferred, surfaced)

No tests are written that pretend to exercise mixed-quantum arithmetic, because the
contract (`ties_to_even`, per-field quantum, overflow policy) is unresolved (OD-06.2). An
ADR (0010) records: the determinism steering's "cross-quantum arithmetic" conformance case
is **blocked** on the fixed-point lowering design; it will be added when OD-06.2 is
resolved. This is explicit non-implementation, not a silent skip and not a vacuous test.

## 10. Determinism, security, forbidden list

- Frozen fixtures only; no auto-regeneration; a digest change is a reviewed, deliberate
  event tied to a format-version decision.
- No floats, wall-clock, locale, filesystem order, or unseeded randomness in any canonical
  path or fixture; property-test generation is bounded and seeded.
- `proptest` and fuzz deps are dev/fuzz-only; contract crates keep minimal deps; the fuzz
  crate stays workspace-excluded (nightly, gnu target).
- **Not** added: a real replay engine, a resolution of OD-06.2, native-parser fuzzing, any
  PIR change, any new public Rust type.

## 11. Compatibility & CI

- New tests join the existing 3-OS `check` matrix; the new fuzz target joins `fuzz.yml`;
  parity additions ride the existing `digest-parity` jobs. No new workflow is required
  (optionally a one-line MSRV-consistency step).
- No contract-crate API change ⇒ `semver` and `docs` jobs unaffected.
- ADR 0010 records OD-16.1…OD-16.5 and the OD-06.2 deferral; CHANGELOG updated.

## 12. Tests (what proves this feature done)

- `hpdl-pir`: frozen-digest vector test (REQ-01); property suite (REQ-04); existing
  malformed/structural tests retained.
- `hpdl-runtime-api`: contract replay-vector test (REQ-02).
- `fuzz`: `decode_ron` builds and runs a short smoke campaign (REQ-03).
- `hpdl-compiler`: extended `emit_digests` (REQ-05); CI parity green across 3 OSes.
- Docs/config: reconciled `SECURITY.md`, MSRV consistency (REQ-06); ADR 0010 (REQ-07).
