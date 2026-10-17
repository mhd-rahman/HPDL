# HPDL — HORIZON Planet Definition Language

HPDL is an independent, Rust-native, declarative language for defining persistent
planetary and celestial worlds. A planet definition declares a world's identity,
initial conditions, constraints, processes, statuses, actions, and world-law
contracts. HPDL compiles those definitions into a versioned, canonical **Planetary
Intermediate Representation (PIR)** that the separate HORIZON simulation engine
consumes to evolve mutable state.

HPDL is **not** a game engine, a simulation solver, a CUE wrapper, or a RON dialect.
It defines and compiles worlds deterministically; it never executes them.

## Core principles

- **Definition, not simulation.** A definition is an immutable artifact. A separate
  authoritative engine evolves state. Declaring a domain (an orbit, an atmosphere, a
  world law) is not the same as simulating it — unsupported executable features fail
  explicitly or are labelled non-executable, never faked.
- **Status is derived, never written.** No action or definition may assign a status
  score; statuses are computed by versioned evaluators at runtime.
- **No Earth defaults.** A missing value is *undeclared* (`Absent`/`Unknown`), never a
  silently-assumed Earth value. A missing ecosystem is "not applicable", not "zero".
- **Deterministic & reproducible.** Continuous quantities are scaled integers (no
  floats on the canonical path). The same locked inputs produce byte-identical PIR
  across platforms, and definitions and runtime state are versioned independently.

## Two frontends, one meaning

A planet can be authored in two equivalent syntaxes:

- **Native HPDL** (`.hpdl`) — the dedicated language (grammar **v1.0**, ratified; see
  [`crates/hpdl-syntax/GRAMMAR.md`](crates/hpdl-syntax/GRAMMAR.md) and
  [ADR 0010](docs/rfcs/0010-native-hpdl-grammar-v1.md)).
- **RON** (`.ron`) — a Serde/RON authoring form that mirrors the model types.

Both frontends decode into the same model, run through the same semantic validator,
and lower to the **same canonical PIR bytes**. Equivalent semantics in either syntax
produce an identical content hash — this is a tested guarantee, not an aspiration.

See the equivalent example pair in [`examples/`](examples/): `earth.hpdl` and
`earth.ron` compile to byte-identical PIR.

```hpdl
language hpdl "1.0";

planet earth {
    id: "earth",
    physical: {
        mass_kg: 5972000000000000000000000,  // scaled integers, canonical SI units
        radius_m: 6371000,
        surface_gravity_mms2: 9810,          // milli-m/s²
    },
    traits: { tags: ["terrestrial", "habitable-zone"] },
    identity: Planet,
    parent: ParentRef(target: BodyId("sol"), frame: EclipticJ2000),
    environment: EnvironmentDefinition(
        biosphere: Present(Intelligent),
        atmosphere: AtmosphereProfile(
            surface_pressure_pa: 101325,
            composition: Composition(
                components: [ (GasId("n2"), 780800), (GasId("o2"), 209500), (GasId("ar"), 9700) ],
                tolerance_ppm: 0,
            ),
        ),
    ),
}
```

## Quick start

Build the workspace and the `hpdl` CLI:

```sh
cargo build
```

Compile a definition (the CLI picks the frontend from the file extension — `.hpdl`
→ native, anything else → RON):

```sh
# Validate and report success or diagnostics
cargo run -p hpdl-cli -- check examples/earth.hpdl

# Print a compile summary (language/PIR version, content hash, resolved modules)
cargo run -p hpdl-cli -- inspect examples/earth.ron

# Print the reproducible build manifest (text or --format json)
cargo run -p hpdl-cli -- build examples/earth.hpdl

# Write canonical PIR bytes (hex to stdout, or --out <path> for raw bytes)
cargo run -p hpdl-cli -- export examples/earth.ron --out earth.pir
```

`inspect` on either example prints the same hash, because the two sources are
semantically identical:

```text
source:          earth
pir_version:     2.0
content_hash:    15e4f82a78da73f8728cfb9367605dd8128ee7c0c894305b8d997175f2780e6f
modules:         (none)
```

CLI exit codes: `0` success, `1` compilation/diagnostic failure, `2` usage error.

## Workspace layout

One Cargo workspace; the simulation engine, validator network, and world content
live in separate repositories and consume only the published contract crates.

| Crate | Responsibility |
|---|---|
| `hpdl-model` | Source-independent typed domain model (identity, physical, orbital, environment, status, action, migration, world-law declarations) |
| `hpdl-sema` | The single semantic validator — constraints, dimensions, statuses, world-law checks |
| `hpdl-ron` | RON input adapter → model |
| `hpdl-syntax` | Native `.hpdl` lexer, parser, AST, and model converter |
| `hpdl-pir` | Published canonical PIR types, deterministic encode/decode, strict decoder, domain-separated hashing |
| `hpdl-compiler` | Orchestration: frontend decode → validate → resolve → lower → emit canonical PIR |
| `hpdl-cli` | The `hpdl` developer CLI (`check` / `build` / `inspect` / `export`) |
| `hpdl-runtime-api` | Published runtime/migration **contracts** (state, module, event, migration-execution); the executor is external |

Dependency direction (no cycles):

```text
hpdl-cli → hpdl-compiler → { hpdl-ron, hpdl-syntax } → hpdl-model
                         → hpdl-sema → hpdl-model
                         → hpdl-pir   (leaf)
hpdl-runtime-api → hpdl-pir (+ selected stable model types)
```

## The pipeline

```text
source (.hpdl or .ron)
  → frontend decode     → PlanetDefinition
  → semantic validation → ValidatedDefinition
  → module resolution   → ResolvedModules (offline, lockfile-pinned)
  → representability     → fixed-point lowering check
  → canonical emission  → PIR envelope (+ content hash)
  → manifest            → reproducible BuildManifest
```

Equivalent semantics yield identical PIR bytes regardless of frontend.

## Development

The checks every change must pass (also run in CI):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

Published contract crates additionally run
`cargo semver-checks -p hpdl-pir -p hpdl-runtime-api`.

The pinned toolchain is in [`rust-toolchain.toml`](rust-toolchain.toml). Design
decisions are recorded as ADRs under [`docs/rfcs/`](docs/rfcs/); per-domain design
docs live under [`docs/design/`](docs/design/).

## Current scope

HPDL's v1 architecture spans 16 domains (celestial identity, astronomy, orbits,
classification, geology, atmosphere/climate, hydrosphere, weather, biosphere, assets,
vitality contracts, canonical PIR, status scales, action impact contracts, and world
laws / custom reality). Architectural scope is not the same as implemented solvers:
HPDL declares and validates these domains and compiles them to PIR, but runs no
simulation itself.

Known boundaries worth calling out:

- **World laws / magic / alternative physics** are modelled and validated as
  contracts, but are not yet part of either serialized frontend or the PIR payload
  (coupled to a future PIR format-version decision — see ADR 0005 / ADR 0010).
- Native HPDL grammar v1.0 reserves `\uXXXX` string escapes and non-decimal number
  bases (rejected today).
- Runtime execution, migration execution, and the validator network live in separate
  repositories; `hpdl-runtime-api` here is the contract surface only.

## License

Dual-licensed under MIT or Apache-2.0.
