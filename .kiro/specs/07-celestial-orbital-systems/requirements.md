# Celestial and Orbital Systems — Requirements

## Overview

This feature adds the typed declaration surface and shared semantic validation for
**celestial identities, reference frames, and Keplerian orbit declarations** to HPDL
v1. It introduces five new domain types to `hpdl-model` and one new validation pass to
`hpdl-sema`:

- `CelestialIdentity` — the kind/role of a body (star, planet, moon, …) as a declared,
  typed classification.
- `ParentRef` — a declared gravitational-parent reference to another `BodyId`.
- `ReferenceFrame` — the named frame an orbit's elements are expressed in.
- `KeplerianElements` — the six classical orbital elements, stored as scaled integers.
- `RotationModel` — spin-axis and sidereal-rotation declaration.

These are **declarations only**. Declaring an orbit is not an orbital solver, and
declaring a rotation model does not evolve orientation. The external HORIZON engine
alone executes any state transition. This feature produces a validated, immutable model
and typed diagnostics — nothing is simulated here.

Per the design doc (`docs/design/07-celestial-orbital-systems.md`), the primary crates
are `hpdl-model` and `hpdl-sema`.

## Goals

1. **Typed, source-independent orbital model**: Add the five types to `hpdl-model` with
   validating constructors, no Earth defaults, and no floating-point storage.
2. **One semantic validator**: Add orbital/frame/parent validation to the existing
   `hpdl_sema::Validator` as a single additional pass — never duplicated per frontend.
3. **Explicit failure on invalid declarations**: Parent cycles, reference-frame
   mismatches, out-of-range eccentricity, and unit/dimension errors fail with typed
   diagnostics using stable codes.
4. **Deterministic**: All ordering that can reach output or hashes uses `BTreeMap`/
   `BTreeSet` or explicit sorting; no float arithmetic on any canonical path.
5. **Safety on untrusted input**: Validate counts, string lengths, and numeric ranges
   with checked arithmetic before use; never panic on malformed input.

## Scope boundaries

- **No solver.** An orbit declaration is static data. This feature performs no
  propagation, no ephemeris generation, and no two-body/N-body computation.
- **PIR format bump required (resolved during implementation).** Adding the four
  `Option` fields to the bincode-encoded `CelestialBody` changes canonical PIR bytes for
  every definition (bincode writes a presence byte per `Option`, even `None`). This is
  handled as a deliberate PIR format-version change (v1.0 → v2.0) with reviewed golden
  vector updates — see OD-07.1 and `docs/rfcs/0003-pir-v2-orbital-fields.md`. The orbital
  fields are serialized by the existing encoder; no new per-field scaled-integer PIR
  schema is defined here (that remains future work if a tighter encoding is wanted).
- **Single-definition parent graph.** The current model holds one `CelestialBody` per
  `PlanetDefinition`. Parent-cycle validation operates over the parent references
  declared *within a single definition* (including self-reference). Multi-file / cross
  definition parent resolution is out of scope and flagged.
- **No new frontend syntax.** The native `.hpdl` grammar (feature 05) and RON mapping
  (feature 04) are not extended here; this feature defines the model + validation seam
  so a frontend can populate it later without changing the validation contract.

## Functional Requirements

### FR1: CelestialIdentity

**FR1.1**: `CelestialIdentity` is a typed classification of a body's kind. It is a
closed enum of declared kinds (e.g. `Star`, `Planet`, `DwarfPlanet`, `Moon`,
`Asteroid`, `Comet`, `Barycenter`) plus an explicit `Other(name)` escape hatch for a
namespaced, validated custom kind string.

**FR1.2**: `Other` custom-kind names are validated (1–64 chars, `[A-Za-z0-9_\-.]`) and
rejected otherwise with a typed error. The closed variants carry no free-form data.

**FR1.3**: `CelestialIdentity` derives `Clone, Debug, PartialEq, Eq, Hash, Serialize,
Deserialize` and has a stable, deterministic serialized form.

### FR2: ParentRef

