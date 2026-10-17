# ADR 0005: World Laws & Custom Reality (domain 16)

**Status:** Accepted
**Date:** 2026-10-17
**Author:** HPDL Team
**Feature:** `11-world-laws-and-magic` (OD-11.1 … OD-11.5)

## Summary

Implements domain 16 (world laws, magic, alternative physics) on the definition +
validation side. Adds the typed world-law model (`WorldLaw`, `LawMode`, `LawDomain`,
`MagicBridge`) to `hpdl-model`, makes the previously-stubbed
`ModuleValidator::validate_world_laws` real in `hpdl-sema`, and gives the reserved
`E6003`/`E6004` diagnostic codes behavior (plus an additive `E6005`). HPDL ships **no
built-in or universal magic** and runs no alternative-physics solver — a declared law is a
contract, never executed.

## Context

- The product/architecture/language steering already specify the invariants: optional,
  registered, versioned, deterministic law modules; modes `additive` or explicit domain
  replacement; **one authoritative law per exclusive domain**; **magic affects physical
  state only through a declared bridge**; **no universal magic system**.
- The diagnostic surface was already reserved: `E6003 MultipleWorldLawsForDomain` and
  `E6004 MissingMagicBridge` existed in `diagnostic.rs` / `DIAGNOSTIC_CODES.md`, and
  `validate_world_laws` was an explicit stub ("Future: when world law support is added to
  the model"). This feature fills that gap rather than inventing a new vocabulary.

## Decisions

- **OD-11.1 — Domain taxonomy: open namespaced tokens.** `LawDomain` is a validated token
  (1–64 chars, dotted segments of `[A-Za-z0-9_\-]`). The reserved first-segment prefix
  `physics` marks a *physical* domain; any other prefix (e.g. `arcane.aether`) is a *custom*
  dimension. No built-in closed enum of domains — alternative physics and magic stay
  open-ended, while the prefix gives the bridge rule a concrete test.

- **OD-11.2 — World laws are NOT lowered into PIR this feature. Resolved: keep out, no byte
  change.** The `CelestialBody.world_laws` field is `#[serde(skip)]`.
  - *Why not `#[serde(skip_serializing_if = "Vec::is_empty")]`:* that was tried and it
    **breaks the format**. Canonical PIR uses bincode, which is not self-describing: a
    conditional-skip serializes nothing for an empty vec, but the bincode *decoder*
    unconditionally reads the field's length prefix and hits EOF. (This is the same
    non-self-describing property that forced the PIR 2.0 bump in feature 07.) Two PIR decode
    tests failed under that approach.
  - `#[serde(skip)]` is symmetric: encode writes nothing, decode uses `Default` (empty vec).
    Canonical PIR therefore sees **no new field**. `PirVersion::CURRENT` stays `2.0`, the
    `HPDL-PIR-v2` domain separator is unchanged, and all PIR golden vectors / hashes are
    byte-identical (verified by `hpdl-pir` `golden_hash` tests still passing untouched).
  - World laws are carried in-memory for the single semantic validator, which runs on the
    live model, not on decoded bytes. A serialized frontend surface for world laws is
    deferred to the RON / native parsers (features 04 / 05); encoding them into PIR later is
    a separate, deliberate format-version decision.

- **OD-11.3 — Bridge shape: many sources → one physical target.** `MagicBridge` maps an
  ordered `BTreeSet` of source (custom) dimensions to a single physical `target` dimension
  via one versioned bridge module. Richer N-to-M bridges are deferred. A non-physical target
  is rejected (`NonPhysicalBridgeTarget`): a bridge exists to reach physical state.

- **OD-11.4 — "Supported" vs registry: all law modules non-executable now.** There is no
  module registry yet (deferred to feature 14), so every declared law/bridge is flagged
  `E6005 UnsupportedWorldLaw` at **warning** severity — accepted as a declaration, never
  simulated. The warning does not block a valid declaration under the default config; strict
  mode promotes it like any other warning.

- **OD-11.5 — Diagnostic reuse.** A world-law module referenced by two laws within one body
  reuses `E6002 ConflictingModuleVersions`. `E6003` is reserved strictly for the
  one-exclusive-domain-claimed-by-two-authoritative-laws conflict.

## Alternatives considered

- **Encode world laws into PIR now (OD-11.2 = encode).** Rejected for this feature: it would
  require a deliberate PIR format-version decision and a reviewed golden-vector regeneration
  for a field nothing in the current pipeline consumes from the hashed payload. Mirrors
  feature 10's OD-10.2 (capabilities kept out of PIR). Can be revisited when a runtime/PIR
  consumer needs it.
- **Closed `LawDomain` enum.** Rejected (OD-11.1): a fixed list of physical domains would
  contradict the open, versioned custom-dimension model in the language-semantics steering.
- **A new world-law-specific duplicate-reference code.** Rejected (OD-11.5): `E6002` already
  means exactly "module referenced more than once."

## Compatibility impact

- **Additive, pre-1.0.** `hpdl-model` and `hpdl-sema` gain new public types / a new
  diagnostic code (`E6005`); no existing signature changes (`CelestialBody::new` unchanged).
- **No PIR format change** (OD-11.2): bytes, hashes, golden vectors, and `PirVersion::CURRENT`
  are unchanged. `cargo semver-checks -p hpdl-pir` is therefore unaffected by this feature.
- `E6003`/`E6004` moving from reserved-placeholder to implemented is not a breaking change —
  the codes and their meanings are unchanged.

## Test consequences

- `hpdl-model`: `world_law` unit tests (token validation, physical/custom classification,
  mode/bridge/law construction and bounds) + doctests; roundtrip tests asserting the laws are
  carried in-memory and that serialization deliberately omits them (freezing OD-11.2).
- `hpdl-sema`: `module_validator` unit tests for every rule (additive, replacement,
  magic±bridge, purely-arcane, duplicate, disjoint-compose) + end-to-end `integration_tests`
  through the full `Validator`, including strict-mode promotion of the `E6005` warning + the
  `E6005` code-stability test.
- **No golden PIR vector changes.**

## Rationale

This is the faithful reading of the already-written steering and the already-reserved
diagnostic codes: build the model the stub was waiting for, enforce the stated invariants in
the one validator, and keep the published format untouched. It refuses the two tempting
shortcuts — inventing a magic system, or quietly changing PIR bytes — and keeps HPDL firmly on
the declaration side of the declaration/execution line.
