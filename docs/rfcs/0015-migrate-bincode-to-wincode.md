# ADR 0015: Migrate PIR Serialization from bincode to wincode

**Status:** Proposed
**Date:** 2026-10-19
**Author:** HPDL Team
**Feature:** `00-workspace-foundation` (dependency maintenance)
**Builds on:** ADR 0003 (PIR v2.0), ADR 0013 (release & support policy), ADR 0014 (PIR v2.0 ratified / locked)

## Status note

This ADR is **Proposed**, not Accepted. It records the decision to pursue the
migration and the gate that must be met before it can ship. The dependency swap
is **not** done yet: this branch adds only a **byte-parity spike** (wincode as a
`dev-dependency` plus `crates/hpdl-pir/tests/wincode_parity.rs`). The shipped
`hpdl-pir` crate still serializes with **bincode 1.3.3**. The ADR moves to
Accepted only when the acceptance gate below is met in a follow-up change.

## Summary

Replace [`bincode`](https://crates.io/crates/bincode) `1.3.3` — now unmaintained
([RUSTSEC-2025-0141](https://rustsec.org/advisories/RUSTSEC-2025-0141)) — with
[`wincode`](https://crates.io/crates/wincode) `0.6.3`, a bincode-wire-compatible
serializer from Anza, as the canonical encoder/decoder in `hpdl-pir`.

The hard constraint: PIR v2.0 is a **ratified, locked, hashed** byte contract
(ADR 0014). The migration is only acceptable as a **maintenance swap that changes
zero canonical bytes** — the frozen golden-vector digests must be byte-identical
before and after. If any digest moves, this is no longer a maintenance swap; it
becomes a PIR v3.0 format decision under ADR 0013 and this ADR is withdrawn in
favour of that.

## Context

### Why move off bincode

bincode's maintainer ceased development after a harassment incident; no further
crates.io releases are expected, and RustSec has flagged `bincode 1.3.3` as
unmaintained (RUSTSEC-2025-0141). HPDL already carries a documented exception for
this advisory in `deny.toml` and the `cargo-audit` CI job (it is a maintenance,
not a security, notice). That exception is a holding position, not a resolution.

### Why wincode

- It is purpose-built as a **bincode-wire-compatible** serializer: "produces the
  same bytes as bincode for the covered shapes when using bincode's default
  configuration, provided your `SchemaWrite`/`SchemaRead` schemas and containers
  match the layout implied by your serde types."
- It is actively maintained (Anza / Agave validator), and `v0.1.2` underwent
  three independent security reviews (Neodyme, OtterSec, Asymmetric Research) and
  is fuzzed for roundtrip correctness against bincode.
- It is **not** a serde drop-in: it uses its own `SchemaWrite`/`SchemaRead`
  derive macros. This is the main source of migration work (below).

### What the spike proved

`crates/hpdl-pir/tests/wincode_parity.rs` (added on this branch, dev-dependency
only) asserts `wincode::serialize == bincode::serialize` under wincode's default
config for every byte shape the PIR payload uses:

| Shape | Result |
|---|---|
| Fixed-width integers incl. `u128` / `i128` | identical |
| `String`, `Vec<u8>` (incl. empty) | identical |
| `Option<T>` presence byte (`Some`/`None`) | identical |
| `BTreeMap` / `BTreeSet` (incl. empty length prefixes) | identical |
| Fieldless enum discriminant (default `u32`) | identical |

This rules out the primary risk — a varint/endianness/length-prefix divergence
that would change digests. It does **not** prove parity on the real, deeply
nested `hpdl-model` types; that is the acceptance gate.

## Decision

1. **Target wincode 0.6.3 as the canonical serializer in `hpdl-pir`**, keeping
   PIR format v2.0 and all frozen golden vectors unchanged. The swap ships only
   if it is byte-neutral.

2. **Acceptance gate (all must hold before Status → Accepted):**
   - The existing frozen digests are unchanged: `minimal =
     608a90a7f19de7551bcf2d72bc44e1756739434f88ae1c7a6c61d24008d9ee9f`, `traits =
     9bc40d4ecb23bed96be2f43fe2fd6acbb8df9bc275c820cddbe9a99a26c01ede`
     (`golden_hash.rs`, `pir_vectors.json`), and the compiler manifest
     `content_hash = 23d96f5d90dfa9fdb8f18c21d1b5b41c08a6ef8f1f1f2e181d94a0da97f86538`
     (`minimal_build.txt`), plus `digest_parity.txt` unchanged.
   - `PirVersion::CURRENT` stays `2.0`; domain separator stays `HPDL-PIR-v2`;
     `hpdl-pir` crate version policy per SemVer (a dependency swap with identical
     public API and identical bytes is not a breaking change).
   - All existing decode-security tests pass unchanged, **especially**
     `test_decode_rejects_inflated_payload_length_prefix` /
     `test_decode_rejects_moderately_inflated_payload_prefix` and
     `test_decode_rejects_oversized_input` — re-validated against wincode's
     allocation behavior (see Risks).
   - Full gate green: `cargo fmt --all --check`, `cargo clippy --workspace
     --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`,
     `cargo semver-checks -p hpdl-pir -p hpdl-runtime-api`, and the cross-platform
     digest-parity CI job.

3. **If any digest changes, withdraw this ADR** and open a PIR v3.0 ADR instead
   (ADR 0013 major-bump procedure: new major, migration path, re-blessed vectors,
   prior-major read-only window). A byte change is explicitly out of scope here.

## Migration scope (follow-up work, not in this branch)

1. **Derive `SchemaWrite`/`SchemaRead` on the PIR-reachable `hpdl-model` type
   graph** — ~35 serde-derived types across 16 files (`planet_definition`,
   `celestial_body`, `physical_properties`, `trait_set`, `orbital`,
   `environment`, `status_vitality`, `world_law`, `action_declaration`,
   `migration_declaration`, `package`, `registry`, `capability`, `body_id`,
   `module_ref`, `status_declaration`). Field **declaration order must be
   preserved** (bincode encodes fields in order; the wincode derive must match).
2. **Rewrite the `hpdl-pir` call sites** in `encoding.rs` / `decoding.rs` and the
   envelope (de)serialization from `bincode::serialize`/`deserialize` to
   `wincode::serialize`/`deserialize`, including the `PirEnvelope` itself
   (`version`, `content_hash`, `serde_bytes` payload).
3. **Re-validate the decode-side resource limits** against wincode's length /
   allocation semantics (wincode fails on large dynamic structures by default —
   different from bincode 1.x's incremental read that the allocation-bomb tests
   pin). Configure wincode's `SeqLen` / limits as needed to preserve the
   fail-closed behavior and the `MAX_PIR_SIZE` cap.
4. **Dependency hygiene:** wincode pulls `wincode-derive`, `darling`,
   `darling_core`, `darling_macro`, `ident_case`, `pastey`, `strsim`, `syn` into
   the closure of a **published contract crate**. Review licenses/advisories in
   `deny.toml`; confirm the contract-crate minimal-dependency principle is still
   acceptable with the new proc-macro deps (build-time only).
5. **Remove the bincode dependency and its `deny.toml` / `cargo-audit`
   RUSTSEC-2025-0141 exception** once the swap lands, and update the CHANGELOG.
6. **Architectural review:** deriving a second serialization framework on
   `hpdl-model` (the "source-independent typed domain model," per the architecture
   steering). Confirm this is acceptable or whether the wire schema should live
   adjacent to, rather than on, the model types.

## Alternatives considered

| Option | Why not (here) |
|--------|----------------|
| Stay on bincode 1.3.3 with the documented advisory ignore | Works today, but leaves the canonical encoder on an unmaintained crate indefinitely — a standing supply-chain risk for a published contract crate. Acceptable short-term, not a resolution. |
| Move to bincode 2.x | bincode 2.0 changed its API and default encoding; it is **not** byte-compatible with 1.3.3 by default, so it would change frozen digests → a v3.0 decision, not a maintenance swap. Larger blast radius than wincode for no byte-stability benefit. |
| Switch to a self-describing / tagged format (CBOR, MessagePack, protobuf) | A deliberate re-design of the canonical format; changes every digest, needs v3.0 + migration. Out of scope; this ADR is specifically a byte-neutral maintenance move. |
| Hand-roll a minimal fixint/LE encoder owned by `hpdl-pir` | Removes the external dependency entirely but reimplements a security-sensitive, determinism-critical encoder from scratch — more risk than adopting a reviewed, fuzzed, wire-compatible crate. Keep as a last resort if wincode parity fails. |

## Compatibility impact

- **Intended: none.** If the acceptance gate holds, canonical bytes are identical,
  every stored/hashed v2.0 PIR artifact remains valid, replay is unaffected, and
  no format-version change occurs.
- **Public API:** `encode_planet_definition` / `decode_planet_definition` keep
  their signatures. The only externally visible change is that
  `bincode::serialize(&envelope)` in doctests/tests becomes the wincode
  equivalent; the envelope bytes themselves are unchanged.
- **Risk-gated:** if bytes change, the migration does not ship under this ADR.

## Risks

- **Deep-nesting / field-order parity.** The spike covered leaf shapes, not the
  full nested graph. Mitigation: the frozen digests over the real `minimal` /
  `traits` / compiler vectors are the gate — they catch any structural
  divergence.
- **Decode allocation semantics.** wincode's default refusal of large dynamic
  structures differs from bincode 1.x incremental reads; the allocation-bomb
  tests must pass unchanged, possibly after configuring wincode limits.
- **Enum encoding.** Parity holds only with wincode's **default** discriminant
  encoding; the `#[wincode(tag)]` / `tag_encoding` attributes must **not** be used
  on any PIR enum, or bytes diverge.
- **Dependency surface.** New proc-macro deps (`syn`, `darling`, …) enter a
  contract crate's build graph; `cargo deny` must stay green.

## Test consequences

- This branch: adds `tests/wincode_parity.rs` (dev-dependency spike) and the
  wincode `dev-dependency` in `hpdl-pir/Cargo.toml`. No shipped code changes; all
  existing `hpdl-pir` tests and frozen digests remain green (verified).
- Follow-up migration branch: the acceptance-gate items above become the
  definition of done. Golden vectors are **not** regenerated — they are the proof
  the swap is byte-neutral.

## Rollback

Because this branch changes only a dev-dependency and adds a test, rollback is
trivial: drop the wincode `dev-dependency` and the parity test. For the follow-up
migration, rollback is reverting the `hpdl-model` derives and the `hpdl-pir` call
sites back to bincode; no persisted artifact is affected because the swap is
byte-neutral by construction.
