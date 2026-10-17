# Planetary Environments — Requirements

## Overview

This feature adds the typed declaration surface and shared semantic validation for a
celestial body's **environment**: atmosphere, thermal/climate profile, hydrosphere,
cryosphere, biosphere, and content-addressed visual asset references. It introduces the
environment domain types to `hpdl-model` and one new validation pass to `hpdl-sema`,
following the same pattern established by feature 07 (celestial & orbital systems).

New types (per `docs/design/08-planetary-environments.md`):

- `EnvironmentDefinition` — the aggregate: optional atmosphere, thermal, hydrosphere,
  cryosphere, biosphere, and a set of asset references.
- `AtmosphereProfile` — surface pressure + gas `Composition` + an optional versioned
  climate-model id.
- `Composition` — a normalized set of gas components (each a mole fraction in parts-per
  million), with an explicit normalization tolerance.
- `ThermalProfile` — mean/min/max surface temperature in milli-kelvin.
- `Hydrosphere` — declared surface-liquid coverage and dominant liquid.
- `Cryosphere` — declared ice/snow coverage.
- `Biosphere` — a tri-state presence (`Absent` / `Unknown` / `Present`) plus, when
  present, a declared complexity tier. A missing biosphere is **"not applicable", never
  "zero"**.
- `AssetRef` — a content-addressed reference to a visual asset: a logical name, a
  versioned asset kind, and a required content digest.

These are **declarations only**. Declaring an atmosphere is not a climate solver;
declaring a biosphere does not simulate an ecosystem. The external HORIZON engine alone
evolves state. This feature produces a validated, immutable model and typed diagnostics.

Primary crates: `hpdl-model` and `hpdl-sema`.

## Goals

1. **Typed, source-independent environment model**: Add the environment types to
   `hpdl-model` with validating constructors, no Earth defaults, and no floating-point
   storage.
2. **One semantic validator**: Add an `EnvironmentValidator` pass to the existing
   `hpdl_sema::Validator` as a single additional pass — never duplicated per frontend.
3. **Explicit failure on invalid declarations**: Out-of-bounds composition, zero/invalid
   pressure, inconsistent temperature ordering, missing asset digests, and unsupported
   module/climate-model ids fail with typed diagnostics using stable codes.
4. **Deterministic**: Composition is stored in a `BTreeMap`-backed ordered form; no float
   arithmetic on any new path; diagnostics sort deterministically.
5. **Safety on untrusted input**: Validate counts, name lengths, digest format, and
   numeric ranges with checked arithmetic before use; never panic.

## Scope boundaries

- **No solver.** Atmosphere/thermal/hydrosphere/biosphere declarations are static data.
  No climate integration, no chemistry, no ecosystem dynamics are performed.
- **No asset fetching.** `AssetRef` records a logical name + kind + content digest. This
  feature does **not** download, decode, or verify the bytes behind a digest — it only
  requires that a digest is present and well-formed. Fetch/verify is owned by the stdlib /
  registry feature (14).
- **No new frontend syntax.** The native `.hpdl` grammar (feature 05) and RON mapping
  (feature 04) are not extended here; this feature defines the model + validation seam so
  a frontend can populate it later without changing the validation contract.
