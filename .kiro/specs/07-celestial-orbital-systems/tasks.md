# Celestial and Orbital Systems — Tasks

## Task 1: ReferenceFrame + validated name newtypes
**Status:** completed
**Depends on:** none

Add the `ReferenceFrame` enum and the validated name newtype used by both
`ReferenceFrame::Custom` and `CelestialIdentity::Other`.

**Acceptance Criteria:**
- [ ] New module `crates/hpdl-model/src/orbital.rs`.
- [ ] `ReferenceFrame` enum: `EquatorialJ2000`, `EclipticJ2000`, `BodyFixed`,
      `ParentCentered`, `Custom(name)`.
- [ ] Validated name helper (1–64 chars, `[A-Za-z0-9_\-.]`) returning
      `Error::InvalidFrameName` / reused for kind names.
- [ ] Derives `Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize`.
- [ ] Rustdoc + example on each public item.
- [ ] Unit tests: each variant constructs; invalid custom name rejected.

**Files:**
- `crates/hpdl-model/src/orbital.rs` (new)
- `crates/hpdl-model/src/error.rs` (new variants)
- `crates/hpdl-model/src/lib.rs` (module + re-export)

---

## Task 2: CelestialIdentity
**Status:** completed
**Depends on:** Task 1

**Acceptance Criteria:**
- [ ] `CelestialIdentity` enum with closed kinds + `Other(name)` (validated name).
- [ ] Derives + rustdoc + example.
- [ ] Unit tests: closed variants; valid/invalid `Other` name.

**Files:**
- `crates/hpdl-model/src/orbital.rs` (update)
- `crates/hpdl-model/src/lib.rs` (re-export)

---

## Task 3: KeplerianElements
**Status:** completed
**Depends on:** Task 1

Scaled-integer orbital elements with a validating constructor.

**Acceptance Criteria:**
- [ ] Fields exactly as in design §2.3 (all integer; no `f32`/`f64`).
- [ ] Constructor rejects: `semi_major_axis_m == 0`, `eccentricity_micro >= 1_000_000`,
      inclination outside `[0, 180_000]`, each longitude/argument/anomaly outside
      `[0, 360_000)` — with typed `Error` variants.
- [ ] Checked arithmetic only; no panics; no float.
- [ ] Accessors for every field + the frame.
- [ ] Rustdoc + example.
- [ ] Unit tests incl. boundaries: `e = 999_999` ok, `e = 1_000_000` rejected;
      inclination `180_000` ok, `180_001` rejected; node `359_999` ok, `360_000` rejected.

**Files:**
- `crates/hpdl-model/src/orbital.rs` (update)
- `crates/hpdl-model/src/error.rs` (new variants)
- `crates/hpdl-model/src/lib.rs` (re-export)

---

## Task 4: ParentRef + RotationModel
**Status:** completed
**Depends on:** Task 1

**Acceptance Criteria:**
- [ ] `ParentRef { target: BodyId, frame: ReferenceFrame }` with accessors; constructor
      never panics.
- [ ] `RotationModel { sidereal_period_s: i64, axial_tilt_mdeg: i32, frame }`.
- [ ] `RotationModel` rejects `sidereal_period_s == 0` and axial tilt outside
      `[0, 180_000]`; negative period allowed (retrograde).
- [ ] Rustdoc + examples.
- [ ] Unit tests: valid construction; zero period rejected; retrograde ok; tilt bounds.

**Files:**
- `crates/hpdl-model/src/orbital.rs` (update)
- `crates/hpdl-model/src/error.rs` (new variants)
- `crates/hpdl-model/src/lib.rs` (re-export)

---

## Task 5: Extend CelestialBody with optional orbital declarations
**Status:** completed
**Depends on:** Task 2, Task 3, Task 4

**Acceptance Criteria:**
- [ ] `CelestialBody` gains `Option<CelestialIdentity>`, `Option<ParentRef>`,
      `Option<KeplerianElements>`, `Option<RotationModel>`.
- [ ] Construction stays non-panicking and preserves action/status uniqueness checks.
      Prefer an additive builder-style setter (e.g. `with_orbit`, `with_parent`,
      `with_identity`, `with_rotation`) so existing `new(..)` call sites keep compiling,
      OR update the signature and fix all in-workspace call sites.
- [ ] Accessors for each new field; absent = undeclared (no default).
- [ ] All existing `hpdl-model` and `hpdl-sema` tests still compile and pass.
- [ ] Rustdoc updated; crate-root docs table lists the new types.

**Files:**
- `crates/hpdl-model/src/celestial_body.rs` (update)
- `crates/hpdl-model/src/lib.rs` (docs table)
- any in-workspace call sites if the constructor signature changes

---

## Task 6: New orbital DiagnosticCodes (hpdl-sema)
**Status:** completed
**Depends on:** none

**Acceptance Criteria:**
- [ ] `DiagnosticCode` gains `ParentCycle` (E8001), `ReferenceFrameMismatch` (E8002),
      `InvalidOrbitalElement` (E8003), `UnsupportedOrbitalModel` (E8004).
