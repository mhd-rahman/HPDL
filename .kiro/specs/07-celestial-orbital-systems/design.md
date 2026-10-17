# Celestial and Orbital Systems — Design

This spec-level design refines the engineering design in
`docs/design/07-celestial-orbital-systems.md` into concrete types, placement, and
validation flow. It introduces **no** PIR encoding change and **no** simulation.

## 1. Crate placement

| Concern | Crate | Module |
|---|---|---|
| `CelestialIdentity`, `ParentRef`, `ReferenceFrame`, `KeplerianElements`, `RotationModel` | `hpdl-model` | `orbital.rs` (new) |
| Orbital fields on the body | `hpdl-model` | `celestial_body.rs` (extended) |
| New error variants | `hpdl-model` | `error.rs` (extended) |
| Orbital validation pass | `hpdl-sema` | `orbital_validator.rs` (new) |
| New diagnostic codes | `hpdl-sema` | `diagnostic.rs` (extended) |
| Wire pass into the one validator | `hpdl-sema` | `validator.rs` (extended) |

Dependency direction is unchanged: `hpdl-sema → hpdl-model`. No edge to `hpdl-pir`,
`hpdl-compiler`, or `hpdl-cli` is added.

## 2. Types (hpdl-model)

### 2.1 ReferenceFrame
```text
enum ReferenceFrame {
    EquatorialJ2000,
    EclipticJ2000,
    BodyFixed,
    ParentCentered,
    Custom(FrameName),   // validated 1..=64, [A-Za-z0-9_\-.]
}
```
`Custom` holds a validated name newtype. A `Custom` frame is a *declaration*; the
validator treats it as non-executable (FR6.4).

### 2.2 CelestialIdentity
```text
enum CelestialIdentity {
    Star, Planet, DwarfPlanet, Moon, Asteroid, Comet, Barycenter,
    Other(KindName),     // validated 1..=64, [A-Za-z0-9_\-.]
}
```

### 2.3 KeplerianElements (scaled integers only — no f32/f64)
```text
struct KeplerianElements {
    semi_major_axis_m: u128,          // > 0, metres (SI)
    eccentricity_micro: u32,          // e * 1e6, in [0, 1_000_000)  => e in [0,1)
    inclination_mdeg: i32,            // [0, 180_000]
    lon_ascending_node_mdeg: i32,     // [0, 360_000)
    arg_periapsis_mdeg: i32,          // [0, 360_000)
    mean_anomaly_epoch_mdeg: i32,     // [0, 360_000)
    frame: ReferenceFrame,            // the frame these elements are expressed in
}
```
Constructor rejects out-of-range values with typed `Error`; parabolic/hyperbolic
(`e >= 1`) rejected (OD-07.3). All checks use `checked_*` where arithmetic is involved.

### 2.4 ParentRef
```text
struct ParentRef {
    target: BodyId,          // the gravitational parent
    frame: ReferenceFrame,   // frame the child's orbit is expressed in
}
```
Self-parent (`target == owning body id`) is not detectable at construction (the owning id
is not known to the `ParentRef`), so it is enforced in the validator (FR6.2).

### 2.5 RotationModel
```text
struct RotationModel {
    sidereal_period_s: i64,  // != 0; negative = retrograde
    axial_tilt_mdeg: i32,    // [0, 180_000]
    frame: ReferenceFrame,
}
```

### 2.6 CelestialBody extension
`CelestialBody` gains four `Option` fields: `identity`, `parent`, `orbit`, `rotation`.
A builder-style setter or an extended constructor keeps construction non-panicking and
preserves the existing uniqueness checks. Absent = undeclared (no Earth default).

## 3. Errors (hpdl-model) and Diagnostics (hpdl-sema)

New `hpdl_model::Error` variants (typed, `thiserror`): invalid frame/kind name,
non-positive semi-major axis, eccentricity out of `[0,1)`, angle out of range,
zero sidereal period.

New `hpdl_sema::DiagnosticCode` variants (E8000 orbital series):
- `ParentCycle` (E8001)
- `ReferenceFrameMismatch` (E8002)
- `InvalidOrbitalElement` (E8003)
- `UnsupportedOrbitalModel` (E8004)

Codes are additive and stable; `Diagnostic` ordering is unchanged.

## 4. Validation flow (hpdl-sema)

`Validator::validate` gains one call: `self.validate_orbital(&definition)`, merged into
the existing diagnostics vector before the final sort. The pass:

1. **Parent cycle** — if `parent.target == body.id` → `ParentCycle`. The detector is a
   generic reachable-set walk bounded by the number of declared bodies (currently one),
   so it generalizes to a DAG check without rework.
2. **Frame consistency** — if both `parent` and `orbit` are present, require
   `orbit.frame == parent.frame` or `orbit.frame == ParentCentered`; else
   `ReferenceFrameMismatch`.
3. **Unsupported model** — if any referenced frame is `Custom(..)` → `UnsupportedOrbitalModel`
   (Error): a declaration is allowed, but it is explicitly non-executable; nothing claims
   simulation.
4. **Defensive element check** — re-verify orbital ranges; any violation →
   `InvalidOrbitalElement`. (Constructors already enforce these, so this is defense in
   depth and never panics.)

No float arithmetic is used anywhere in this pass.

## 5. Testing

- **Model unit tests**: constructor accept/reject for each type and boundary (e.g.
  `e = 999_999` ok, `e = 1_000_000` rejected; inclination `180_000` ok, `180_001`
  rejected; sidereal period `0` rejected, negative ok).
- **Sema unit tests**: parent cycle, frame mismatch, custom-frame unsupported, defensive
  element error, and a fully-valid orbital body that passes clean.
- **Golden fixture**: `hpdl-model/tests/golden/orbital_body.json` frozen and roundtripped
  via the existing mirror-based roundtrip harness.
- **Cross-platform**: serialized form uses only integers and stable enum tags, so bytes
  are identical across OSes.

## 6. PIR format bump (ADR 0003)

The orbital fields are added to the bincode-encoded `CelestialBody`, which changes
canonical PIR bytes for every definition. This is handled as a deliberate breaking format
change: `PirVersion::CURRENT` 1.0 → 2.0, hash domain separator `HPDL-PIR-v1` → `HPDL-PIR-v2`,
`hpdl-pir` crate 0.1.0 → 0.2.0, and the frozen compiler golden fixture updated deliberately.
See `docs/rfcs/0003-pir-v2-orbital-fields.md`. No new per-field PIR schema is introduced;
the existing encoder serializes the new fields.

## 7. What this design deliberately does NOT do

- Does not define a new tighter per-field scaled-integer PIR schema (the existing bincode
  encoder is reused; a tagged/compact encoding is future work).
- Does not propagate, integrate, or evolve any orbit or orientation.
- Does not add frontend syntax.
- Does not introduce `f32`/`f64` anywhere.