**FR2.1**: `ParentRef` wraps a target `BodyId` (the gravitational parent) and the
`ReferenceFrame` the child's orbit is expressed relative to that parent.

**FR2.2**: A `ParentRef` whose target equals the declaring body's own id is a
self-parent and MUST be rejected by validation (it is the degenerate 1-cycle).

**FR2.3**: `ParentRef` construction validates its inputs via the existing `BodyId`
constructor and the `ReferenceFrame` type; it never panics.

### FR3: ReferenceFrame

**FR3.1**: `ReferenceFrame` is a typed, closed enum of supported frames with an
explicit `Custom(name)` escape hatch (validated name, as in FR1.2). Supported frames
cover at least: `EquatorialJ2000`, `EclipticJ2000`, `BodyFixed`, and `ParentCentered`.

**FR3.2**: Orbit elements and rotation models that reference a frame MUST reference a
constructible `ReferenceFrame`. An unknown/unsupported custom frame that is *required to
be executable* is not silently accepted — see FR6.4.

### FR4: KeplerianElements

**FR4.1**: `KeplerianElements` stores the six classical elements as **scaled integers**
(no `f32`/`f64`), with documented units and scales:
- `semi_major_axis_m: u128` — metres (SI canonical).
- `eccentricity_micro: u32` — eccentricity × 1_000_000 (dimensionless).
- `inclination_mdeg: i32` — inclination in milli-degrees.
- `lon_ascending_node_mdeg: i32` — longitude of ascending node, milli-degrees.
- `arg_periapsis_mdeg: i32` — argument of periapsis, milli-degrees.
- `mean_anomaly_epoch_mdeg: i32` — mean anomaly at epoch, milli-degrees.
- `epoch` — the `ReferenceFrame` the elements are expressed in, plus the frame choice
  recorded on the `ParentRef`.

**FR4.2**: The constructor validates ranges with checked arithmetic:
- `semi_major_axis_m > 0`.
- `eccentricity_micro < 1_000_000` (strictly bound to `[0, 1)` — i.e. closed/elliptical
  orbits only in v1). Parabolic (`e = 1`) and hyperbolic (`e > 1`) are **rejected** with
  a typed error, not silently clamped. (This v1 bound is an explicit, documented
  decision; see OD-07.3.)
- Angular fields normalized/accepted in documented bounds (longitude/argument/anomaly in
  `[0, 360_000)` milli-degrees; inclination in `[0, 180_000]` milli-degrees). Out-of
  range values are **rejected**, never wrapped silently.

**FR4.3**: No field defaults to an Earth-like value. There is no implicit epoch, no
implicit frame, and no implicit zero element — every element is explicitly provided to
the constructor.

### FR5: RotationModel

**FR5.1**: `RotationModel` declares sidereal rotation period and spin-axis orientation:
- `sidereal_period_s: i64` — seconds; negative indicates retrograde (consistent with the
  existing `PhysicalProperties::rotation_period_s` convention). Zero is rejected.
- `axial_tilt_mdeg: i32` — obliquity to the orbit's frame, milli-degrees, `[0, 180_000]`.
- `frame: ReferenceFrame` — the frame the spin axis is expressed in.

**FR5.2**: `RotationModel` is optional on a body; its absence means *undeclared*, not a
default spin.

### FR6: Semantic validation (hpdl-sema)

**FR6.1**: A new `OrbitalValidator` pass is added to the single `hpdl_sema::Validator`
and run exactly once per definition, with its diagnostics merged and sorted alongside
the existing passes.

**FR6.2 (parent acyclicity)**: The validator rejects a self-parent (`ParentRef` target ==
body id) with `DiagnosticCode::ParentCycle`. Within a single-definition graph this is the
only reachable cycle; the validator is written to detect cycles generally so it extends
cleanly when multi-body graphs land.

**FR6.3 (reference-frame consistency)**: If a body declares both a `ParentRef` and
`KeplerianElements`, their declared frames MUST be consistent per the documented rule
(the orbit's frame must match the parent reference's frame, or be `ParentCentered`).
A mismatch yields `DiagnosticCode::ReferenceFrameMismatch`.