- **The 12 atmosphere components are inputs, not weighted scores.** `Composition` records
  mole fractions of declared gases; it does not compute a habitability/pollution score.
  (Status derivation is feature 14's concern.)

## Functional Requirements

### FR1: Composition

**FR1.1**: `Composition` stores gas components as a `BTreeMap<GasId, u32>` where the value
is a mole fraction in **parts-per-million** (ppm; `fraction × 1_000_000`). `GasId` is a
validated name newtype (1–32 chars, `[A-Za-z0-9_\-.]`, e.g. `n2`, `co2`, `arcane.aether`).

**FR1.2**: The constructor validates that the sum of all component ppm values equals
`1_000_000` within an explicit, caller-supplied **normalization tolerance** (in ppm). A
sum outside `[1_000_000 - tol, 1_000_000 + tol]` is rejected with a typed error. The
tolerance is stored so the exact rule is auditable. Sum arithmetic uses checked addition
(`u64` accumulator) and never overflows silently.

**FR1.3**: An empty composition is rejected (a declared atmosphere must have at least one
gas). Duplicate gas ids are impossible by construction (`BTreeMap`), and ids are validated.

### FR2: AtmosphereProfile

**FR2.1**: `AtmosphereProfile` holds a surface pressure in **pascals** (`u64`, `> 0`), a
`Composition`, and an optional `climate_model` versioned id (a `ModuleRef`-style
name+version, or reuse `ModuleRef`).

**FR2.2**: Zero or absent pressure on a *declared* atmosphere is rejected (an atmosphere
with no pressure is meaningless; a body with no atmosphere simply omits the profile). This
is the design's "invalid negative pressure" case — pressure is unsigned, so the invalid
value is `0`.

**FR2.3**: A declared `climate_model` is a *contract reference*: it is recorded but not
executed. An unsupported/unresolvable climate model is surfaced by the validator as an
explicit non-executable declaration (see FR8.4), never silently simulated.

### FR3: ThermalProfile

**FR3.1**: `ThermalProfile` stores `mean_mk`, `min_mk`, `max_mk` as **milli-kelvin**
(`u64`; absolute temperature, so unsigned). Each is optional-at-the-field level only via
the enclosing `Option<ThermalProfile>`; when a thermal profile is declared, the three
values are provided.

**FR3.2**: The constructor validates `min_mk <= mean_mk <= max_mk`; any inversion is
rejected with a typed error. (Temperature is distinct from temperature *delta* per the
units steering; this type is absolute temperature only.)

### FR4: Hydrosphere and Cryosphere

**FR4.1**: `Hydrosphere` stores `surface_coverage_ppm: u32` (fraction of surface covered
by liquid, in ppm, `0..=1_000_000`) and an optional dominant-liquid `GasId`-style name.
Out-of-range coverage is rejected.

**FR4.2**: `Cryosphere` stores `surface_coverage_ppm: u32` (`0..=1_000_000`). Out-of-range
coverage is rejected.

**FR4.3**: Hydrosphere + cryosphere coverage are independent declarations; the validator
does not require them to sum to any particular total (surfaces overlap, and other cover
types exist). This is documented so the absence of a cross-check is intentional.

### FR5: Biosphere (absent ≠ zero)

**FR5.1**: `Biosphere` is a typed tri-state: `Absent`, `Unknown`, or
`Present(ComplexityTier)`. `ComplexityTier` is a closed enum (e.g. `Microbial`, `Simple`,
`Complex`, `Intelligent`) plus a validated `Other(name)` escape hatch.

**FR5.2**: `Absent` and `Unknown` are distinct, explicit states — neither is represented
as a zero score. The design's "absent life" test case asserts that an absent biosphere is
a first-class value, not a missing/zero field.

### FR6: AssetRef (content-addressed)

**FR6.1**: `AssetRef` holds a logical `name` (validated), a versioned `kind` (e.g.
`texture`, `mesh`, `heightmap` — closed enum + `Other(name)` escape hatch), and a required
`digest`.

**FR6.2**: `digest` is a validated content hash string: a lowercase hex SHA-256 (exactly
64 hex chars) — the format is fixed and documented. A missing or malformed digest is
rejected with a typed error. This is the design's "missing asset digest" case.

**FR6.3**: `AssetRef` does **not** fetch or verify asset bytes; it only guarantees a
well-formed, content-addressed reference. Verification of the referenced bytes is out of
scope (feature 14).

### FR7: EnvironmentDefinition and model integration

**FR7.1**: `EnvironmentDefinition` aggregates `Option<AtmosphereProfile>`,
`Option<ThermalProfile>`, `Option<Hydrosphere>`, `Option<Cryosphere>`, `Biosphere`
(tri-state, so not `Option`), and an ordered set of `AssetRef`s
(`BTreeMap<String, AssetRef>` keyed by logical name, or a sorted `Vec` with a duplicate-name
check). Absent sub-profiles mean *undeclared*.

**FR7.2**: `CelestialBody` gains an optional `environment: Option<EnvironmentDefinition>`,
added via an additive builder-style setter (`with_environment`) so existing `new(..)` call
sites and the golden roundtrip fixtures remain valid.

**FR7.3**: The new types are re-exported from `hpdl_model`'s crate root with rustdoc and
at least one example per primary type.

### FR8: Semantic validation (hpdl-sema)

**FR8.1**: A new `EnvironmentValidator` pass is added to the single `hpdl_sema::Validator`
and run exactly once per definition, merging/sorting its diagnostics with the existing
passes.

**FR8.2 (composition bounds)**: The validator defensively re-checks that composition sums
to `1_000_000` within tolerance and that every component ppm is `<= 1_000_000`; a violation
yields `DiagnosticCode::CompositionOutOfBounds`.

**FR8.3 (invalid pressure / temperature)**: Zero pressure on a declared atmosphere, or an
inverted temperature ordering, yields `DiagnosticCode::InvalidEnvironmentValue`.

**FR8.4 (unsupported climate model / module)**: A declared `climate_model` whose
name/version the validator cannot interpret as a supported model is flagged
`DiagnosticCode::UnsupportedEnvironmentModule` (Error) — accepted as a declaration, never
simulated. This is the design's "unsupported module rejection" case.

**FR8.5 (asset digest)**: A missing or malformed asset digest yields
`DiagnosticCode::MissingAssetDigest`. (Constructors already enforce this; the pass is
defensive and never panics.)

**FR8.6**: All new diagnostics use stable, documented `DiagnosticCode` values (E9000-series
"environment" block, chosen so it does not collide with the existing E1000–E8000 ranges)
and sort deterministically with existing diagnostics.

## Non-Functional Requirements

### NFR1: Determinism
- No `f32`/`f64` storage or arithmetic on any model or canonical path.
- Composition and asset collections use `BTreeMap`/`BTreeSet` or explicit sorting.
- Diagnostics sort deterministically.

### NFR2: Security / untrusted input
- Validate gas-id / asset-name / digest lengths and formats before use.
- All numeric range and sum checks use `checked_*`; no panicking indexing/`unwrap`/`expect`
  on any input path.
- Bound the number of gas components and asset refs (documented maxima) to prevent
  resource exhaustion.

### NFR3: Reliability
- `hpdl-model`: keep `#![forbid(unsafe_code)]`, `#![deny(missing_docs, unwrap_used,
  expect_used, indexing_slicing, float_arithmetic)]`.
- `hpdl-sema`: the new pass performs no float arithmetic.

### NFR4: Documentation
- Rustdoc with at least one example on every new public item.
- CHANGELOG updated with the new model surface and the new diagnostic codes.

### NFR5: Golden fixtures
- A golden fixture under `hpdl-model/tests/golden/` freezes the canonical serialized form
  of a body **with** a full environment (atmosphere, thermal, hydrosphere, cryosphere,
  biosphere, assets); it roundtrips byte-for-byte.

## Constraints

- `hpdl-model` depends only on `thiserror` (+ `serde`), remaining the dependency root.
- `hpdl-sema` depends on `hpdl-model` only.
- No new frontend syntax; no simulation; no asset fetching.

## Dependency & versioning implications (surfaced up front)

- **No new crate dependency edges.** The feature lives in `hpdl-model` and `hpdl-sema`.
- **PIR format impact — DECISION REQUIRED (OD-08.1).** Adding
  `environment: Option<EnvironmentDefinition>` to the bincode-encoded `CelestialBody`
  changes canonical PIR bytes for every definition (bincode writes a presence byte for
  every `Option`, including `None`), exactly as feature 07 did. The PIR format is currently
  at **v2.0** and that version is **still `[Unreleased]`** (introduced by feature 07, ADR
  0003, not yet published). Two options, to be chosen explicitly (not silently):
  - **(A) Fold into v2.0** — since v2.0 has not shipped, treat environment fields as part of
    the same unreleased format revision: update the feature-07 golden vectors again and
    keep `PirVersion::CURRENT = 2.0` and domain separator `HPDL-PIR-v2`. Simplest; valid
    only while v2.0 is unreleased.
  - **(B) Bump to v3.0** — treat it as a distinct breaking revision (`PirVersion::CURRENT
    = 3.0`, domain separator `HPDL-PIR-v3`, `hpdl-pir` crate `0.2.0 → 0.3.0`), with a new
    ADR, if v2.0 is considered "locked".
  This spec does not pick; it records the decision and its consequences. Whichever is
  chosen, the compiler/PIR golden vectors are updated deliberately and the change is
  recorded in `docs/rfcs/`. **No golden vector is silently regenerated.**
- **SemVer.** `hpdl-model`/`hpdl-sema` are pre-1.0; new `Option` field + new types/codes are
  additive surface. If option (B) is chosen, `hpdl-pir` takes a breaking (pre-1.0) bump.

## Open decisions / approvals required

- **OD-08.1 PIR format revision** — **RESOLVED: Option A (fold into v2.0).** Since PIR v2.0
  is still `[Unreleased]`, the `environment` field is treated as part of the same unreleased
  format revision: `PirVersion::CURRENT` stays `2.0`, the hash domain separator stays
  `HPDL-PIR-v2`, and the `hpdl-pir` crate version stays `0.2.0`. The feature-07 golden
  vectors are regenerated deliberately (reviewed, not silent) and ADR 0003 is amended to
  record that v2.0 also carries environment fields. Decision recorded in
  `docs/rfcs/0003-pir-v2-orbital-fields.md`.
- **OD-08.2 Composition precision & tolerance**: ppm (`u32`) for mole fractions with a
  caller-supplied tolerance. Confirm precision/units and a default tolerance before PIR
  stabilization.
- **OD-08.3 Digest algorithm**: SHA-256 lowercase hex (64 chars) assumed, matching the PIR
  content-hash algorithm. Confirm whether multihash/other algorithms must be supported.
- **OD-08.4 Climate-model registry**: how "supported" climate-model ids are recognized is
  deferred to the stdlib/registry feature (14); until then any declared climate model is
  treated as a non-executable declaration and flagged.
- **OD-08.5 Richer absence states**: `Biosphere` uses an explicit tri-state; other
  sub-profiles use `Option` (consistent with the existing model). Upgrading those to a full
  `Absent`/`Unknown`/`Generated` tri-state is deferred and flagged.

## Success Criteria

1. ✅ The environment types exist in `hpdl-model` with validating, non-panicking
   constructors, scaled-integer storage, no Earth defaults, and full rustdoc + examples.
2. ✅ `CelestialBody` carries an optional `environment`; existing invariants and tests still
   hold; golden roundtrip for a fully-declared environment passes byte-for-byte.
3. ✅ `hpdl_sema::Validator` runs one new environment pass producing the five required
   negative cases: composition bounds, invalid pressure, absent-life as a first-class
   value, missing asset digest, and unsupported module/climate-model.
4. ✅ No panics on malformed input; all range/sum checks use checked arithmetic; no floats.
5. ✅ CI passes: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
   warnings`, `cargo test --workspace`, `cargo deny check`.
6. ✅ OD-08.1 resolved via an ADR; any PIR golden-vector change is deliberate and reviewed.
7. ✅ CHANGELOG updated; rustdoc complete.

## Out of Scope

- Climate / chemistry / ecosystem simulation (external HORIZON engine).
- Asset byte fetching, decoding, or digest *verification* (feature 14).
- Status/score derivation from atmosphere components (feature 14).
- Native `.hpdl` or RON syntax for environments (features 05 / 04).
- CLI presentation (feature 15).

## References

- Design Doc: `docs/design/08-planetary-environments.md`
- Feature 07 (pattern + PIR v2 precedent): `.kiro/specs/07-celestial-orbital-systems/`,
  `docs/rfcs/0003-pir-v2-orbital-fields.md`
- Semantic Validation: `.kiro/specs/03-semantic-validation/`
- Product & Language steering (absent≠zero, units, no-float rules)
