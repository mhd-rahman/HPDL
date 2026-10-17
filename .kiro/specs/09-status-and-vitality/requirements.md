# Status and Vitality — Requirements

## Overview

This feature makes HPDL status/vitality a **multidimensional, derived, planet-specific**
contract. It extends the existing minimal `StatusDeclaration` (name + scale) with the
pieces a versioned evaluator needs to turn planetary state into a banded status score:
a **baseline** reference, an **evaluator** reference, ordered **cut-points** that partition
the scale into named bands, and an **evaluation window**. It also defines the
runtime-facing `StatusResult` contract that an evaluator produces.

New / extended types (per `docs/design/09-status-and-vitality.md`):

- `StatusDimension` — the independent dimension a status belongs to: `Physical`,
  `NativeEcosystem`, `Civilization`, optional `Supernatural`, plus a registered
  `Custom(name)` escape hatch. Dimensions are **independent**; a missing dimension is
  "not applicable", never "zero".
- `BaselineRef` — a planet-specific baseline the evaluator measures against (a versioned,
  content-addressed reference; baselines are per-planet, not global).
- `EvaluatorRef` — a versioned reference to the evaluator implementation (name + version),
  declaring the evaluator that derives this status.
- `CutPoints` — ordered lower-bound cut points over the status scale (Amendment A1):
  each band is `[lower, next)`, the last band includes the maximum. First cut must equal
  the scale minimum; cuts strictly increasing; in range; out-of-range **rejected**, never
  clamped.
- `EvaluationWindow` — the temporal window/scope the evaluator aggregates over (e.g. a
  tick span), as scaled integers.
- `StatusResult` — the derived output contract: dimension, score, the band it fell in,
  the evaluator version and input hashes used (for replay/audit), and an evidence summary.
  **Statuses are derived, never written** — `StatusResult` is produced by an evaluator,
  not authored.

Primary crates named by the design: `hpdl-model`, `hpdl-sema`, **and `hpdl-runtime-api`**.
See the blocking open decision OD-09.1 below regarding the deferred runtime crate.

## Goals

1. **Multidimensional status model**: add `StatusDimension`, `BaselineRef`, `EvaluatorRef`,
   `CutPoints`, and `EvaluationWindow` to the declaration surface so a status declares which
   independent dimension it measures, against which baseline, via which versioned evaluator,
   banded by which cut-points, over which window.
2. **Derived, never mutated**: nothing in this feature lets an action or author assign a
   status score. The existing action-write prohibition (E4005) is preserved and extended to
   the richer declaration.
3. **Amendment A1 cut-points**: reuse/extend the existing threshold rules so the required
   boundary classifications (24.5 / 25 / 49.999) are correct and out-of-range scores are
   rejected.
4. **Deterministic & replayable**: `StatusResult` records evaluator version + input hashes
   so a result can be audited and replayed; no floats, no ambient state.
5. **Explicit failure**: unknown dimensions, unsupported evaluators, malformed cut-points,
   and out-of-range scores fail with typed diagnostics using stable codes.

## Scope boundaries

- **No evaluation engine.** This feature declares the *contract* for how a status is
  derived (dimension, baseline, evaluator, cut-points, window) and the *shape* of a result.
  The actual computation of a `StatusResult` from state is the external HORIZON engine's
  job. HPDL does not run evaluators.
- **No baseline/evaluator fetching.** `BaselineRef`/`EvaluatorRef` are versioned,
  content-addressed references validated for well-formedness; resolving/fetching them is the
  stdlib/registry feature (14).
- **The 12 atmosphere components are inputs, not 12 weighted scores.** A status over the
  `Physical` dimension may consume atmosphere inputs, but this feature never asserts
  illustrative weights are scientifically validated.
- **Dimensions are independent and not aggregated.** This feature does not combine
  dimensions into a single "overall vitality" number; each dimension is its own status.

## Functional Requirements

### FR1: StatusDimension (independent dimensions, absent ≠ zero)

