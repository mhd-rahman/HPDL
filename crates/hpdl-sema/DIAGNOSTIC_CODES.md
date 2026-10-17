# HPDL Semantic Diagnostic Codes

This is the stable reference for every diagnostic code emitted by `hpdl-sema`.
Codes are grouped by series and are **stable**: a code's meaning does not change
once published. New checks get new codes rather than overloading existing ones.

Each diagnostic carries a `code`, a `Severity` (Error / Warning / Info), a
message, an optional source span, and optional notes. Diagnostics are sorted
deterministically (by span, then code, then message) before being returned.

## E1000 — Type validation

| Code | Name | Meaning |
|---|---|---|
| E1001 | `MissingRequiredField` | A required field is absent. |
| E1002 | `InvalidFieldType` | A field has an invalid type or shape. |
| E1003 | `InvalidEnumVariant` | An enum value is not a recognized variant. |

## E2000 — Physical constraints

| Code | Name | Meaning |
|---|---|---|
| E2001 | `NegativeMass` | Mass is zero or otherwise non-physical. |
| E2002 | `InvalidTemperatureRange` | A temperature range is non-physical. |
| E2003 | `InconsistentOrbitalParameters` | Orbital parameters are mutually inconsistent. |
| E2004 | `ContradictoryConstraints` | Two or more constraints contradict (e.g. impossible density). |

## E3000 — Units and dimensions

| Code | Name | Meaning |
|---|---|---|
| E3001 | `MissingUnit` | A quantity is missing its unit. |
| E3002 | `IncompatibleDimensions` | Units have incompatible dimensions. |
| E3003 | `InvalidTemperatureDelta` | `Temperature` used where `TemperatureDelta` expected, or vice versa. |
| E3004 | `UnregisteredCustomDimension` | A custom dimension is used without being registered. |
| E3005 | `InvalidNumericValue` | A numeric value is NaN or infinite. |
| E3006 | `NumericOverflow` | A numeric operation would overflow. |

## E4000 — Status thresholds (Amendment A1)

| Code | Name | Meaning |
|---|---|---|
| E4001 | `UnsortedThresholds` | Threshold cut points are not strictly increasing. |
| E4002 | `DuplicateThreshold` | A threshold value appears more than once. |
| E4003 | `ThresholdOutOfRange` | A threshold is outside the `[min, max]` scale. Out-of-range is rejected, never clamped. |
| E4004 | `InvalidScaleBounds` | `scale_min >= scale_max`. |
| E4005 | `StatusInWriteList` | An action attempts to write to a status field. Statuses are derived, never written. |
| E4006 | `FirstThresholdMismatch` | The first cut point does not equal the scale minimum. |

## E5000 — Actions

| Code | Name | Meaning |
|---|---|---|
| E5001 | `InvalidActionReference` | An action references an invalid or empty target. |
| E5002 | `UndeclaredWrite` | An action writes to a field it did not declare. |

## E6000 — Modules and world laws

| Code | Name | Meaning |
|---|---|---|
| E6001 | `MalformedModuleId` | A module identifier is empty, too long, or uses invalid characters. |
| E6002 | `ConflictingModuleVersions` | A module is referenced more than once / with conflicting versions. Also raised when a world-law module is referenced by two laws within one body. |
| E6003 | `MultipleWorldLawsForDomain` | More than one *authoritative* (replacement) world law claims the same exclusive domain. Composition is intersection, never last-write-wins — the conflict is reported, not silently resolved. |
| E6004 | `MissingMagicBridge` | A magic law reaches a physical (`physics.*`) domain with no declared `MagicBridge`. Magic affects physical state only through a declared bridge. |
| E6005 | `UnsupportedWorldLaw` | A declared world-law / magic module has no registered implementation. **Warning**, not error: the law is accepted as a declaration but never simulated (the module registry is deferred to feature 14). Mirrors E8004 / E9103 / E9202. |

## E7000 — Security and limits