**FR6.4 (unsupported executable frame/model)**: A `Custom` reference frame (or any
declaration the validator cannot interpret) is accepted as a *non-executable
declaration* only; it is flagged with an explicit diagnostic
(`DiagnosticCode::UnsupportedOrbitalModel`, severity Error) rather than silently treated
as if it were a supported, simulatable frame. Nothing claims simulation works.

**FR6.5 (unit/dimension errors)**: Values failing the FR4/FR5 numeric contracts that
reach the validator (e.g. an orbit assembled from parts that bypassed a constructor path)
produce typed diagnostics (`DiagnosticCode::InvalidOrbitalElement`) rather than panics.
Because the model constructors already enforce ranges, this pass is defensive and
deterministic.

**FR6.6**: All new diagnostics use stable, documented `DiagnosticCode` values and sort
deterministically with existing diagnostics.

### FR7: Integration with existing model

**FR7.1**: `CelestialBody` gains optional orbital declarations: an optional
`CelestialIdentity`, an optional `ParentRef`, optional `KeplerianElements`, and an
optional `RotationModel`. All are `Option`-typed; absence means undeclared.

**FR7.2**: These additions are additive to `CelestialBody::new` (or an extended
constructor). Existing construction call sites and the existing golden roundtrip fixture
remain valid: an absent orbital section serializes/deserializes exactly as before where
practical, or the fixture is updated deliberately (see NFR5).

**FR7.3**: The new types are re-exported from `hpdl_model`'s crate root with rustdoc and
examples on every public item.

## Non-Functional Requirements

### NFR1: Determinism
- No `f32`/`f64` storage or arithmetic on any model or canonical path
  (`#![deny(clippy::float_arithmetic)]` already enforced in `hpdl-model`).
- Any collection reaching output uses `BTreeSet`/`BTreeMap` or explicit sorting.
- Diagnostics sort deterministically (existing `Ord` on `Diagnostic`).

### NFR2: Security / untrusted input
- Validate custom-name lengths and numeric ranges before use; reject with typed errors.
- All numeric range checks use `checked_*`; no panicking indexing, `unwrap`, or `expect`
  on any input path.
- Parent-cycle detection is bounded (no unbounded recursion/allocation on crafted input).

### NFR3: Reliability
- `hpdl-model`: keep `#![forbid(unsafe_code)]`, `#![deny(missing_docs, unwrap_used,
  expect_used, indexing_slicing, float_arithmetic)]`.
- `hpdl-sema`: keep `#![forbid(unsafe_code)]`, `#![deny(missing_docs, unwrap_used,
  expect_used, indexing_slicing)]`. The new orbital pass performs **no** float
  arithmetic (unlike the legacy `constraint_solver`, which is unchanged by this feature).

### NFR4: Documentation
- Rustdoc with at least one example on every new public item.
- CHANGELOG updated with the new model surface and the new diagnostic codes.

### NFR5: Golden fixtures
- A golden fixture under `hpdl-model/tests/golden/` freezes the canonical serialized
  form of a body **with** a full orbital declaration (identity, parent, Keplerian
  elements, rotation). The existing `minimal_planet.json` fixture is updated **only if**
  the additive fields change its serialized shape, and any such change is deliberate and
  reviewed.

## Constraints

- `hpdl-model` depends only on `thiserror` (+ `serde`), remaining the dependency root.
- `hpdl-sema` depends on `hpdl-model` only; it must not gain a dependency on `hpdl-pir`,
  `hpdl-compiler`, or `hpdl-cli`.
- Canonical PIR bytes change as a deliberate v1.0 → v2.0 format bump (ADR 0003); no new
  per-field PIR schema is introduced, only the format-version/domain-separator change.
- No new frontend syntax; no simulation.

## Dependency & versioning implications (surfaced up front)

- **No new crate dependency edges.** The feature lives in `hpdl-model` and `hpdl-sema`,
  which already depend as required. `serde` is already used by `hpdl-model` types.
