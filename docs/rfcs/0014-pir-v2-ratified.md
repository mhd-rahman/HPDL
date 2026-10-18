# ADR 0014: PIR Format v2.0 Ratified (locked / released)

**Status:** Accepted
**Date:** 2026-10-19
**Author:** HPDL Team
**Feature:** `00-workspace-foundation` (production-readiness)
**Builds on:** ADR 0003 (introduced PIR v2.0), ADR 0013 (release & support policy)

## Summary

Declares PIR format **v2.0 final** — no longer "unreleased." The format
version, its hash domain separator (`HPDL-PIR-v2`), and the frozen golden
vectors are now a locked contract. From this point the ADR 0003 "fold-in while
unreleased" allowance is closed: any further change to the canonical byte layout
requires a new major format version (v3.0), its own ADR, a migration path, and a
reviewed golden-vector update, per ADR 0013.

This ADR changes **no code and no bytes.** `PirVersion::CURRENT` stays `{ major:
2, minor: 0 }`, the domain separator stays `HPDL-PIR-v2`, and the `hpdl-pir`
crate stays `0.2.0`. It freezes what already exists and records the v1→v2
compatibility story as a ratified, tested break.

## Context

PIR v2.0 has been the current format since ADR 0003 (feature 07). It
subsequently absorbed feature 08's `environment` field under the ADR 0003
amendment (OD-08.1 option A), which was explicitly permitted **only because v2.0
was still unreleased**. That amendment also stated the governing rule: once v2.0
is released or declared locked, the fold-in door shuts and the next byte change
is a v3.0 major bump.

Everything v2.0 needs is in place and stable:

- The format is additive-complete for v1-scope domains: v2.0 = v1 + orbital
  fields (`identity`, `parent`, `orbit`, `rotation`) + `environment`. Later
  features (world laws 11, extended status 09) deliberately do **not** lower into
  PIR (`#[serde(skip)]` / not encoded), so they are not part of the v2.0 byte
  layout and do not pressure a bump.
- The golden vectors are frozen and cross-platform digest-parity is enforced in
  CI (`emit_digests.rs` vs `digest_parity.txt`).
- The decoder's version gate and the v1-rejection behavior are implemented and
  tested.

Leaving v2.0 perpetually "unreleased" keeps the fold-in allowance open
indefinitely, which is the opposite of a stable published format. Ratifying it
is what makes the determinism guarantee meaningful for downstream consumers (the
external simulation engine, stored artifacts, replay).

## Decision

1. **PIR v2.0 is final / locked.** `PirVersion::CURRENT = { major: 2, minor: 0 }`
   is the ratified current format. The hash domain separator `HPDL-PIR-v2`
   (bound to the major version) is frozen.

2. **Freeze the golden vectors as the v2.0 contract.** The following are frozen
   and may change only under a deliberate format-version decision (ADR 0013
   procedure), never a silent regeneration:
   - `crates/hpdl-pir/tests/golden/pir_vectors.json` — `pir_version 2.0`;
     `minimal = 608a90a7f19de7551bcf2d72bc44e1756739434f88ae1c7a6c61d24008d9ee9f`,
     `traits = 9bc40d4ecb23bed96be2f43fe2fd6acbb8df9bc275c820cddbe9a99a26c01ede`.
   - `crates/hpdl-pir/tests/golden_hash.rs` — enforced copies of the two hashes
     above (`FROZEN_MINIMAL_HASH`, `FROZEN_TRAITS_HASH`).
   - `crates/hpdl-compiler/tests/golden/minimal_build.txt` — `pir_version 2.0`,
     `content_hash = 23d96f5d90dfa9fdb8f18c21d1b5b41c08a6ef8f1f1f2e181d94a0da97f86538`.
   - `crates/hpdl-compiler/tests/golden/digest_parity.txt` — `minimal`, `traits`,
     `environment_frontends = 438303d9dd4b02e8f4b6671ced345375a2325f59536847ea0dade4b60cbc5505`,
     `locked_closure = 9ed3b771f808e67fbc51aedaf2622a5dfc9fb2d4835d36395e4c80fcfcbdaa22`.

3. **Close the fold-in allowance.** The ADR 0003 amendment rule now applies in
   its post-release form: any further canonical-byte change requires **v3.0 + a
   new ADR + a migration path + a reviewed vector update**. No more folding into
   v2.0.

