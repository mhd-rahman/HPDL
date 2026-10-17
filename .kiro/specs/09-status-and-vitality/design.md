# Status and Vitality — Design

Refines `docs/design/09-status-and-vitality.md`. Extends the existing status surface into a
multidimensional, derived, planet-specific contract. Introduces **no** evaluation engine and
**no** evaluator/baseline fetching. Two decisions gate parts of it: OD-09.1 (the deferred
`hpdl-runtime-api` crate) and OD-09.2 (PIR revision). This design specifies the shapes and
flags those gates rather than pre-deciding.

## 1. Crate placement

| Concern | Crate | Module |
|---|---|---|
| `StatusDimension`, `BaselineRef`, `EvaluatorRef`, `CutPoints`, `EvaluationWindow` | `hpdl-model` | `status_vitality.rs` (new) |
| Extended status declaration fields | `hpdl-model` | `status_declaration.rs` (extended) OR a new `StatusSpec` (OD-09.3) |
| New error variants | `hpdl-model` | `error.rs` (extended) |
| Dimension/evaluator/window checks | `hpdl-sema` | extend `status_validator.rs` |
| New diagnostic codes | `hpdl-sema` | `diagnostic.rs` (extended) |
| `StatusResult` runtime contract | **gated by OD-09.1** | `hpdl-runtime-api` (if scaffolded) or documented-only |

`hpdl-sema → hpdl-model` unchanged. No edge into `hpdl-compiler` from a contract crate.

## 2. Types (hpdl-model) — integers only, no f32/f64

### 2.1 StatusDimension
```text
enum StatusDimension { Physical, NativeEcosystem, Civilization, Supernatural, Custom(Name) }
```
Independent dimensions. Absence of a dimension is "not applicable", never a zero score.

### 2.2 EvaluatorRef / BaselineRef
```text
struct EvaluatorRef { module: ModuleRef }               // versioned evaluator identity
struct BaselineRef  { name: Name, digest: Digest }      // planet-specific, content-addressed
```
Reuses `ModuleRef` (feature 01) and `Digest` (feature 08, 64-char lowercase hex).

### 2.3 CutPoints (Amendment A1)
```text
struct CutPoints {
    scale_min: i64,
    scale_max: i64,
    cuts: Vec<(i64, BandLabel)>,   // ordered lower bounds; first == scale_min
}
```
Constructor rejects: `scale_min >= scale_max`; empty; first cut != scale_min; any cut outside
`[min, max]`; non-strictly-increasing cuts. `classify(score) -> Result<&BandLabel, Error>`:
rejects out-of-range (no clamp); band `i` is `[cut_i, cut_{i+1})`, last includes `max`.
This reuses the Amendment A1 logic already proven in `hpdl-sema::status_validator`
(OD-09.4: lift into the model, or have the model call a shared helper).

### 2.4 EvaluationWindow
```text
struct EvaluationWindow { span: u64 /* > 0, ticks or seconds */, scope: Option<Name> }
```

### 2.5 Extended declaration (OD-09.3: extend in place)
`StatusDeclaration` gains optional `dimension`, `baseline`, `evaluator`, `cut_points`,
`window` via builder setters, preserving `new(name, min, max)`.

### 2.6 StatusResult (gated by OD-09.1)
```text
struct StatusResult {
    dimension: StatusDimension,
    score: i64,                 // in [scale_min, scale_max]
    band: BandLabel,
    evaluator_version: String,
    input_hashes: Vec<Digest>,  // deterministic, sorted
    evidence: String,
}
```
Replay invariant: same evaluator version + same input hashes + same window ⇒ identical
`StatusResult`. Placement decided by OD-09.1.

## 3. Errors / Diagnostics

New `hpdl_model::Error`: invalid dimension/evaluator/baseline name; invalid cut-points
(empty / unsorted / duplicate / out-of-range / first-cut mismatch); score out of range;
zero evaluation window.

New / reused `hpdl_sema::DiagnosticCode`: reuse the E4000 status block for cut-point rules
where it already fits (`UnsortedThresholds`, `DuplicateThreshold`, `ThresholdOutOfRange`,
`FirstThresholdMismatch`); add an E9200 block for `IncompleteEvaluatorDeclaration` and
`UnsupportedEvaluator`.

## 4. Validation flow (hpdl-sema)

Extend `StatusValidator` (and wire via the existing `validate_statuses` call):
1. **Cut-points** — Amendment A1 rules → existing E4000 codes.
2. **Evaluator completeness** — evaluator present but window or cut-points missing →
   `IncompleteEvaluatorDeclaration`.
3. **Unsupported evaluator** — declared evaluator with no registered implementation →
   `UnsupportedEvaluator` (no registry yet; OD-09.5).
4. **Dimension independence** — any subset of dimensions is valid; absence is never an error.
5. **Derived-only** — the existing action→status write prohibition (E4005) is unchanged.

No floats; constructors already enforce ranges, so the pass is defensive and never panics.

## 5. Testing

- Model unit tests: cut-point accept/reject + `classify` at 24, 25, 49, 50, boundary, and
  out-of-range (rejected); dimension/evaluator/baseline/window validation.
- **Boundary golden**: freeze the 24.5 / 25 / 49.999 classifications (expressed over the
  integer scale the design implies — e.g. scaled by 10 so 24.5 → 245 if sub-integer bands
  are needed; precision confirmed in OD-09.2/A2) under `tests/golden/`.
- Sema tests: incomplete evaluator, unsupported evaluator, dimension independence, derived-
  only enforcement.
- Golden roundtrip: a fully-declared multidimensional status.
- **Replay test** (gated OD-09.1): identical `StatusResult` for identical versioned inputs.

## 6. Gated decisions

- **OD-09.1** `hpdl-runtime-api`: deferred per steering vs. named by design. Model/sema work
  proceeds regardless; only `StatusResult` placement waits.
- **OD-09.2** PIR revision: fold into unreleased v2.0 or bump v3.0, only if the extended
  declaration is encoded into PIR. Recorded as an ADR; golden vectors updated deliberately.

## 7. What this design deliberately does NOT do

- No evaluation/aggregation engine; no evaluator/baseline fetching or digest verification.
- No single combined "overall vitality" score.
- No float arithmetic; no authored status scores.
- No scaffolding of a deferred crate without explicit approval (OD-09.1).
