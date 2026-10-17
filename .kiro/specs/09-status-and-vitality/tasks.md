# Status and Vitality — Tasks

> Primary crates: `hpdl-model`, `hpdl-sema` (and `hpdl-runtime-api` only if OD-09.1 chooses
> to activate the deferred crate).
>
> **Two decisions gate specific tasks — neither is pre-decided:**
> - **OD-09.1** (`hpdl-runtime-api` deferred vs. scaffold) gates Task 8 (`StatusResult`) and
>   Task 10 (replay test).
> - **OD-09.2** (PIR fold into v2.0 vs. bump v3.0) gates Task 9, and only applies if the
>   extended status declaration is encoded into PIR.
>
> Tasks 1–7 (the model + sema declaration surface) do **not** depend on either decision and
> can be implemented immediately.

## Task 1: StatusDimension + name/error foundation
**Status:** completed
**Depends on:** none

**Acceptance Criteria:**
- [ ] New module `crates/hpdl-model/src/status_vitality.rs`.
- [ ] `StatusDimension { Physical, NativeEcosystem, Civilization, Supernatural, Custom(name) }`
      with a validated `Custom` (1–64, `[A-Za-z0-9_\-.]`).
- [ ] New `Error` variants: `InvalidStatusDimensionName`, `InvalidCutPoints { reason }`,
      `ScoreOutOfRange { score, min, max }`, `ZeroEvaluationWindow`,
      `InvalidEvaluatorRef`, `InvalidBaselineRef`.
- [ ] Rustdoc + example; unit tests for dimension variants + invalid custom name.

**Files:**
- `crates/hpdl-model/src/status_vitality.rs` (new)
- `crates/hpdl-model/src/error.rs`, `crates/hpdl-model/src/lib.rs`

---

## Task 2: EvaluatorRef + BaselineRef
**Status:** completed
**Depends on:** Task 1

**Acceptance Criteria:**
- [ ] `EvaluatorRef` wrapping a `ModuleRef` (versioned evaluator identity).
- [ ] `BaselineRef { name, digest: Digest }` (reuses feature-08 `Digest`).
- [ ] Validating constructors; accessors; rustdoc + examples.
- [ ] Unit tests: valid refs; malformed digest/name rejected.

**Files:**
- `crates/hpdl-model/src/status_vitality.rs` (update)

---

## Task 3: CutPoints + classify (Amendment A1)
**Status:** completed
**Depends on:** Task 1

**Acceptance Criteria:**
- [ ] `CutPoints { scale_min, scale_max, cuts: Vec<(i64, BandLabel)> }`.
- [ ] Constructor rejects: `min >= max`; empty; first cut != min; out-of-range cut;
      non-strictly-increasing cuts (`InvalidCutPoints { reason }`).
- [ ] `classify(score) -> Result<&BandLabel, Error>`: band `i` = `[cut_i, cut_{i+1})`, last
      includes `max`; out-of-range score → `ScoreOutOfRange` (NO clamping).
- [ ] Pure integer logic; checked arithmetic; no floats.
- [ ] Unit tests incl. the required boundary classifications (24/25/49/50 over an integer
      scale; sub-integer 24.5/49.999 handled per OD-09.2 scaling) and out-of-range rejection.
- [ ] (OD-09.4) Reuse / share the Amendment A1 logic with `hpdl-sema::status_validator`.

**Files:**
- `crates/hpdl-model/src/status_vitality.rs` (update)

---

## Task 4: EvaluationWindow
**Status:** completed
**Depends on:** Task 1

**Acceptance Criteria:**
- [ ] `EvaluationWindow { span: u64 (>0), scope: Option<Name> }`.
- [ ] Zero span rejected (`ZeroEvaluationWindow`).
- [ ] Rustdoc + example; unit tests incl. zero-span rejection.

**Files:**
- `crates/hpdl-model/src/status_vitality.rs` (update)

---

## Task 5: Extend StatusDeclaration (OD-09.3: extend in place)
**Status:** completed
**Depends on:** Task 2, Task 3, Task 4

**Acceptance Criteria:**
- [ ] Optional `dimension`, `baseline`, `evaluator`, `cut_points`, `window` added via builder
      setters; `StatusDeclaration::new(name, min, max)` unchanged.
- [ ] Accessors for each; absent = undeclared.
- [ ] Existing model/sema tests still compile and pass.
- [ ] Re-exports + crate-root docs table updated.

**Files:**
- `crates/hpdl-model/src/status_declaration.rs` (update)
- `crates/hpdl-model/src/lib.rs`

---

## Task 6: New diagnostic codes (hpdl-sema)
**Status:** completed
**Depends on:** none