4. **Confirm the v1→v2 break as intended and tested.** A v2 decoder **rejects**
   v1 bytes. This is the intended, documented break. It is enforced two ways and
   both are tested (see Compatibility impact and Test consequences).

## Compatibility impact — the v1 → v2 story

**A v2 decoder rejects v1. This is intentional and there is no migration.**

The rejection is guaranteed by two independent mechanisms, so it cannot be
bypassed by tampering with just one:

1. **Explicit version gate.** `decode_planet_definition` checks
   `PirVersion::CURRENT.is_compatible(&envelope.version)`. `is_compatible` is
   "same major, encoded minor ≤ decoder minor," so a `{1,0}` envelope under the
   `{2,0}` decoder fails on the major mismatch and returns
   `Error::IncompatibleVersion { found: 1.0, expected: 2.0 }`.
2. **Structural layout.** Even if an attacker rewrites the envelope's version
   field to `2.0`, the v1 **payload** cannot deserialize into the v2
   `PlanetDefinition`: v2 added `Option` fields and bincode writes a presence
   byte for every `Option`, so the v1 byte stream is the wrong shape. The decode
   fails closed with `Error::MalformedData`. (And because the content hash is
   computed over the payload, a mismatched/forged payload is also caught by the
   hash check before decoding.)

**No v1 → v2 migration exists, by design.** ADR 0003 records that there are no v1
production artifacts to migrate (the only v1 bytes were the test vectors, which
were updated at the v1→v2 bump). If v1 artifacts ever need reading, a dedicated
v1→v2 decoder/migration must be added under its own ADR; ratifying v2.0 does not
add one because none are in use.

**Digests differ across majors by construction.** The domain separator is bound
to the major (`HPDL-PIR-v1` → `HPDL-PIR-v2`), so "the same" planet hashes to
deliberately non-colliding digests under v1 vs v2 — a decoder can never confuse
them.

**No code or crate-version change.** `PirVersion::CURRENT` unchanged; `hpdl-pir`
stays `0.2.0`; no canonical bytes change; no SemVer effect on the contract
crates.

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| Leave v2.0 "unreleased" indefinitely | Keeps the fold-in allowance open forever, which defeats the purpose of a stable, published, hashed format. Downstream replay/stored artifacts need a locked contract. |
| Bump to v3.0 now instead of ratifying v2.0 | No pending byte change motivates a major bump; v2.0 is additive-complete for v1-scope domains. A bump with no layout change would churn every frozen digest for nothing. |
| Add a v1→v2 migration decoder as part of ratification | No v1 artifacts exist to migrate (ADR 0003). Adding an unused migration path is dead code on an untrusted-input surface; defer until a real v1 artifact needs reading. |

## Test consequences

- **New end-to-end rejection test** (`crates/hpdl-pir/tests/v1_rejection.rs`):
  constructs a real v1-shaped envelope (version `{1,0}`, payload hashed with the
  same canonical hasher) and asserts `decode_planet_definition` returns
  `Error::IncompatibleVersion`. This complements the existing `is_compatible`
  unit tests and the forced-`{99,0}` decode test by exercising the actual decode
  path against a specifically v1 version stamp, so the ratified break is backed
  by a decode attempt, not only by the version-comparison helper.
- Existing version-rejection tests remain and still pass:
  `version::test_v1_bytes_are_incompatible_with_v2_decoder`,
  `version::test_incompatibility_different_major`,
  `decoding::test_decode_rejects_incompatible_version`,
  `error::test_incompatible_version_display`.
- **Golden vectors are deliberately unchanged** — the correct outcome for a
  freeze. The frozen-vector comments are updated to say "ratified / locked (ADR
  0014)" so a future contributor sees the fold-in door is closed.
- Definition-of-done gates must stay green: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo deny check`, and
  `cargo semver-checks -p hpdl-pir -p hpdl-runtime-api` (no API change expected).

## Rationale

Ratification is the honest close to the ADR 0003 amendment: v2.0 either stays a
moving target (and the fold-in rule never ends) or it becomes a contract. The
format is complete for the v1 architectural scope that actually lowers into PIR,
its digests are frozen and CI-verified across platforms, and the v1→v2 break is
implemented and now tested end-to-end. Locking it turns "current in-development
format" into "published format," which is what deterministic replay and stored
artifacts require.