- [ ] `code()` returns the stable strings above.
- [ ] Unit test asserting each new code string.

**Files:**
- `crates/hpdl-sema/src/diagnostic.rs` (update)

---

## Task 7: OrbitalValidator pass
**Status:** completed
**Depends on:** Task 5, Task 6

**Acceptance Criteria:**
- [ ] New `crates/hpdl-sema/src/orbital_validator.rs` with an `OrbitalValidator`.
- [ ] Checks per design §4: parent self-cycle → `ParentCycle`; frame mismatch →
      `ReferenceFrameMismatch`; `Custom` frame → `UnsupportedOrbitalModel` (Error);
      defensive element range check → `InvalidOrbitalElement`.
- [ ] Cycle detection is a bounded reachable-set walk (no unbounded recursion).
- [ ] No float arithmetic; no `unwrap`/`expect`/panicking index.
- [ ] Unit tests for each branch + a fully-valid orbital body producing no diagnostics.

**Files:**
- `crates/hpdl-sema/src/orbital_validator.rs` (new)
- `crates/hpdl-sema/src/lib.rs` (module)

---

## Task 8: Wire the pass into the single Validator
**Status:** completed
**Depends on:** Task 7

**Acceptance Criteria:**
- [ ] `Validator` constructs an `OrbitalValidator` and calls `validate_orbital` once in
      `validate`, merging diagnostics before the final sort.
- [ ] Existing passes and tests unaffected; diagnostics remain sorted/deterministic.
- [ ] Validator-level test: a definition with a parent cycle fails with `ParentCycle`.

**Files:**
- `crates/hpdl-sema/src/validator.rs` (update)

---

## Task 9: Golden fixture for a fully-declared orbital body
**Status:** completed
**Depends on:** Task 5

**Acceptance Criteria:**
- [ ] `crates/hpdl-model/tests/golden/orbital_body.json` holds a body with identity,
      parent, Keplerian elements, and rotation.
- [ ] The roundtrip harness (`tests/roundtrip.rs`) is extended (mirror structs + a data
      driven case) so the fixture roundtrips byte-for-byte.
- [ ] `minimal_planet.json` updated only if the additive fields change its serialized
      shape; any change is deliberate and noted in the test comment.

**Files:**
- `crates/hpdl-model/tests/golden/orbital_body.json` (new)
- `crates/hpdl-model/tests/roundtrip.rs` (update)
- `crates/hpdl-model/tests/golden/minimal_planet.json` (update only if required)

---

## Task 10: Docs and CHANGELOG
**Status:** completed
**Depends on:** Task 9

**Acceptance Criteria:**
- [ ] Rustdoc on every new public item (examples on primary types).
- [ ] `cargo doc -p hpdl-model -p hpdl-sema` builds without warnings.
- [ ] CHANGELOG records the new model types, new error variants, and the four new
      diagnostic codes.
- [ ] A note records OD-07.1 (no PIR change here) and OD-07.3 (eccentricity bound).

**Files:**
- `crates/hpdl-model/src/*.rs`, `crates/hpdl-sema/src/*.rs`
- `CHANGELOG.md`

---

## Task 11: PIR format v2 bump (ADR 0003)
**Status:** completed
**Depends on:** Task 5

Adding the orbital `Option` fields to the bincode-encoded `CelestialBody` changes
canonical PIR bytes, so the format is bumped deliberately rather than silently
regenerating vectors.

**Acceptance Criteria:**
- [ ] `PirVersion::CURRENT` bumped `1.0` → `2.0`; test asserts current is 2.0 and that a
      v2 decoder rejects v1 payloads.
- [ ] Hash domain separator `HPDL-PIR-v1` → `HPDL-PIR-v2`.
- [ ] `hpdl-pir` crate version `0.1.0` → `0.2.0`.
- [ ] Compiler golden fixture `tests/golden/minimal_build.txt` updated deliberately to
      `pir_version = 2.0` + new hash.
- [ ] ADR `docs/rfcs/0003-pir-v2-orbital-fields.md` recorded.
- [ ] `cargo semver-checks -p hpdl-pir` run.

**Files:**
- `crates/hpdl-pir/src/version.rs`, `crates/hpdl-pir/src/hash.rs`, `crates/hpdl-pir/Cargo.toml`
- `crates/hpdl-compiler/tests/golden/minimal_build.txt`
- `docs/rfcs/0003-pir-v2-orbital-fields.md`

---

## Definition of Done (All Tasks)

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo deny check`
- [ ] No `unwrap()`/`expect()`/panic on input paths; checked arithmetic in range checks.
- [ ] No unsafe code; no `f32`/`f64` on the new model or validation paths.
- [ ] No unsupported feature silently claims simulation or success (custom frames are
      flagged, not simulated).
- [ ] No PIR bytes changed; OD-07.1 recorded.

## Task Execution Order

1. Model primitives: Tasks 1–4.
2. Model integration: Task 5.
3. Validation: Tasks 6–8.
4. Fixtures & docs: Tasks 9–10.