**Acceptance Criteria:**
- [ ] Reuse E4000 cut-point codes where they fit; add `IncompleteEvaluatorDeclaration`
      (E9201) and `UnsupportedEvaluator` (E9202) with stable `code()` strings.
- [ ] Unit test asserting the new code strings.

**Files:**
- `crates/hpdl-sema/src/diagnostic.rs` (update)

---

## Task 7: Status-vitality validation + wiring
**Status:** completed
**Depends on:** Task 5, Task 6

**Acceptance Criteria:**
- [ ] Extend `StatusValidator` to check extended declarations: cut-point rules (A1),
      evaluator completeness (window + cut-points required if evaluator present →
      `IncompleteEvaluatorDeclaration`), unsupported evaluator (`UnsupportedEvaluator`),
      dimension independence (any subset valid; absence never an error).
- [ ] Derived-only prohibition (E4005) unchanged.
- [ ] Wired via the existing `validate_statuses` call in the single `Validator`.
- [ ] No floats; no panics. Unit tests per branch + a fully-valid multidimensional status.

**Files:**
- `crates/hpdl-sema/src/status_validator.rs` (update)
- `crates/hpdl-sema/src/validator.rs` (if a new sub-pass is added)

---

## Task 8: StatusResult runtime contract — GATED on OD-09.1
**Status:** completed — BLOCKED on OD-09.1 decision
**Depends on:** Task 1, Task 3

Define the derived-output `StatusResult` contract. **Placement depends on OD-09.1:**

**Option A (keep `hpdl-runtime-api` deferred):**
- [ ] Specify `StatusResult` shape in the design; either place it provisionally in
      `hpdl-model` under a clearly-labelled "runtime contract (pre-runtime-api)" section, or
      leave unimplemented-but-documented. No deferred crate scaffolded.

**Option B (scaffold `hpdl-runtime-api`):**
- [ ] Create the `hpdl-runtime-api` crate (depends on `hpdl-pir` + selected stable model
      types); put `StatusResult` + `EvaluationWindow` runtime contracts there.
- [ ] Update the workflow steering crate-status table (`deferred` → active) and record an ADR.

**Common:**
- [ ] `StatusResult` carries dimension, score, band, evaluator version, sorted input hashes,
      evidence; derived-only (never authored).

---

## Task 9: PIR revision — GATED on OD-09.2 (only if encoded)
**Status:** completed — BLOCKED on OD-09.2 decision
**Depends on:** Task 5

If the extended status declaration is encoded into PIR, canonical bytes change.
Execute the approved option (fold into unreleased v2.0, or bump to v3.0), update the compiler
golden vector deliberately, and record/append an ADR. No golden vector silently regenerated.
If the extended fields are **not** encoded into PIR in this feature, this task is N/A and is
recorded as such.

**Files:**
- `crates/hpdl-pir/*` (option B only), `crates/hpdl-compiler/tests/golden/minimal_build.txt`,
  `docs/rfcs/*.md`

---

## Task 10: Fixtures, replay, docs
**Status:** completed
**Depends on:** Task 7 (and Task 8 for replay)

**Acceptance Criteria:**
- [ ] Golden roundtrip fixture for a fully-declared multidimensional status.
- [ ] Boundary-classification golden freezing 24.5 / 25 / 49.999 outcomes.
- [ ] Replay test asserting identical `StatusResult` for identical versioned inputs
      (location per OD-09.1).
- [ ] `DIAGNOSTIC_CODES.md` + CHANGELOG updated; rustdoc complete; `cargo doc` clean.

**Files:**
- `crates/hpdl-model/tests/golden/*`, `crates/hpdl-sema/DIAGNOSTIC_CODES.md`, `CHANGELOG.md`

---

## Definition of Done (All Tasks)

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo deny check`
- [ ] `cargo semver-checks -p hpdl-pir` (and `-p hpdl-runtime-api` if OD-09.1 scaffolds it).
- [ ] No `unwrap()`/`expect()`/panic on input paths; checked arithmetic; no `f32`/`f64`.
- [ ] Statuses remain derived — nothing authors a score; out-of-range rejected, never clamped.
- [ ] Dimensions independent; absence never represented as zero.
- [ ] OD-09.1 and OD-09.2 resolved via ADRs; any PIR golden-vector change deliberate.

## Task Execution Order

1. Model primitives: Tasks 1–4.
2. Model integration: Task 5.
3. Validation: Tasks 6–7.
4. **Decision gate:** resolve OD-09.1 → Task 8; resolve OD-09.2 → Task 9 (if applicable).
5. Fixtures/replay/docs: Task 10.

