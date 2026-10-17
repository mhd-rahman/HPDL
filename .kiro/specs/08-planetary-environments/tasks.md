# Planetary Environments — Tasks

> Follows the feature-07 pattern. Primary crates: `hpdl-model`, `hpdl-sema`.
> **Blocking decision:** OD-08.1 (PIR format: fold into v2.0 vs. bump to v3.0) must be
> resolved before Task 9 (model→PIR encoding) is executed. Tasks 1–8 do not depend on it.

## Task 1: Name/digest newtypes + error variants
**Status:** completed
**Depends on:** none

Add the validated newtypes shared by the environment types, plus the new error variants.

**Acceptance Criteria:**
- [ ] New module `crates/hpdl-model/src/environment.rs`.
- [ ] `GasId` (1–32, `[A-Za-z0-9_\-.]`) and a shared name validator; `Digest` newtype
      (exactly 64 lowercase hex chars).
- [ ] `Error` variants: `InvalidEnvironmentName`, `EmptyComposition`,
      `CompositionFractionOutOfRange`, `CompositionSumOutOfTolerance`, `ZeroPressure`,
      `InvalidTemperatureOrder`, `CoverageOutOfRange`, `InvalidAssetDigest`.
- [ ] Rustdoc + example on each public newtype; unit tests for valid/invalid names + digests.

**Files:**
- `crates/hpdl-model/src/environment.rs` (new)
- `crates/hpdl-model/src/error.rs` (new variants)
- `crates/hpdl-model/src/lib.rs` (module + re-exports)

---

## Task 2: Composition
**Status:** completed
**Depends on:** Task 1

**Acceptance Criteria:**
- [ ] `Composition { components: BTreeMap<GasId, u32>, tolerance_ppm: u32 }`.
- [ ] Constructor rejects: empty; any component `> 1_000_000`; checked sum outside
      `[1e6 - tol, 1e6 + tol]` (u64 accumulator, no overflow).
- [ ] Accessors; rustdoc + example.
- [ ] Unit tests incl. boundaries: exact sum ok; sum at ±tolerance ok; just beyond rejected;
      component `1_000_001` rejected; empty rejected.

**Files:**
- `crates/hpdl-model/src/environment.rs` (update)
- `crates/hpdl-model/src/error.rs`

---

## Task 3: AtmosphereProfile + ThermalProfile
**Status:** completed
**Depends on:** Task 2

**Acceptance Criteria:**
- [ ] `AtmosphereProfile { surface_pressure_pa: u64 (>0), composition, climate_model: Option<ModuleRef> }`.
- [ ] `ThermalProfile { mean_mk, min_mk, max_mk: u64 }` rejecting unless `min <= mean <= max`.
- [ ] Zero pressure rejected (`ZeroPressure`); inverted temperature rejected
      (`InvalidTemperatureOrder`).
- [ ] Rustdoc + examples; unit tests incl. boundary (min==mean==max ok).

**Files:**
- `crates/hpdl-model/src/environment.rs` (update)

---

## Task 4: Hydrosphere, Cryosphere, Biosphere
**Status:** completed
**Depends on:** Task 1

**Acceptance Criteria:**
- [ ] `Hydrosphere { surface_coverage_ppm: u32 (0..=1e6), dominant_liquid: Option<GasId> }`.
- [ ] `Cryosphere { surface_coverage_ppm: u32 (0..=1e6) }`.
- [ ] Coverage `> 1_000_000` rejected (`CoverageOutOfRange`).
- [ ] `ComplexityTier` (closed + `Other(name)`) and `Biosphere { Absent, Unknown, Present(tier) }`.
- [ ] Rustdoc + examples; unit tests incl. absent/unknown/present round-trips and coverage bounds.

**Files:**
- `crates/hpdl-model/src/environment.rs` (update)

---

## Task 5: AssetRef + AssetKind
**Status:** completed
**Depends on:** Task 1

**Acceptance Criteria:**
- [ ] `AssetKind { Texture, Mesh, Heightmap, Other(name) }`.
- [ ] `AssetRef { name, kind, digest: Digest }` — digest required and validated.
- [ ] Missing/malformed digest rejected (`InvalidAssetDigest`): wrong length, non-hex,
      uppercase all rejected.
- [ ] Rustdoc + example; unit tests for each rejection + a valid ref.

**Files:**
- `crates/hpdl-model/src/environment.rs` (update)

---

## Task 6: EnvironmentDefinition + CelestialBody integration
**Status:** completed
**Depends on:** Task 3, Task 4, Task 5

**Acceptance Criteria:**
- [ ] `EnvironmentDefinition` aggregating the sub-profiles + tri-state `Biosphere` +
      `BTreeMap<String, AssetRef>` assets (dedup by name).
- [ ] `CelestialBody::with_environment(..)` additive builder; `environment()` accessor;
      absent = undeclared.
- [ ] Existing `new(..)` call sites and all existing model/sema tests still compile and pass.
- [ ] Re-exports from `lib.rs`; crate-root docs table lists the new types.

**Files:**
- `crates/hpdl-model/src/environment.rs` (update)
- `crates/hpdl-model/src/celestial_body.rs` (update)
- `crates/hpdl-model/src/lib.rs` (docs + re-exports)

---

## Task 7: Environment DiagnosticCodes (hpdl-sema)
**Status:** completed
**Depends on:** none

