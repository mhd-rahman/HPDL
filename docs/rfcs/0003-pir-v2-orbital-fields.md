# ADR 0003: PIR Format v2 — Orbital Fields on `CelestialBody`

**Status:** Accepted
**Date:** 2026-10-16
**Author:** HPDL Team
**Feature:** `07-celestial-orbital-systems`
**Supersedes format:** PIR v1.0 → PIR v2.0

## Summary

Feature 07 adds four optional orbital declaration fields to the
`hpdl_model::CelestialBody` struct (`identity`, `parent`, `orbit`, `rotation`).
Because the canonical PIR encoding serializes the model directly with **bincode**
— a non-self-describing format that writes a presence byte for every `Option`,
including `None` — adding these fields changes the canonical byte layout for
**every** planet definition, even one that declares no orbital data.

This ADR records the deliberate decision to treat that as a **breaking PIR
format change** and bump the format from **v1.0 to v2.0**, rather than silently
regenerating golden vectors (which the determinism steering forbids).

## Context

The determinism steering (`.kiro/steering`) states:

> Changing canonical bytes requires a format-version decision and a reviewed
> vector update. A crate release alone is not sufficient. Never silently
> regenerate vectors to make tests pass.

And the feature-07 requirements (OD-07.1) originally scoped this feature as
"no change to canonical PIR bytes." That scoping assumption was **wrong** given
the current encoder: there is no way to add a field to the bincode-encoded
`PlanetDefinition` without changing its bytes. bincode does not honour
`#[serde(skip_serializing_if)]` in a way that preserves round-trip, so the
`None` fields cannot be omitted from the stream.

The drift was caught by the frozen compiler golden fixture
(`hpdl-compiler/tests/golden/minimal_build.txt`), exactly as the golden-vector
discipline intends.

## Decision

1. **Bump the PIR format version to 2.0** (`PirVersion::CURRENT = { major: 2, minor: 0 }`).
   This is a **major** bump because the change is not backward compatible: v1
   bytes cannot be decoded by the v2 struct and vice versa. The existing
   `is_compatible` rule (same major, encoded minor ≤ decoder minor) therefore
   correctly rejects v1 payloads under a v2 decoder.

2. **Change the hash domain separator** from `HPDL-PIR-v1` to `HPDL-PIR-v2`.
   The determinism rule requires the domain separator to include the format
   version; binding it to the major version guarantees a v1 and a v2 encoding of
   "the same" planet produce deliberately non-colliding digests.

3. **Update the frozen golden vectors deliberately**:
   - `hpdl-compiler/tests/golden/minimal_build.txt` — new `pir_version = 2.0`
     and new `content_hash`.
   - The `hpdl-pir` golden-hash tests capture the digest dynamically and assert
     structural invariants, so they continue to hold; the determinism assertions
     (determinism across encodings, id-sensitivity) are unchanged.

4. **Bump the `hpdl-pir` crate version** `0.1.0 → 0.2.0` (pre-1.0 breaking bump).
   The crate version is independent of the format version, but a format-breaking
   change warrants a crate minor bump under 0.x semantics.

## Alternatives considered

### A. Keep PIR frozen; move orbital fields off the encoded struct
Hold the orbital declarations in a model-side type not part of the bincode
`PlanetDefinition`, deferring encoding to a later task. **Rejected for now**:
the orbital data genuinely belongs to the planet definition, and splitting it
off adds indirection and a second source of truth. We would still face the same
format bump when the fields are eventually encoded. (This was option B in the
implementation discussion.)

### B. Minor bump (v1.1) instead of major
**Rejected**: the change is not backward compatible. Treating a breaking byte
layout change as a minor version would let a v1.1 decoder accept v1.0 bytes that
no longer deserialize correctly, violating "one semantic value has exactly one
valid byte encoding" and the decoder's duty to reject incompatible input.

### C. Switch to a self-describing / field-tagged canonical encoding
A tagged encoding could allow additive fields without a format break. **Deferred**:
this is a much larger change to the canonical format and its own ADR. Not in
scope for feature 07.

## Compatibility impact

- **Breaking** for any stored v1.0 PIR artifacts: they will be rejected by the
  v2 decoder with `Error::IncompatibleVersion`. There are no committed v1
  production artifacts in this repository to migrate; the only v1 bytes were the
  frozen test vectors, which are updated here.
- **Replay**: no runtime replay artifacts exist yet (`hpdl-runtime-api` is
  deferred), so there is no state-replay migration to perform.
- **Future migration**: if v1 artifacts ever need to be read, a dedicated v1→v2
  migration/decoder must be provided; this ADR does not add one because none are
  in use.

## Test consequences

- `hpdl-pir::version` gains a test asserting `CURRENT` is 2.0 and that a v2
  decoder rejects v1-encoded payloads.
- `hpdl-compiler` golden fixture updated to `pir_version = 2.0` and the new hash.
- Full suite (`cargo test --workspace`), `clippy -D warnings`, `fmt --check`,
  `cargo deny check`, and `cargo semver-checks -p hpdl-pir` must pass.

## Rationale

This is the honest resolution: the orbital fields are a real part of the planet
definition, so the canonical bytes legitimately change. Recording it as a
format-version decision with reviewed vector updates preserves the
deterministic-replay invariant and the golden-vector discipline, instead of
masking a format change behind a quietly regenerated hash.

---

## Amendment (2026-10-16): feature 08 environment fields folded into v2.0

**Context:** Feature 08 (planetary environments) adds an optional
`environment: Option<EnvironmentDefinition>` field to the bincode-encoded
`CelestialBody`. As with the orbital fields, this changes canonical PIR bytes for
every definition (bincode writes a presence byte for every `Option`, including
`None`).

**Decision (OD-08.1, option A):** Because PIR format **v2.0 is still unreleased**,
the environment field is folded into the **same** v2.0 revision rather than bumping
to v3.0. Concretely:

- `PirVersion::CURRENT` stays `2.0`; the hash domain separator stays `HPDL-PIR-v2`;
  the `hpdl-pir` crate version stays `0.2.0`.
- The canonical bytes of v2.0 now include the environment field. Any PIR produced by
  an earlier in-development build of v2.0 (orbital-only) is **not** compatible and
  must be regenerated — acceptable because v2.0 has not shipped.
- The compiler golden vector (`hpdl-compiler/tests/golden/minimal_build.txt`) was
  regenerated deliberately: `pir_version` remains `2.0`, `content_hash` changed from
  `6e1610fe…` to `23d96f5d…`. This is a reviewed vector update, not a silent one.

**Consequence / rule going forward:** This "fold-in" is only valid while v2.0 is
unreleased. Once v2.0 is released (or otherwise declared locked), any further change
to the canonical byte layout requires a new major format version (v3.0) and a new
ADR — it may not be folded into a shipped version.

> **Update (2026-10-19, ADR 0014): v2.0 is now ratified / locked.** PIR format
> v2.0 is declared final by [ADR 0014](0014-pir-v2-ratified.md). The fold-in
> allowance above is therefore **closed**: no further change may be folded into
> v2.0. The next change to the canonical byte layout requires a v3.0 major bump,
> its own ADR, a migration path, and a reviewed golden-vector update, per
> [ADR 0013](0013-release-support-policy.md). The v1→v2 break (a v2 decoder
> rejects v1 bytes, no migration) is ratified and now covered end-to-end by
> `crates/hpdl-pir/tests/v1_rejection.rs`.
