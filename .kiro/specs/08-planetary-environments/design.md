# Planetary Environments — Design

This spec-level design refines `docs/design/08-planetary-environments.md` into concrete
types, placement, and validation flow. It mirrors feature 07's structure. It introduces
**no** simulation and **no** asset fetching. It does change canonical PIR bytes (see §6),
which is an explicit, approved decision — not a silent one.

## 1. Crate placement

| Concern | Crate | Module |
|---|---|---|
| `EnvironmentDefinition`, `AtmosphereProfile`, `Composition`, `ThermalProfile`, `Hydrosphere`, `Cryosphere`, `Biosphere`, `AssetRef`, `GasId`, `ComplexityTier`, `AssetKind` | `hpdl-model` | `environment.rs` (new) |
| `environment` field on the body | `hpdl-model` | `celestial_body.rs` (extended) |
| New error variants | `hpdl-model` | `error.rs` (extended) |
| Environment validation pass | `hpdl-sema` | `environment_validator.rs` (new) |
| New diagnostic codes | `hpdl-sema` | `diagnostic.rs` (extended) |
| Wire pass into the one validator | `hpdl-sema` | `validator.rs` (extended) |

Dependency direction is unchanged: `hpdl-sema → hpdl-model`.

## 2. Types (hpdl-model) — scaled integers only, no f32/f64

### 2.1 GasId / name newtypes
```text
GasId(String)        // 1..=32, [A-Za-z0-9_\-.]  (e.g. "n2", "co2", "arcane.aether")
```
Reuses the feature-07 validated-name helper pattern (`Error::InvalidEnvironmentName`).

### 2.2 Composition
```text
struct Composition {
    components: BTreeMap<GasId, u32>,   // mole fraction in ppm (fraction * 1e6)
    tolerance_ppm: u32,                 // allowed deviation of the sum from 1_000_000
}
```
Constructor rejects: empty map; any component `> 1_000_000`; a `checked` sum (u64
accumulator) outside `[1_000_000 - tol, 1_000_000 + tol]`.

### 2.3 AtmosphereProfile
```text
struct AtmosphereProfile {
    surface_pressure_pa: u64,           // > 0
    composition: Composition,
    climate_model: Option<ModuleRef>,   // contract reference; not executed
}
```

### 2.4 ThermalProfile
```text
struct ThermalProfile { mean_mk: u64, min_mk: u64, max_mk: u64 }  // milli-kelvin
```
Constructor rejects unless `min_mk <= mean_mk <= max_mk`.

### 2.5 Hydrosphere / Cryosphere
```text
struct Hydrosphere { surface_coverage_ppm: u32 /*0..=1_000_000*/, dominant_liquid: Option<GasId> }
struct Cryosphere  { surface_coverage_ppm: u32 /*0..=1_000_000*/ }
```

### 2.6 Biosphere (absent ≠ zero)
```text
enum ComplexityTier { Microbial, Simple, Complex, Intelligent, Other(Name) }
enum Biosphere { Absent, Unknown, Present(ComplexityTier) }
```

### 2.7 AssetRef (content-addressed)
```text
enum AssetKind { Texture, Mesh, Heightmap, Other(Name) }
struct AssetRef { name: Name, kind: AssetKind, digest: Digest }
Digest = lowercase hex SHA-256, exactly 64 hex chars (validated).
```

### 2.8 EnvironmentDefinition
```text
struct EnvironmentDefinition {
    atmosphere: Option<AtmosphereProfile>,
    thermal:    Option<ThermalProfile>,
    hydrosphere: Option<Hydrosphere>,
    cryosphere:  Option<Cryosphere>,
    biosphere:   Biosphere,                 // tri-state, not Option
    assets:      BTreeMap<String, AssetRef>, // keyed by logical name (dedup + order)
}
```

### 2.9 CelestialBody extension
`CelestialBody` gains `environment: Option<EnvironmentDefinition>` with a
`with_environment(..)` builder setter. Absent = undeclared.

## 3. Errors (hpdl-model) and Diagnostics (hpdl-sema)

New `hpdl_model::Error` variants (typed, `thiserror`): invalid environment name; empty
composition; component fraction out of range; composition sum out of tolerance; zero
pressure; inverted temperature ordering; coverage out of range; malformed asset digest.

New `hpdl_sema::DiagnosticCode` (E9100 "environment" block — chosen above the E9000 generic
code but distinct; final codes decided in Task 6):
- `CompositionOutOfBounds`
- `InvalidEnvironmentValue`   (zero pressure / inverted temperature / out-of-range coverage)
- `UnsupportedEnvironmentModule`
- `MissingAssetDigest`

## 4. Validation flow (hpdl-sema)

`Validator::validate` gains one call: `self.validate_environment(&definition)`, merged
before the final sort. The pass, when `environment` is present:

1. **Composition** — re-verify sum within tolerance and each component `<= 1e6` →
   `CompositionOutOfBounds`.
2. **Atmosphere values** — pressure `> 0`; if a `climate_model` is declared, flag it
   `UnsupportedEnvironmentModule` (no registry yet, OD-08.4) → explicit non-executable.
3. **Thermal** — `min <= mean <= max` → `InvalidEnvironmentValue`.
4. **Hydrosphere/Cryosphere** — coverage `<= 1e6` → `InvalidEnvironmentValue`.
5. **Assets** — each digest well-formed (64 lowercase hex) → `MissingAssetDigest`.
6. **Biosphere** — `Absent`/`Unknown`/`Present` are all valid; no diagnostic. (The point of
   the "absent life" test is that absence is a value, not an error and not a zero.)

No float arithmetic anywhere. Constructors already enforce these, so the pass is defensive
and never panics.

## 5. Testing

- **Model unit tests**: constructor accept/reject per type and boundary (composition sum at
  tolerance edges; pressure `0`; temperature inversion; coverage `1_000_001`; digest wrong
  length / non-hex / uppercase; biosphere tri-state round-trips).
- **Sema unit tests**: each diagnostic branch + a fully-valid environment passing clean +
  an `Absent` biosphere producing no diagnostic.
- **Golden fixture**: `hpdl-model/tests/golden/environment_body.json` frozen and
  roundtripped via the existing mirror/real-type roundtrip harness.
- **Cross-platform**: integers + stable enum tags only → identical bytes across OSes.

## 6. PIR format revision (OD-08.1 — DECISION REQUIRED)

Adding the `environment` field changes canonical PIR bytes. PIR is at v2.0 (feature 07),
currently unreleased. The chosen option (fold into v2.0, or bump to v3.0) is recorded in a
new ADR under `docs/rfcs/` before the field is encoded, and the compiler/PIR golden vectors
are updated deliberately. This design does not pre-decide it; Task 9 executes whichever
option is approved.

## 7. What this design deliberately does NOT do

- No climate/chemistry/ecosystem simulation.
- No asset fetching, decoding, or digest verification (only format validation).
- No status/score derivation from atmosphere components.
- No new frontend syntax.
- No `f32`/`f64` anywhere.
