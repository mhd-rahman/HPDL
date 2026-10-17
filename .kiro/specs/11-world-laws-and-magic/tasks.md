# World Laws & Custom Reality — Tasks

> Domain 16 (world laws, magic, alternative physics), definition + validation only.
> **Implemented.** All open decisions resolved (ADR 0005): OD-11.1 open namespaced domain
> tokens; **OD-11.2 world laws kept OUT of canonical PIR — no byte change** (field is
> `#[serde(skip)]`, PIR golden hashes unchanged); OD-11.3 many bridge sources → one physical
> target; OD-11.4 all law modules non-executable now (`E6005` warning), registry deferred to
> feature 14; OD-11.5 reuse `E6002` for duplicate law refs. HPDL ships contracts, not an
> executor — no built-in or universal magic, no alternative-physics solver.

## Task 1: `LawDomain` + `LawMode` in hpdl-model
**Status:** completed — `crates/hpdl-model/src/world_law.rs`: `LawDomain` validated token
(1–64, dotted segments, reserved `physics` physical prefix; `is_physical()`/`is_custom()`),
`LawMode::{Additive, Replaces(BTreeSet<LawDomain>)}`. New `Error::InvalidLawDomain`. Exported
from the crate root with the `PHYSICAL_NAMESPACE` constant. Unit tests + doctest.

**Files:** `crates/hpdl-model/src/world_law.rs`, `error.rs`, `lib.rs`.

---

## Task 2: `MagicBridge` + `WorldLaw` in hpdl-model
**Status:** completed — `MagicBridge { sources: BTreeSet<LawDomain>, target, module }` (non-empty
sources, physical target enforced) and `WorldLaw { module, mode, domains, bridge }` with
`authoritative_domains()`, `touches_physical()`, `is_magic()`. Bounds (`MAX_WORLD_LAWS`,
`MAX_DOMAINS_PER_LAW`, `MAX_BRIDGE_SOURCES`); new errors `EmptyLawDomains`, `TooManyLawDomains`,
`EmptyBridgeSources`, `TooManyBridgeSources`, `NonPhysicalBridgeTarget`. Unit tests + doctests.

**Files:** `crates/hpdl-model/src/world_law.rs`, `error.rs`.

---

## Task 3: Attach world laws to `CelestialBody`
**Status:** completed — `world_laws: Vec<WorldLaw>` with `with_world_law`/`with_world_laws`
builders and `world_laws()` accessor. Field is `#[serde(skip)]` (OD-11.2): carried in memory
for the validator, **never serialized**, so canonical PIR bytes/hashes and existing golden
fixtures are unchanged. `CelestialBody::new` signature and uniqueness checks unchanged.

**Files:** `crates/hpdl-model/src/celestial_body.rs`.

---

## Task 4: Implement `validate_world_laws` (hpdl-sema)
**Status:** completed — stub replaced with the real pass (already wired into the single
`Validator` via `validate_modules`). Enforces: duplicate law module ref → `E6002`;
one-authoritative-law-per-exclusive-domain → `E6003 MultipleWorldLawsForDomain`; magic reaching
physics with no bridge → `E6004 MissingMagicBridge`; well-formed law with no registered impl →
`E6005 UnsupportedWorldLaw` (warning, never blocks). New `E6005` code added to `diagnostic.rs`
with a stability test. 10 unit tests covering every rule (additive, replacement, magic±bridge,
purely-arcane, duplicate, disjoint-compose).

**Files:** `crates/hpdl-sema/src/module_validator.rs`, `diagnostic.rs`.

---

## Task 5: Golden + negative tests
**Status:** completed — model roundtrip tests `world_law_body_carries_laws_in_memory` and
`world_laws_are_not_serialized_this_feature` (freezes the OD-11.2 no-serialization decision;
asserts PIR-relevant bytes unaffected). End-to-end sema `integration_tests`: `E6003` (two laws,
one domain), `E6004` (magic, no bridge), a valid bridged-magic law passing, and strict-mode
promotion of the `E6005` warning. (No JSON golden fixture: world laws are `#[serde(skip)]`, so
there is no serialized form to freeze — the no-serialization test replaces it.)

