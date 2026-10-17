# Tasks — Planetary Domain Model

> **Feature:** `01-planetary-domain-model`  
> **Requirements:** `.kiro/specs/01-planetary-domain-model/requirements.md`  
> **Design doc:** `docs/design/01-planetary-domain-model.md`  
> **Crate:** `hpdl-model`  
> **Status:** ✅ Complete — all checks pass

---

### TASK-01 — `Error` enum
**Req:** REQ-10

- [x] `src/error.rs` — all variants: `InvalidId`, `InvalidTraitName`, `InvalidModuleRef`, `InvalidScale`, `StatusWriteForbidden`, `InvalidMigrationVersions`, `DuplicateActionName`, `DuplicateStatusName`, `InvalidLanguageVersion`, `InvalidName`.
- [x] Derived with `thiserror::Error`, `Debug`, `Clone`, `PartialEq`, `Eq`.
- [x] Re-exported from `lib.rs`.

---

### TASK-02 — `BodyId` newtype
**Req:** REQ-01

- [x] `src/body_id.rs` — validates charset `[A-Za-z0-9_-]` and 1–128 char length.
- [x] `BodyId::new` returns `Result<BodyId, Error::InvalidId>`.
- [x] Implements `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`, `Display`.
- [x] Unit tests: valid, empty, too long, invalid chars, equality, ordering, display.

---

### TASK-03 — `PhysicalProperties`
**Req:** REQ-02

- [x] `src/physical_properties.rs` — all fields `Option<integer>`, no floats.
- [x] `Default` impl produces all-`None`.
- [x] Unit tests: default all-None, retrograde rotation, partial declaration.

---

### TASK-04 — `TraitSet`
**Req:** REQ-03

- [x] `src/trait_set.rs` — wraps `BTreeSet<String>`, validates each name.
- [x] Accepts dot-separated namespaced names (`[A-Za-z0-9_\-.]`, 1–64 chars).
- [x] Unit tests: valid, duplicates deduplicated, lexicographic order, invalid name, empty name, too long.

---

### TASK-05 — `ModuleRef`
**Req:** REQ-04

- [x] `src/module_ref.rs` — name (1–128) and version (1–64) validated.
- [x] Unit tests: valid, empty name, empty version, name too long, version too long.

---

### TASK-06 — `StatusDeclaration`
**Req:** REQ-05

- [x] `src/status_declaration.rs` — validates name and `scale_min < scale_max`.
- [x] Unit tests: valid, equal min/max, inverted, negative scale, empty name.

---

### TASK-07 — `ActionDeclaration`
**Req:** REQ-06

- [x] `src/action_declaration.rs` — rejects any `writes` entry starting with `"status."`.
- [x] Unit tests: valid, status write rejected, empty read/write sets, empty name, `"statusfield"` (no dot) allowed.

---

### TASK-08 — `MigrationDeclaration`
**Req:** REQ-07

- [x] `src/migration_declaration.rs` — validates `to_version > from_version`.
- [x] Unit tests: valid, equal versions, downgrade, empty name, large jump.

---

### TASK-09 — `CelestialBody`
**Req:** REQ-08

- [x] `src/celestial_body.rs` — unique action names, unique status names enforced.
- [x] Unit tests: valid minimal body, duplicate action rejected, duplicate status rejected, distinct names ok.

---

### TASK-10 — `PlanetDefinition`
**Req:** REQ-09

- [x] `src/planet_definition.rs` — validates `language_version` 1–32 chars.
- [x] Unit tests: valid, empty version, too-long version, max-length version.

---

### TASK-11 — Golden fixture + roundtrip test
**Req:** REQ-12

- [x] `tests/golden/minimal_planet.json` — fixture for a minimal valid `PlanetDefinition`.
- [x] `tests/roundtrip.rs` — deserializes, validates through model constructors, re-serializes, asserts byte-for-byte match.
- [x] `serde` + `serde_json` added as dev-dependencies (OD-07: production serde deferred).
- [x] `cargo test -p hpdl-model` passes — 46 tests (44 unit + 2 integration).

---

### TASK-12 — Final lint and doc pass
**Req:** REQ-11, NFR-03, NFR-04

- [x] `lib.rs` has `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]`.
- [x] `lib.rs` has `#![deny(clippy::float_arithmetic)]`.
- [x] Every public type has a rustdoc comment with an example.
- [x] `workflow.md` updated: `hpdl-model` → `active (feature 01)`.
- [x] `cargo clippy -p hpdl-model -- -D warnings` passes — zero warnings.
