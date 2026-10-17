# Requirements — Planetary Domain Model

> **Feature:** `01-planetary-domain-model`  
> **Design doc:** `docs/design/01-planetary-domain-model.md`  
> **Crate:** `hpdl-model`  
> **Status:** ✅ Complete

---

## Overview

Define the typed, immutable Rust structs that represent HPDL declarations for
celestial bodies and planetary subsystems. This crate is the dependency root —
it contains no parsing, no semantic validation, and no PIR encoding. Every
value it holds is a *declaration*, never mutable runtime state.

---

## Functional Requirements

### REQ-01 — `BodyId` newtype

A strongly-typed, validated identifier for celestial bodies.

**Acceptance criteria:**
- `BodyId` is a newtype wrapping a non-empty, validated string (alphanumeric + `-` + `_`, max 128 chars).
- `BodyId::new` returns `Result<BodyId, Error>` — never panics.
- `BodyId` implements `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`, `Display`.
- Two `BodyId` values with the same string are equal.
- An empty string, a string exceeding 128 chars, or a string containing invalid characters returns `Err(Error::InvalidId)`.

---

### REQ-02 — `PhysicalProperties` struct

Physical constants for a celestial body, all using exact-decimal or integer representations.

**Acceptance criteria:**
- Fields: `mass_kg: Option<u128>`, `radius_m: Option<u128>`, `surface_gravity_mms2: Option<u64>` (milli-m/s²), `orbital_period_s: Option<u64>`, `rotation_period_s: Option<i64>` (negative = retrograde).
- No `f32` or `f64` fields anywhere in the struct or its transitively owned types.
- All fields are `Option` — absent values must be representable without implying an Earth default.
- `PhysicalProperties` implements `Clone`, `Debug`, `PartialEq`, `Eq`, `Default` (all `None`).
- `PhysicalProperties::default()` produces all-`None` (no Earth-like defaults).

---

### REQ-03 — `TraitSet` — body trait declarations

An ordered, deduplicated set of named traits applied to a body.

**Acceptance criteria:**
- `TraitSet` wraps a `BTreeSet<String>` (ordered, no duplicates, deterministic iteration).
- `TraitSet::new(items: impl IntoIterator<Item=String>) -> Result<TraitSet, Error>` validates each trait name (non-empty, ≤ 64 chars, alphanumeric + `-` + `_` + `.`).
- An invalid trait name returns `Err(Error::InvalidTraitName)`.
- `TraitSet` implements `Clone`, `Debug`, `PartialEq`, `Eq`.
- Iteration order is lexicographic (BTreeSet guarantee).

---

### REQ-04 — `ModuleRef` — versioned module reference

A reference to a versioned external module by name and semver-compatible version string.

**Acceptance criteria:**
- `ModuleRef { name: String, version: String }` — both non-empty.
- `ModuleRef::new(name, version) -> Result<ModuleRef, Error>` validates both fields: non-empty, name ≤ 128 chars, version ≤ 64 chars.
- `ModuleRef` implements `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`.
- An empty name or version returns `Err(Error::InvalidModuleRef)`.

---

### REQ-05 — `StatusDeclaration` — status field declaration

A declaration that a named status exists, with its scale bounds. Does **not** hold a runtime value.

**Acceptance criteria:**
- `StatusDeclaration { name: String, scale_min: i64, scale_max: i64 }`.
- `StatusDeclaration::new(name, scale_min, scale_max) -> Result<StatusDeclaration, Error>`.
- `scale_min` must be strictly less than `scale_max`; otherwise `Err(Error::InvalidScale)`.
- Name validation: non-empty, ≤ 128 chars, alphanumeric + `-` + `_`.
- `StatusDeclaration` implements `Clone`, `Debug`, `PartialEq`, `Eq`.

---

### REQ-06 — `ActionDeclaration` — action field declaration

Declares that a named action exists with explicit read and write sets. Does not execute.

**Acceptance criteria:**
- `ActionDeclaration { name: String, reads: BTreeSet<String>, writes: BTreeSet<String> }`.
- `ActionDeclaration::new(name, reads, writes) -> Result<ActionDeclaration, Error>`.
- `writes` must not contain any string that begins with `"status."` — writing to a status is forbidden at the declaration level; returns `Err(Error::StatusWriteForbidden)`.
- Name validation: non-empty, ≤ 128 chars.
- `ActionDeclaration` implements `Clone`, `Debug`, `PartialEq`, `Eq`.

---

### REQ-07 — `MigrationDeclaration` — schema migration declaration

Declares that a named forward-only schema migration exists.