**Files:** `crates/hpdl-model/tests/roundtrip.rs`,
`crates/hpdl-sema/tests/integration_tests.rs`.

---

## Task 6: PIR encoding + format decision (OD-11.2)
**Status:** completed as **N/A (keep world laws out of PIR)** — resolved not to lower world laws
into canonical PIR this feature. The `#[serde(skip)]` field means PIR (bincode) sees no new
field: `PirVersion::CURRENT` stays `2.0`, the `HPDL-PIR-v2` separator is unchanged, and the
`hpdl-pir` `golden_hash` tests pass untouched. (`skip_serializing_if` was tried and rejected: it
desynchronizes bincode's non-self-describing encode/decode and broke two PIR decode tests;
`#[serde(skip)]` is the byte-safe, symmetric choice.) No PIR golden-vector change; no
`semver-checks` impact for `hpdl-pir`.

**Files:** none changed in `hpdl-pir` (deliberately).

---

## Task 7: Docs + DIAGNOSTIC_CODES + CHANGELOG
**Status:** completed — rustdoc with examples on every new public item; `DIAGNOSTIC_CODES.md`
E6000 block refined (E6002 note, E6003/E6004 implemented behavior) + new `E6005` row + a
behaviour note; CHANGELOG "Feature 11" entry.

**Files:** `crates/hpdl-sema/DIAGNOSTIC_CODES.md`, `CHANGELOG.md`, source rustdoc.

---

## Task 8: ADR + workflow status
**Status:** completed — `docs/rfcs/0005-world-laws-and-magic.md` records OD-11.1…11.5 (status,
rationale, alternatives, compatibility, test consequences). No workflow crate-status change:
feature 11 lives entirely in the already-active `hpdl-model`/`hpdl-sema`.

**Files:** `docs/rfcs/0005-world-laws-and-magic.md`.

---

## Deferred / out of scope (explicit, not silently skipped)

- **Execution / alternative-physics solver.** HPDL does not run world laws, apply magic, or
  evolve state. The external HORIZON engine + registered modules do.
- **Bridge arithmetic.** `MagicBridge` is a declared contract; the custom→physical lowering is
  a runtime module. Richer N-to-M bridges and full fixed-point lowering metadata are deferred
  (OD-11.3).
- **Module registry / trust roots** (OD-11.4) — deferred to feature 14; all law modules are
  flagged non-executable (`E6005`) at this stage.
- **Built-in / universal magic system** — never shipped; every law is explicitly declared.
- **Native `.hpdl` / RON surface syntax** for world laws (features 04 / 05); the field is not
  serialized this feature.
- **PIR encoding of world laws** — N/A this feature (OD-11.2); a later deliberate PIR revision
  would own any format-version decision.
- **CLI presentation** of world-law diagnostics (feature 15).

## Definition of Done

- [x] `cargo fmt --all --check` (ran `cargo fmt --all`; tree is formatted)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` (clean)
- [x] `cargo test --workspace` (all suites pass; model 101 + sema 103 unit, world-law model/sema
      unit + e2e tests, PIR 31 + golden_hash 3 unchanged, all doctests)
- [ ] `cargo deny check` — run in CI (tool not installed in dev env)
- [ ] `cargo semver-checks -p hpdl-pir -p hpdl-runtime-api` — unaffected by this feature
      (no PIR/runtime-api change); run in CI
- [x] No `unwrap`/`expect`/panic/indexing on input paths; checked, bounded construction; no floats.
- [x] One authoritative law per exclusive domain (`E6003`); magic-without-bridge (`E6004`);
      declared laws flagged non-executable, never simulated (`E6005`).
- [x] No built-in magic; no world-law execution; no bridge arithmetic.
- [x] OD-11.2 resolved (ADR 0005); PIR golden vectors unchanged (verified).
- [x] `DIAGNOSTIC_CODES.md`, CHANGELOG, ADR updated.