**FR1.1**: `StatusDimension` is a typed enum: `Physical`, `NativeEcosystem`, `Civilization`,
`Supernatural`, plus a validated `Custom(name)` escape hatch (1–64 chars, `[A-Za-z0-9_\-.]`).

**FR1.2**: Dimensions are independent; the model never requires all dimensions to be present
and never represents an absent dimension as a zero score. A body may declare statuses for any
subset of dimensions.

### FR2: EvaluatorRef and BaselineRef (versioned, content-addressed)

**FR2.1**: `EvaluatorRef` carries a versioned evaluator identity (name + version, reusing the
`ModuleRef` shape or a dedicated newtype). It declares *which* evaluator derives the status;
it is not executed here.

**FR2.2**: `BaselineRef` carries a planet-specific baseline reference: a logical name plus a
required content digest (reusing the feature-08 `Digest` type, 64-char lowercase hex). Per
the design, baselines are *planet-specific*, so a `BaselineRef` is meaningful only in the
context of the body that declares it.

**FR2.3**: An unresolved/unsupported evaluator is surfaced by the validator as an explicit
non-executable declaration (see FR6.3), never silently treated as runnable.

### FR3: CutPoints (Amendment A1)

**FR3.1**: `CutPoints` stores ordered lower-bound cut points over a status's `[scale_min,
scale_max]` scale, each paired with a band label. Rules (Amendment A1):
- first cut equals `scale_min`;
- cuts strictly increasing (sorted, unique);
- every cut within `[scale_min, scale_max]`;
- band `i` is `[cut_i, cut_{i+1})`; the last band includes `scale_max`.

**FR3.2**: Construction rejects unsorted, duplicate, out-of-range, or first-cut-mismatch
inputs with typed errors. Out-of-range scores at classification time are **rejected, never
clamped**. The 24.5 / 25 / 49.999 boundary classifications must be correct (these drive the
required test cases).

**FR3.3**: `CutPoints` provides a `classify(score)` that returns the band for an in-range
score and an explicit error for an out-of-range score — this is pure, integer, deterministic.

### FR4: EvaluationWindow

**FR4.1**: `EvaluationWindow` declares the temporal aggregation window as scaled integers
(e.g. a tick count or a duration in seconds; `> 0`), plus an optional scope descriptor.

**FR4.2**: A zero or absent window on a declared evaluator is rejected (an evaluator that
aggregates over nothing is meaningless).

### FR5: Extended StatusDeclaration

**FR5.1**: The status declaration gains optional `dimension`, `baseline`, `evaluator`,
`cut_points`, and `window`. These are added additively so existing `StatusDeclaration::new`
call sites keep compiling (either new optional fields with builder setters, or a parallel
extended type — see OD-09.3).

**FR5.2**: A status that declares an `evaluator` must also declare a `window` and
`cut_points` (an evaluator with no bands/window cannot produce a banded result); this
cross-field rule is enforced by the validator.

**FR5.3**: Status values remain **derived**; nothing added here permits authoring a score.
The existing action→status write prohibition is unchanged.

### FR6: StatusResult (runtime contract) — SUBJECT TO OD-09.1

**FR6.1**: `StatusResult` is the derived-output contract an evaluator produces:
`dimension`, `score` (in range for the scale), the classified band label, the
`evaluator` version, the input hashes consumed, and an evidence summary string.

**FR6.2**: `StatusResult` is a *contract type for replay/audit*: given the same evaluator
version, same inputs (by hash), same window, a replay produces an identical `StatusResult`.
It is never authored by a definition.

**FR6.3**: Where `StatusResult` lives depends on OD-09.1 (the `hpdl-runtime-api` crate is
currently deferred). Until that is resolved, this feature does **not** scaffold a deferred
crate; the result-contract shape is specified but its crate placement is gated.

### FR7: Semantic validation (hpdl-sema)

**FR7.1**: A `StatusDimensionValidator` (or an extension of the existing `StatusValidator`)
runs once in the single `Validator`, merging/sorting diagnostics with existing passes.

**FR7.2 (cut-points)**: Re-verify Amendment A1 cut-point rules on the extended declaration
(`CutPointsOutOfRange` / reuse existing E4000-series codes where they already fit).

**FR7.3 (evaluator completeness)**: A declared evaluator without a window or cut-points →
typed diagnostic (`IncompleteEvaluatorDeclaration`).

**FR7.4 (unsupported evaluator)**: A declared evaluator with no registered implementation →
`UnsupportedEvaluator` (Error) — accepted as a declaration, never run.

**FR7.5 (dimension independence)**: Declaring statuses on a subset of dimensions is valid;
the validator never requires all dimensions and never treats absence as zero.

**FR7.6**: New diagnostics use stable, documented `DiagnosticCode` values (reusing the E4000
status block where applicable, with a small E9200-series extension for the dimension/
evaluator/window checks if needed), sorting deterministically.

## Non-Functional Requirements

### NFR1: Determinism
- No `f32`/`f64` storage or arithmetic; cut-points, scores, and windows are integers.
- Cut-points and any maps use `BTreeMap`/ordered storage; `classify` is a pure function.
- `StatusResult` replay is byte-stable given identical versioned inputs.

### NFR2: Security / untrusted input
- Validate dimension/evaluator/baseline name + digest formats and window bounds before use.
- Checked arithmetic in classification; no panicking index/`unwrap`/`expect` on input paths.
- Bound the number of cut-points per status (documented maximum).

### NFR3: Reliability
- `hpdl-model`: keep the existing strict lints (`forbid(unsafe_code)`, `deny(missing_docs,
  unwrap_used, expect_used, indexing_slicing, float_arithmetic)`).
- `hpdl-sema`: the new checks perform no float arithmetic.

### NFR4: Documentation
- Rustdoc with examples on every new public item; `DIAGNOSTIC_CODES.md` extended.
- CHANGELOG updated.

### NFR5: Golden fixtures & replay
- A golden fixture freezes a fully-declared multidimensional status (dimension + baseline +
  evaluator + cut-points + window) and roundtrips byte-for-byte.
- A classification golden/boundary test freezes the 24.5 / 25 / 49.999 band outcomes.
- A replay test asserts identical `StatusResult` for identical versioned inputs (location
  gated on OD-09.1).

## Constraints

- `hpdl-model` depends only on `thiserror` (+ `serde`); `hpdl-sema` depends on `hpdl-model`.
- No new frontend syntax; no evaluation engine; no baseline/evaluator fetching.
- `hpdl-pir`/`hpdl-runtime-api` must not depend on `hpdl-compiler`.

## Dependency & versioning implications (surfaced up front)

- **PIR format impact — DECISION REQUIRED (OD-09.2).** If the extended status declaration
  adds fields to the bincode-encoded model, canonical PIR bytes change again. PIR is at the
  unreleased v2.0. As with feature 08 (OD-08.1), the options are **(A) fold into v2.0** (if
  still unreleased) or **(B) bump to v3.0**. This spec records the decision; it does not
  pre-pick. Whichever is chosen, golden vectors are updated deliberately and recorded in an
  ADR.
- **Runtime-contract crate — BLOCKING DECISION (OD-09.1):** the design names
  `hpdl-runtime-api`, which the workflow status table marks **deferred** ("Do NOT scaffold
  crates marked deferred unless the task explicitly asks"). `StatusResult` /
  `EvaluationWindow` are the runtime-facing contracts. This is a direct conflict between the
  design and the steering; see OD-09.1 for options. The model/sema declaration surface
  (FR1–FR5, FR7) does **not** require the runtime crate and can proceed; only `StatusResult`
  placement (FR6) is gated.
- **SemVer.** `hpdl-model`/`hpdl-sema` are pre-1.0; additive. If `hpdl-runtime-api` is
  scaffolded (OD-09.1 option B), it is a new published-contract crate with its own versioning.

## Open decisions / approvals required

- **OD-09.1 `hpdl-runtime-api` — RESOLVED: Option A (keep deferred, specify-only).**
  `hpdl-runtime-api` is **not** scaffolded (it stays deferred per the workflow steering). The
  full declaration surface (FR1–FR5, FR7) is implemented in `hpdl-model`/`hpdl-sema`.
  `StatusResult` is implemented provisionally in `hpdl-model` under a clearly-labelled
  "runtime contract (pre-runtime-api)" section, so the shape and replay invariant exist and
  are tested without activating the deferred crate. When runtime integration begins,
  `StatusResult`/`EvaluationWindow` move to `hpdl-runtime-api` (tracked as future work).
- **OD-09.2 PIR revision — RESOLVED: N/A (fields not encoded into PIR this feature).** The
  extended status fields are **not** lowered into canonical PIR in feature 09, so PIR bytes
  do not change and no golden-vector churn is required. When these fields are eventually
  encoded, that is a separate PIR format-version decision (fold-into-current vs. bump),
  recorded in an ADR at that time.
- **OD-09.3 Extend-in-place vs parallel type:** add optional fields to `StatusDeclaration` vs
  introduce a richer `StatusSpec` wrapping it. Default: extend in place with builder setters,
  consistent with how `CelestialBody` gained orbital/environment fields.
- **OD-09.4 Cut-point reuse:** the existing `StatusValidator::validate_status_thresholds`
  already implements Amendment A1 over a `BTreeMap`. Confirm whether `CutPoints` is a new
  model type that reuses that logic, or whether the validator logic is lifted into the model.
- **OD-09.5 Evaluator/baseline registry trust:** how "supported" evaluators/baselines are
  recognized is deferred to feature 14; until then declared evaluators are flagged
  non-executable.

## Success Criteria

1. ✅ `StatusDimension`, `BaselineRef`, `EvaluatorRef`, `CutPoints`, `EvaluationWindow` exist
   in `hpdl-model` with validating, non-panicking, float-free constructors + rustdoc.
2. ✅ Status declaration carries the optional dimension/baseline/evaluator/cut-points/window;
   existing invariants and tests still hold.
3. ✅ `CutPoints::classify` gives correct Amendment A1 bands incl. 24.5 / 25 / 49.999, and
   rejects out-of-range scores (no clamping).
4. ✅ `hpdl_sema::Validator` enforces cut-point rules, evaluator completeness, unsupported
   evaluators, and dimension independence with stable typed codes.
5. ✅ A replay-oriented `StatusResult` contract is specified; its placement follows OD-09.1.
6. ✅ No panics on malformed input; no floats; checked arithmetic.
7. ✅ Golden fixtures: multidimensional status roundtrip + boundary classification; replay
   test per OD-09.1.
8. ✅ OD-09.1 and OD-09.2 resolved via ADRs; any PIR golden-vector change deliberate.
9. ✅ CI passes; CHANGELOG + `DIAGNOSTIC_CODES.md` updated.

## Out of Scope

- Running evaluators / computing `StatusResult` from state (external HORIZON engine).
- Baseline/evaluator fetch, resolution, or digest verification (feature 14).
- Aggregating dimensions into a single overall vitality number.
- Native `.hpdl` / RON syntax for the extended status surface (features 05 / 04).
- CLI presentation (feature 15).

## References

- Design Doc: `docs/design/09-status-and-vitality.md`
- Existing status work: `.kiro/specs/03-semantic-validation/` (Amendment A1 thresholds),
  `hpdl-model::StatusDeclaration`, `hpdl-sema::status_validator`.
- PIR v2 precedent: `docs/rfcs/0003-pir-v2-orbital-fields.md` (and feature 08 fold-in).
- Steering: workflow (crate status / deferred `hpdl-runtime-api`), language-semantics
  (Amendment A1, statuses derived not written, dimensions independent, absent ≠ zero).