| Code | Name | Meaning |
|---|---|---|
| E7001 | `MaxDepthExceeded` | Input exceeds the maximum nesting depth. |
| E7002 | `MaxCollectionSizeExceeded` | A collection exceeds the maximum size. |
| E7003 | `MaxStringLengthExceeded` | A string exceeds the maximum length. |

## E8000 — Orbital / celestial (feature 07)

| Code | Name | Meaning |
|---|---|---|
| E8001 | `ParentCycle` | A body's declared parent chain contains a cycle (including declaring itself as its own parent). |
| E8002 | `ReferenceFrameMismatch` | An orbit's frame is inconsistent with its parent reference's frame. |
| E8003 | `InvalidOrbitalElement` | A Keplerian element is outside its valid range. |
| E8004 | `UnsupportedOrbitalModel` | An orbital declaration references an unsupported (non-executable) model, such as a custom reference frame. Accepted as a declaration but never simulated. |

## E9100 — Planetary environment (feature 08)

| Code | Name | Meaning |
|---|---|---|
| E9101 | `CompositionOutOfBounds` | A gas composition has a component over 100% or a sum outside the declared normalization tolerance. |
| E9102 | `InvalidEnvironmentValue` | An environment numeric value is invalid: zero atmosphere pressure, inverted temperature ordering, or surface coverage over 100%. |
| E9103 | `UnsupportedEnvironmentModule` | An environment references a module (e.g. a climate model) with no registered implementation. Accepted as a declaration, never simulated. |
| E9104 | `MissingAssetDigest` | An asset reference has a missing or malformed content digest (must be 64 lowercase hex chars). |

## E9200 — Status / vitality (feature 09)

| Code | Name | Meaning |
|---|---|---|
| E9201 | `IncompleteEvaluatorDeclaration` | A status declares an evaluator but is missing a window and/or cut points — it cannot produce a banded result. |
| E9202 | `UnsupportedEvaluator` | A status references an evaluator with no registered implementation. Accepted as a declaration, never run. |

Status-dimension and cut-point rules also reuse the E4000 status block (e.g.
`UnsortedThresholds`, `ThresholdOutOfRange`, `FirstThresholdMismatch`, `InvalidScaleBounds`)
and `StatusInWriteList` (E4005) for the derived-only prohibition.

## E9300 — Action capabilities (feature 10)

| Code | Name | Meaning |
|---|---|---|
| E9301 | `MalformedFieldPath` | An action read/write field path is empty or has an invalid dotted segment. |
| E9302 | `UncoveredWrite` | An action declares a mutating write not backed by any declared capability (declaration-level authorization failure). |

Status writes from actions remain forbidden at `ActionDeclaration` construction and via the
E4000/E5000 status rules — an action can never write a `status.*` field.

## E9000 — Generic

| Code | Name | Meaning |
|---|---|---|
| E9999 | `Generic` | A generic validation error that does not fit another category. |

---

### Notes on behaviour

- **No silent clamping.** Out-of-range status scores and thresholds are rejected
  (E4003), never truncated into range.
- **Statuses are derived.** Any action writing to a `status.*` field is rejected
  (E4005); the model constructor also rejects this at construction time.
- **Custom frames are declarations, not simulations.** A `ReferenceFrame::Custom`
  is accepted structurally but flagged `UnsupportedOrbitalModel` (E8004) so nothing
  claims it is executable. The same applies to environment modules: a declared climate
  model is flagged `UnsupportedEnvironmentModule` (E9103), never simulated.
- **World laws & magic are contracts, never executed.** A declared world law is accepted
  structurally and flagged `UnsupportedWorldLaw` (E6005, warning) — HPDL ships no built-in
  or universal magic and runs no alternative-physics solver. One authoritative law may own
  an exclusive domain (two is `E6003`); magic reaches physical state only through a declared
  bridge (absence is `E6004`).
- **Absent ≠ zero.** A `Biosphere::Absent` or `Biosphere::Unknown` is a valid value and
  produces no diagnostic; absence is never represented as a zero score.
- **Determinism.** Diagnostics are produced in a stable, sorted order and never
  depend on `HashMap` iteration, wall-clock, or locale.
