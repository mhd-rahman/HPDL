# hpdl-sema

The **one** shared semantic validator for HPDL. Every frontend (the RON adapter
`hpdl-ron`, the native parser `hpdl-syntax`) routes its parsed
`hpdl_model::PlanetDefinition` through this crate, so validation rules are defined
exactly once and all frontends agree.

## What it validates

The `Validator` runs a fixed set of passes and merges their diagnostics:

- **Type checker** — field shapes, required fields, enum variants.
- **Constraint solver** — physical constraints and cross-field consistency.
- **Unit registry** — numeric validity (no NaN/inf) and custom-dimension registration.
- **Status validator** — Amendment A1 threshold rules; enforces that statuses are
  derived, never written by actions.
- **Action validator** — action `reads`/`writes` well-formedness.
- **Module validator** — module id/version well-formedness and duplicate detection.
- **Orbital validator** (feature 07) — parent-graph acyclicity, reference-frame
  consistency, Keplerian element ranges, and flagging of unsupported custom frames.

## Usage

```rust
use hpdl_sema::{Validator, ValidatorConfig};
use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};

let body = CelestialBody::new(
    BodyId::new("earth").unwrap(),
    PhysicalProperties::default(),
    TraitSet::empty(),
    vec![], vec![], vec![], vec![],
).unwrap();
let definition = PlanetDefinition::new("1.0", body).unwrap();

let validator = Validator::new(ValidatorConfig::default());
match validator.validate(definition) {
    Ok(validated) => { /* use validated.definition() */ }
    Err(diagnostics) => {
        for d in diagnostics {
            eprintln!("{d}");
        }
    }
}
```

See the runnable examples:

```sh
cargo run -p hpdl-sema --example basic_validation
cargo run -p hpdl-sema --example error_handling
```

## Diagnostics

All diagnostics carry a stable code (e.g. `E8001`), a severity, a message, an
optional source span, and notes. They are returned in a deterministic sorted
order. The full catalogue is in [`DIAGNOSTIC_CODES.md`](./DIAGNOSTIC_CODES.md).

## Configuration

`ValidatorConfig` carries security limits (`max_nesting_depth`,
`max_collection_size`, `max_string_length`) and a `strict_mode` flag that promotes
warnings to errors. `ValidatorConfig::default()` is strict about structure and
deterministic (no ambient configuration is read).

## Guarantees

- No panics on untrusted input; collections and strings are bounded by the config.
- Deterministic output: no `HashMap` iteration into results, no wall-clock, locale,
  or RNG. Identical input yields byte-identical diagnostics.
- `#![forbid(unsafe_code)]`, and `unwrap`/`expect`/panicking-index are denied in
  non-test code.

## Dependencies

Depends only on `hpdl-model` (and `thiserror`). It must **not** depend on
`hpdl-pir`, `hpdl-compiler`, or `hpdl-cli`.