**Acceptance Criteria:**
- [ ] `DiagnosticCode` gains `CompositionOutOfBounds`, `InvalidEnvironmentValue`,
      `UnsupportedEnvironmentModule`, `MissingAssetDigest` with stable E9100-series codes
      (distinct from the existing E1000–E8000 and the E9999 generic).
- [ ] `code()` returns the stable strings; unit test asserts each.

**Files:**
- `crates/hpdl-sema/src/diagnostic.rs` (update)

---

## Task 8: EnvironmentValidator pass + wiring
**Status:** completed
**Depends on:** Task 6, Task 7

**Acceptance Criteria:**
- [ ] New `crates/hpdl-sema/src/environment_validator.rs` with an `EnvironmentValidator`.
- [ ] Checks per design §4: composition bounds; zero pressure / temperature inversion /
      coverage range → `InvalidEnvironmentValue`; declared `climate_model` →
      `UnsupportedEnvironmentModule`; malformed digest → `MissingAssetDigest`; `Absent`
      biosphere produces NO diagnostic.
- [ ] Wired once into `Validator::validate`, merged before the final sort.
- [ ] No float arithmetic; no `unwrap`/`expect`/panicking index.
- [ ] Unit tests per branch + a fully-valid environment producing no diagnostics + an
      `Absent`-biosphere body producing no diagnostics; a validator-level test.

**Files:**
- `crates/hpdl-sema/src/environment_validator.rs` (new)
- `crates/hpdl-sema/src/lib.rs` (module)
- `crates/hpdl-sema/src/validator.rs` (wire pass)

---

## Task 9: PIR format revision (OD-08.1)
**Status:** completed — OD-08.1 resolved as **option A** (folded into unreleased v2.0): PirVersion stays 2.0, domain separator stays HPDL-PIR-v2, hpdl-pir stays 0.2.0; compiler golden vector regenerated deliberately (hash `6e1610fe…` → `23d96f5d…`); ADR 0003 amended.
**Depends on:** Task 6

Encoding the new `environment` field changes canonical PIR bytes. Execute the approved
option from OD-08.1.

**Acceptance Criteria (option A — fold into unreleased v2.0):**
- [ ] `PirVersion::CURRENT` stays `2.0`; domain separator stays `HPDL-PIR-v2`.
- [ ] Compiler golden fixture `tests/golden/minimal_build.txt` regenerated deliberately with
      the new hash; ADR note appended to `docs/rfcs/0003-pir-v2-orbital-fields.md` (or a new
      ADR) recording that v2.0 also carries environment fields.

**Acceptance Criteria (option B — bump to v3.0):**
- [ ] `PirVersion::CURRENT = 3.0`; domain separator `HPDL-PIR-v3`; `hpdl-pir` crate
      `0.2.0 → 0.3.0`; v3 decoder rejects v2 payloads (test).
- [ ] New ADR `docs/rfcs/0004-pir-v3-environment-fields.md`.
- [ ] Compiler golden fixture + any hardcoded PIR-version assertions updated deliberately.

**Common:**
- [ ] No golden vector silently regenerated; the change is reviewed and recorded.

**Files:**
- `crates/hpdl-pir/src/{version.rs,hash.rs}` + `Cargo.toml` (option B only)
- `crates/hpdl-compiler/tests/golden/minimal_build.txt`
- `docs/rfcs/*.md`

---

## Task 10: Golden fixture for a fully-declared environment
**Status:** completed
**Depends on:** Task 6

**Acceptance Criteria:**
- [ ] `crates/hpdl-model/tests/golden/environment_body.json` with atmosphere (multi-gas
      composition), thermal, hydrosphere, cryosphere, a `Present` biosphere, and ≥1 asset.
- [ ] Roundtrip harness extended so the fixture roundtrips byte-for-byte through the real
      model types.

**Files:**
- `crates/hpdl-model/tests/golden/environment_body.json` (new)
- `crates/hpdl-model/tests/roundtrip.rs` (update)

---

## Task 11: Docs and CHANGELOG
**Status:** completed
**Depends on:** Task 10

**Acceptance Criteria:**
- [ ] Rustdoc on every new public item (examples on primary types).
- [ ] `DIAGNOSTIC_CODES.md` (hpdl-sema) extended with the E9100 environment block.
- [ ] `cargo doc -p hpdl-model -p hpdl-sema` builds without warnings.
- [ ] CHANGELOG records the new model types, error variants, diagnostic codes, and the
      OD-08.1 PIR decision outcome.

**Files:**
- `crates/hpdl-model/src/*.rs`, `crates/hpdl-sema/src/*.rs`, `crates/hpdl-sema/DIAGNOSTIC_CODES.md`
- `CHANGELOG.md`

---

## Definition of Done (All Tasks)

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo deny check`
- [ ] Published-contract check if PIR changed: `cargo semver-checks -p hpdl-pir`.
- [ ] No `unwrap()`/`expect()`/panic on input paths; checked arithmetic in all range/sum checks.
- [ ] No unsafe; no `f32`/`f64` on the new model or validation paths.
- [ ] No unsupported feature silently claims simulation (climate models flagged, assets not
      fetched); absent biosphere is a first-class value, never a zero score.
- [ ] OD-08.1 recorded as an ADR; any PIR golden-vector change deliberate and reviewed.

## Task Execution Order

1. Model primitives: Tasks 1–5.
2. Model integration: Task 6.
3. Validation: Tasks 7–8.
4. **Decision gate:** resolve OD-08.1, then Task 9.
5. Fixtures & docs: Tasks 10–11.