- **PIR format impact: triggered and handled (ADR 0003).** Adding orbital fields to the
  bincode-encoded model changes canonical PIR bytes, so the PIR format is bumped v1.0 →
  v2.0, the hash domain separator becomes `HPDL-PIR-v2`, and the `hpdl-pir` crate is bumped
  `0.1.0` → `0.2.0`. Frozen golden vectors are updated deliberately (compiler
  `minimal_build.txt`). `cargo semver-checks -p hpdl-pir` is run as required for the
  published contract crate.
- **SemVer.** `hpdl-model` and `hpdl-sema` are pre-1.0 (0.1.x) and not published contract
  crates; adding `Option` fields and new enum variants/codes is new surface. New
  `DiagnosticCode` variants are additive. The `CelestialBody` constructor signature change
  is a pre-1.0 source change handled by updating in-workspace call sites (Task-tracked).

## Open decisions / approvals required

- **OD-07.1 PIR lowering of orbital fields**: **RESOLVED (ADR 0003).** The original
  assumption that this feature would not change canonical PIR bytes was incorrect: because
  PIR is encoded with bincode (non-self-describing, a presence byte per `Option` including
  `None`), adding the four `Option` fields to `CelestialBody` changes the canonical bytes
  for every definition. This is handled as a deliberate breaking format change — PIR
  format v1.0 → v2.0, hash domain separator `HPDL-PIR-v1` → `HPDL-PIR-v2`, `hpdl-pir` crate
  `0.1.0` → `0.2.0` — with the frozen golden vectors updated deliberately. See
  `docs/rfcs/0003-pir-v2-orbital-fields.md`.
- **OD-07.2 Angular representation**: This spec uses milli-degrees (`i32`) for angles and
  micro-units (`u32`) for eccentricity. Confirm precision/units before any PIR
  stabilization; an alternative (micro-degrees, or milli-arcseconds) can be chosen in the
  ADR without changing the validation contract shape.
- **OD-07.3 Eccentricity bound**: v1 restricts orbits to `[0, 1)` (closed/elliptical).
  Whether to support parabolic/hyperbolic trajectories (and their non-`a` parameterization)
  is deferred and recorded as an open decision; until then they are rejected explicitly.
- **OD-07.4 Richer absence states**: Language semantics distinguish `Absent`/`Unknown`/
  `Generated`. This feature models absence with `Option` (consistent with existing model
  types). Upgrading to an explicit tri-state is deferred and flagged, not silently
  decided in code.
- **OD-07.5 Multi-body parent graphs**: The current model is single-body. Cross-definition
  parent resolution and full DAG validation are deferred; the cycle detector is written to
  generalize.

## Success Criteria

1. ✅ The five types exist in `hpdl-model` with validating, non-panicking constructors,
   scaled-integer storage, no Earth defaults, and full rustdoc + examples.
2. ✅ `CelestialBody` carries optional orbital declarations; existing invariants (status/
   action uniqueness) and tests still hold.
3. ✅ `hpdl_sema::Validator` runs one new orbital pass producing the four required
   negative cases: parent cycle, reference-frame mismatch, unit/element error, and
   supported vs unsupported orbital-model declaration.
4. ✅ No panics on malformed input; all range checks use checked arithmetic; no floats on
   the new paths.
5. ✅ A golden fixture freezes the serialized form of a fully-declared orbital body; it
   roundtrips byte-for-byte.
6. ✅ CI passes: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
   warnings`, `cargo test --workspace`, `cargo deny check`.
7. ✅ CHANGELOG updated; rustdoc complete.

## Out of Scope

- Orbital propagation / ephemeris / N-body simulation (external HORIZON engine).
- PIR encoding of orbital fields and any PIR format-version change (OD-07.1).
- Native `.hpdl` or RON syntax for orbits (features 05 / 04).
- Multi-file / cross-definition parent-graph resolution (OD-07.5).
- CLI presentation (feature 15).

## References

- Design Doc: `docs/design/07-celestial-orbital-systems.md`
- Domain model: `.kiro/specs/01-planetary-domain-model/`
- Semantic Validation: `.kiro/specs/03-semantic-validation/`
- Language & Semantics steering (units, absence, no-float rules)