**Acceptance criteria:**
- `MigrationDeclaration { name: String, from_version: u32, to_version: u32 }`.
- `MigrationDeclaration::new(name, from, to) -> Result<MigrationDeclaration, Error>`.
- `to_version` must be strictly greater than `from_version`; otherwise `Err(Error::InvalidMigrationVersions)`.
- Name validation: non-empty, ≤ 128 chars.
- `MigrationDeclaration` implements `Clone`, `Debug`, `PartialEq`, `Eq`.

---

### REQ-08 — `CelestialBody` struct

A complete, immutable declaration of a single celestial body.

**Acceptance criteria:**
- Fields: `id: BodyId`, `properties: PhysicalProperties`, `traits: TraitSet`, `modules: Vec<ModuleRef>`, `statuses: Vec<StatusDeclaration>`, `actions: Vec<ActionDeclaration>`, `migrations: Vec<MigrationDeclaration>`.
- `CelestialBody::new(id, properties, traits, modules, statuses, actions, migrations) -> Result<CelestialBody, Error>`.
- Action names within a single body must be unique; duplicate returns `Err(Error::DuplicateActionName)`.
- Status names within a single body must be unique; duplicate returns `Err(Error::DuplicateStatusName)`.
- `CelestialBody` implements `Clone`, `Debug`, `PartialEq`, `Eq`.

---

### REQ-09 — `PlanetDefinition` — top-level declaration

The root type representing a complete planet file.

**Acceptance criteria:**
- Fields: `language_version: String`, `body: CelestialBody`.
- `PlanetDefinition::new(language_version, body) -> Result<PlanetDefinition, Error>`.
- `language_version` must be non-empty and ≤ 32 chars; otherwise `Err(Error::InvalidLanguageVersion)`.
- `PlanetDefinition` implements `Clone`, `Debug`, `PartialEq`, `Eq`.

---

### REQ-10 — `Error` enum — typed, exhaustive error type

All fallible constructors return a single `Error` enum with typed variants.

**Acceptance criteria:**
- `Error` is derived with `thiserror::Error`.
- Variants cover: `InvalidId`, `InvalidTraitName`, `InvalidModuleRef`, `InvalidScale`, `StatusWriteForbidden`, `InvalidMigrationVersions`, `DuplicateActionName`, `DuplicateStatusName`, `InvalidLanguageVersion`.
- Every variant has a human-readable `#[error("...")]` message.
- `Error` implements `Debug`, `Display`, `std::error::Error`, `PartialEq`, `Eq`.

---

### REQ-11 — No `f32`/`f64`, no panics, no Earth defaults

Cross-cutting constraint applying to the entire crate.

**Acceptance criteria:**
- No `f32` or `f64` in any type in the crate.
- No `unwrap()`, `expect()`, `panic!()`, or unguarded slice indexing in any production code path.
- No field has a default value that implies Earth-specific physics.
- `#![deny(clippy::float_arithmetic)]` and `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` are present and the crate compiles clean under them.

---

### REQ-12 — Golden roundtrip tests

**Acceptance criteria:**
- At least one golden fixture file under `tests/golden/` contains a serialized `PlanetDefinition` (using `serde_json` or similar) that must roundtrip without change.
- Tests cover: valid construction, each `Err` variant, boundary values (empty string, max-length string, min/max scale, retrograde rotation).
- All tests pass on Linux, macOS, and Windows.

---

## Non-Functional Requirements

| ID | Requirement |
|---|---|
| NFR-01 | `hpdl-model` has zero workspace dependencies other than `thiserror`. |
| NFR-02 | No `f32`/`f64` anywhere in the crate. |
| NFR-03 | `cargo doc -p hpdl-model --no-deps` produces no warnings. |
| NFR-04 | Every public struct and enum has a rustdoc comment. |
| NFR-05 | `BTreeMap`/`BTreeSet` used wherever deterministic order matters; no `HashMap`/`HashSet` for canonical data. |

---

## Out of Scope

- Parsing `.hpdl` or `.ron` source (features 04, 05).
- Semantic validation rules (feature 03).
- PIR encoding (feature 02).
- Runtime state or mutable simulation values.
- Earth-specific or Solar System constants — no hardcoded defaults.

---

## Open Decisions (require explicit approval)

| # | Decision |
|---|---|
| OD-06 | Exact integer representation for mass/radius (u128 assumed; confirm precision requirements). |
| OD-07 | Whether `serde` derives are added to model types now or deferred to the RON frontend feature. |
| OD-08 | Trait name namespace format (dot-separated namespaces assumed; confirm). |
