# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for published crates (`hpdl-pir`, and others as they stabilize).

---

## [Unreleased] — PIR format v2.0 ratified / locked (ADR 0014) (2026-10-19)

### Changed — PIR format status (no bytes, no code, no crate-version change)

- **PIR format v2.0 is now final / locked.** Declared by
  `docs/rfcs/0014-pir-v2-ratified.md`. `PirVersion::CURRENT` stays `2.0`, the hash
  domain separator stays `HPDL-PIR-v2`, and `hpdl-pir` stays `0.2.0` — this is a
  governance freeze, not a byte or API change.
- **The "fold into unreleased v2.0" allowance (ADR 0003 amendment, OD-08.1) is
  closed.** Any further change to the canonical byte layout now requires a **v3.0**
  major bump, a new ADR, a migration path, and a reviewed golden-vector update, per
  ADR 0013. No more folding into v2.0.
- **Frozen golden vectors are the v2.0 contract:** `pir_vectors.json`
  (`minimal=608a90a7…`, `traits=9bc40d4e…`), `minimal_build.txt`
  (`content_hash=23d96f5d…`), and `digest_parity.txt` (`minimal`, `traits`,
  `environment_frontends=438303d9…`, `locked_closure=9ed3b771…`). These are
  deliberately **unchanged** — the correct outcome for a freeze. Frozen-vector
  comments updated to note the lock.

### v1 → v2 compatibility (confirmed + newly tested)

- A v2 decoder **rejects** v1 bytes — the intended hard break, no migration. Enforced
  two independent ways: the explicit version gate
  (`PirVersion::CURRENT.is_compatible` → `Error::IncompatibleVersion`) and the
  structural bincode layout change (v1 payload cannot deserialize into the v2
  struct → `Error::MalformedData`). Digests differ across majors by construction
  (separator bound to the major version).
- **Added** `crates/hpdl-pir/tests/v1_rejection.rs` — end-to-end tests that drive a
  real v1-stamped envelope (valid content hash) through `decode_planet_definition`
  and assert `Error::IncompatibleVersion`, including the case where a v2-shaped
  payload carries a v1 version stamp (rejected before payload decode). This closes
  the gap where v1 rejection was previously proven only via the `is_compatible`
  helper and a forced-`{99,0}` decode.

---

## [Unreleased] — Feature 14: Module registry (existence / kind / integrity) (2026-10-18)

Adds the compile-time module registry that backs the previously-unconditional
"no registered implementation" flags, and closes two silent gaps the audit found
(magic-bridge modules and baseline artifacts were never resolved). Recorded in
`docs/rfcs/0011-module-registry.md`. The registry resolves and verifies; it never
executes. **No PIR change.**

### Added — `hpdl-model`

- `ModuleKind` enum (`Evaluator`, `ClimateModel`, `WorldLaw`, `MagicBridge`,
  `Library`, `Baseline`) and `RegisteredModule` descriptor (`PackageId` + kind +
  `Digest`). Pure declarations; resolve nothing, execute nothing.

### Added — `hpdl-sema`

- A `ModuleResolver` trait + `ResolverError` (`NotFound` / `KindMismatch` /
  `IntegrityMismatch`) — the seam the single validator consults. `Validator` gains
  `with_resolver(..)`; **no resolver = exact legacy behavior** (references flagged
  non-executable). sema keeps its model-only dependency.
- Two new stable diagnostic codes: `E6006 UnsupportedMagicBridge` and
  `E9203 UnsupportedBaseline`.
- The world-law (`E6005`), magic-bridge (`E6006`), evaluator (`E9202`),
  climate-model (`E9103`), and baseline (`E9203`) checks now resolve against a
  supplied registry: registered + correct kind + matching digest → clean;
  otherwise the same code, now a genuine resolution error. With no registry,
  world-law/bridge stay non-blocking warnings and the others stay errors —
  unchanged defaults.

### Added — `hpdl-compiler`

- `ModuleRegistry` implementing `hpdl_sema::ModuleResolver`: an offline,
  deterministic index verifying **existence, kind, and integrity** (digest match,
  reusing `hpdl_model::Digest`). No I/O, no fetch, no execution.
- `CompileOptions::with_registry(..)`; the validator is attached to the registry
  during compilation. Re-exports `ModuleKind`/`RegisteredModule`/`Digest` for
  downstream registry construction.
- `tests/registry.rs`: an evaluator fails without a registry, compiles with a
  matching one, fails with a non-matching one, and a registry does **not** change
  canonical PIR bytes.

### Scope and deferrals (ADR 0011)

- **Full set** resolved: evaluator, climate-model, world-law, magic-bridge,
  library modules, and baseline artifacts.
- **Deferred — authenticity / trust roots (ADR 0008 OD-14.4):** the registry
  verifies integrity, existence, and kind only; *who signed* a module and *which
  registries/trust roots are trusted* is left as a surfaced, un-coded seam pending
  a dedicated ADR.
- **Out of registry scope — custom orbital reference frame (`E8004`):** a custom
  `ReferenceFrame` is a frame variant, not a `ModuleRef`, so it stays a
  non-executable frame-variant flag, unchanged.

### Compatibility

- Additive public types across `hpdl-model` / `hpdl-sema` / `hpdl-compiler`;
  existing `Validator` / `CompileOptions` constructors unchanged (resolver/registry
  default to `None`). **No PIR format change**; `PirVersion::CURRENT` and golden
  vectors untouched (asserted by `registry_does_not_change_pir_bytes`).

---

## [Unreleased] — Feature 05: Native HPDL grammar v1.0 ratified (2026-10-18)

Ratifies the native `.hpdl` grammar at **full parity** with the RON frontend
(scope A2) and lifts its "illustrative / not-yet-ratified" status. Recorded in
`docs/rfcs/0010-native-hpdl-grammar-v1.md`.

### Added — `hpdl-syntax` crate

- **Variant/newtype value syntax** (`ValueNode::Variant`) and **anonymous tuples**
  (`ValueNode::Tuple`), mirroring RON: `Planet`, `Custom("x")`, `Present(Microbial)`,
  `ModuleRef(name:, version:)`, `None`/`Some(x)`, and pairs `(25, "degraded")`. No new
  lexer tokens — `(`/`)` already lexed; this is a pure parser/AST addition.
- **Full converter wiring** (`convert.rs`): native HPDL now extracts `modules`,
  `statuses` (incl. `dimension`/`baseline`/`evaluator`/`cut_points`/`window`),
  `actions` (incl. `capabilities`, with the `status.*` write prohibition enforced),
  `migrations`, and the optional `identity`/`parent`/`orbit`/`rotation`/`environment`
  blocks — the previously-stubbed extractors are implemented.
- **Normative `GRAMMAR.md` v1.0** and a `comprehensive.hpdl` example exercising the
  full surface. Example/README stale "not-yet-ratified" notes corrected.
- Negative tests (unknown enum variant, float in an integer field, `None`/`Some`
  sugar) and cross-frontend parity tests covering the full surface.

### Fixed — `hpdl-syntax` lexer

- **Byte/char position desync on multibyte UTF-8 input.** `Lexer::current_char`
  treated the byte offset `pos` as a `char` index (`chars().nth(pos)`), so any
  multibyte character (e.g. an em-dash in a comment or string) desynced the cursor
  and could raise a spurious `Unexpected character: '\0'` near EOF. `current_char`
  now decodes at the byte offset via a non-panicking `get(pos..)`, and the comment /
  float-peek lookaheads use a consistent `char_after` helper. Regression tests added.

### Changed — `hpdl-compiler` crate

- Wired the native frontend seam: new `frontend::decode_hpdl` and a public
  `Compiler::compile_hpdl_str(name, source)` entry point, mirroring the RON path.
  Added the `hpdl-compiler → hpdl-syntax` dependency edge (architecture-sanctioned;
  no cycle).
- New `tests/hpdl_ron_equivalence.rs`: proves native `.hpdl` and RON compile to
  **byte-identical canonical PIR** and identical content hashes over the full surface.

### Changed — `hpdl-cli` crate

- The CLI now dispatches by file extension: `.hpdl` sources route through the native
  frontend (`compile_hpdl_str`), everything else (`.ron` and unrecognized) through
  the RON frontend. `check`/`build`/`inspect`/`export` all work on native `.hpdl`
  files. Added a `minimal.hpdl` fixture and tests, including one asserting the
  `.hpdl` and `.ron` fixtures export identical canonical PIR via the CLI.

### Added — repository

- Root `README.md` and an equivalent example planet pair under `examples/`
  (`earth.hpdl` + `earth.ron`) that compile to byte-identical PIR.

### Decisions (ADR 0010)

- **Enum/newtype spelling = RON-identical** (`Variant` / `Variant(args)` /
  `Variant(field: value)`), so the two frontends stay model-identical.
- **World laws out of native v1.0** — the model's `world_laws` is `#[serde(skip)]`
  (ADR 0005); neither serialized frontend can express them, and doing so is coupled to
  a PIR format-version decision. Deferred to a later grammar minor version.
- Open syntax questions resolved: optional whitespace before unit suffix (advisory,
  no rescale); string escapes limited to `\n \t \r \\ \"` (`\uXXXX` reserved); decimal
  numbers only (hex/bin/oct reserved); trailing commas optional.

### Compatibility

- **No PIR format change.** Native HPDL lowers to existing PIR v2.0 bytes;
  `PirVersion::CURRENT` unchanged; no golden PIR vectors regenerated.
- Grammar version `1.0` is now a committed contract; additive syntax → `1.1`,
  breaking → `2.0` (independent of crate and PIR versions).

---

## [Unreleased]

### Added

- **`00-workspace-foundation`** — Initial Cargo workspace with six active crate
  skeletons: `hpdl-model`, `hpdl-sema`, `hpdl-ron`, `hpdl-pir`,
  `hpdl-compiler`, `hpdl-cli`.
- Pinned Rust toolchain via `rust-toolchain.toml` (OD-01). Initially `1.81.0`
  pending approval; **ratified to `1.99.1`** — see the ADR 0001 entry below.
- MSRV declared across all crates (OD-02). Initially `1.80` pending approval;
  **ratified to `1.85.0`** — see the ADR 0001 entry below.
- `deny.toml` with MIT/Apache-2.0 license allowlist and advisory policy.
- `SECURITY.md` with vulnerability disclosure process.
- `.gitignore` for Rust/OS/editor artifacts.
- `docs/rfcs/` directory with ADR format guide and open-decisions record.
- **`01-planetary-domain-model`** — Complete `hpdl-model` implementation with 10 typed
  domain structs (`BodyId`, `PhysicalProperties`, `TraitSet`, `ModuleRef`,
  `StatusDeclaration`, `ActionDeclaration`, `MigrationDeclaration`, `CelestialBody`,
  `PlanetDefinition`, `Error`). 46 unit/integration tests + 9 doctests. Golden fixture
  roundtrip validation. Zero floats, no Earth defaults, strict lints enforced.
- **Release & support policy** (`docs/RELEASE_POLICY.md`, ratified in **ADR 0013**)
  — supported version windows for crate APIs and PIR format versions, the PIR
  format-version deprecation policy (minor = additive/read-forever, major =
  breaking + ADR + reviewed golden vectors + read-only window for the prior major),
  and a pointer to the CVE/disclosure process in `SECURITY.md`. `SECURITY.md` now
  links to the policy. Windows are release-relative and take effect at 1.0; pre-1.0
  remains `main`-only. Governance-only: no code, crate-version, or PIR-byte change.

---

<!-- Add new releases above this line in the format:
## [X.Y.Z] - YYYY-MM-DD
### Added / Changed / Deprecated / Removed / Fixed / Security
-->

## [Unreleased] — Feature 02: Canonical PIR (2026-10-14)

### Added — `hpdl-pir` crate

- **Versioned PIR format**: `PirVersion` with major.minor versioning and backward compatibility checking
- **Content-addressed integrity**: `CanonicalHash` using SHA-256 with domain separator "HPDL-PIR-v1"
- **Envelope structure**: `PirEnvelope` wrapping versioned payloads with automatic hash computation
- **Canonical encoding**: `encode_planet_definition` producing deterministic binary output via bincode
- **Bounded decoding**: `decode_planet_definition` with 10MB size limit and version validation
- **Typed errors**: `Error` enum covering version mismatches, oversized inputs, malformed data, encoding failures
- **Determinism guarantees**: Fixed endianness, sorted BTreeMap/BTreeSet, domain-separated hashing, platform-independent
- **Comprehensive tests**: 36 tests (30 unit + 3 golden + 3 doc) covering roundtrips, malformed input, hash stability

### Changed — `hpdl-model` crate

- Moved `serde` from `dev-dependencies` to `dependencies` with `derive` feature
- Added `Serialize` and `Deserialize` derives to all 9 model types for PIR encoding support
- No semantic changes; fully backward compatible

### Dependencies

- Added `sha2 = "0.10"` for SHA-256 hashing
- Added `bincode = "1"` for deterministic binary serialization  
- Added `serde_bytes = "0.11"` for efficient byte array handling


## [Unreleased] — Feature 06: Compiler Pipeline (2026-10-16)

### Added — `hpdl-compiler` crate

- **Orchestration entry point**: `Compiler` with `compile_ron_str(name, source)`
  (full pipeline from RON text) and `compile_validated(name, validated)` (shared tail
  from an already-validated definition, used by every frontend and equivalence tests).
- **Configuration**: `CompileOptions` (validator config, optional lockfile, resource
  limits) and `ResourceLimits` (`max_source_bytes` default 10 MiB, `max_modules`
  default 1024), both with strict, deterministic defaults.
- **Deterministic module resolution**: `BuildLock` (line-oriented `name version hash`
  lock, `BTreeMap`-backed), `LockedModule`, `ResolvedModule`, `ResolvedModules`
  (sorted by name). Fails closed with `UnresolvedModule`, `ModuleVersionMismatch`, and
  `LockParse` errors; no network or filesystem access.
- **Representability guard**: fixed-point lowering check verifying every physical
  numeric field fits its canonical storage width using checked arithmetic; reserves
  the Amendment A2 per-field quantum seam. No floating-point on any canonical path.
- **Canonical emission**: delegates to `hpdl-pir` and verifies the content hash before
  returning; the compiler never re-implements encoding or hashing.
- **Reproducible build manifest**: `BuildManifest` recording source name, compiler
  semantics version, PIR format version, content hash, and the sorted module closure,
  with a stable `to_text()` serialization.
- **Typed error model**: `Error` enum (`Frontend`, `Validation`, `UnresolvedModule`,
  `ModuleVersionMismatch`, `LockParse`, `Lowering`, `Emission`, `Limit`).
- **Tests**: unit tests per stage plus integration suites — `pipeline.rs` (positive
  and negative paths), `equivalence.rs` (RON vs. direct-validated byte parity), and
  `golden_build.rs` with a frozen manifest/hash fixture under `tests/golden/`.

### Changed — `hpdl-compiler` crate

- Added dependency edge `hpdl-compiler → hpdl-ron` (plus `hpdl-model`), consistent with
  the architecture steering (`hpdl-cli → hpdl-compiler → { hpdl-ron, hpdl-syntax }`).
  Keeps existing edges to `hpdl-sema` and `hpdl-pir`. No reverse edges introduced into
  `hpdl-pir` or `hpdl-runtime-api`.

### Notes

- No change to canonical PIR bytes or the PIR format version; the compiler consumes
  `hpdl-pir` as-is. No `hpdl-pir` golden-vector regeneration was performed.
- Open decisions recorded: OD-06.1 (lock format alignment with feature 14 stdlib),
  OD-06.2 (per-field numeric quanta under Amendment A2).

## [Unreleased] — Feature 07: Celestial and Orbital Systems (2026-10-16)

### Added — `hpdl-model` crate

- **Five orbital declaration types** (`src/orbital.rs`), all storing continuous
  quantities as scaled integers (no `f32`/`f64`), with validating, non-panicking
  constructors and full rustdoc + examples:
  - `ReferenceFrame` — closed set (`EquatorialJ2000`, `EclipticJ2000`, `BodyFixed`,
    `ParentCentered`) plus a validated `Custom(name)` escape hatch that is accepted as
    a declaration but reported as non-executable by the validator.
  - `CelestialIdentity` — closed kinds (`Star`, `Planet`, `DwarfPlanet`, `Moon`,
    `Asteroid`, `Comet`, `Barycenter`) plus a validated `Other(name)`.
  - `KeplerianElements` — six classical elements: semi-major axis (metres),
    eccentricity (`e × 1e6`, bounded `[0, 1_000_000)`), and inclination / node /
    periapsis / mean-anomaly angles in milli-degrees, each range-checked.
  - `ParentRef` — gravitational-parent `BodyId` plus the orbit's `ReferenceFrame`.
  - `RotationModel` — sidereal period (seconds; negative = retrograde; zero rejected),
    axial tilt (milli-degrees), and spin-axis frame.
- **`CelestialBody` orbital fields**: optional `identity`, `parent`, `orbit`, and
  `rotation`, each with a builder-style setter (`with_identity`, `with_parent`,
  `with_orbit`, `with_rotation`) and accessor. Absence means *undeclared* — no Earth
  default. The existing `new(..)` signature and all call sites are unchanged.
- **New typed errors** on `hpdl_model::Error`: `InvalidFrameName`,
  `InvalidSemiMajorAxis`, `InvalidEccentricity`, `InvalidAngle { field, value }`,
  `InvalidRotationPeriod`.
- **Golden fixture** `tests/golden/orbital_body.json` freezing the canonical serialized
  form of a fully-declared orbital body; it roundtrips through the real model types.

### Added — `hpdl-sema` crate

- **`OrbitalValidator`** (`src/orbital_validator.rs`), wired once into the single
  `Validator` pipeline. Checks: parent acyclicity (self-parent → `ParentCycle`),
  reference-frame consistency between parent and orbit (→ `ReferenceFrameMismatch`),
  custom/unsupported frames (→ `UnsupportedOrbitalModel`, explicitly non-executable),
  and a defensive Keplerian element range re-check (→ `InvalidOrbitalElement`). No
  float arithmetic; cycle detection is a bounded reachable-set walk.
- **New stable diagnostic codes** (E8000 series): `ParentCycle` (E8001),
  `ReferenceFrameMismatch` (E8002), `InvalidOrbitalElement` (E8003),
  `UnsupportedOrbitalModel` (E8004).

### Changed — `hpdl-pir` crate (BREAKING: PIR format v1.0 → v2.0)

- **PIR format version bumped to 2.0** (`PirVersion::CURRENT`). Adding the optional
  orbital fields to the bincode-encoded `CelestialBody` changes canonical PIR bytes for
  every definition (bincode writes a presence byte per `Option`, including `None`), which
  is a breaking byte-layout change. A v2 decoder rejects v1-encoded payloads.
- **Hash domain separator** changed `HPDL-PIR-v1` → `HPDL-PIR-v2`, binding digests to the
  new format major version so v1 and v2 encodings never collide.
- **Crate version** bumped `0.1.0` → `0.2.0`.
- Decision recorded in `docs/rfcs/0003-pir-v2-orbital-fields.md`.

### Changed — `hpdl-compiler` crate

- Golden build fixture (`tests/golden/minimal_build.txt`) updated deliberately to
  `pir_version = 2.0` and the new content hash, per ADR 0003 (reviewed vector update, not
  a silent regeneration).

### Notes

- **PIR change (OD-07.1, resolved)**: the original "no PIR change" scope assumption was
  incorrect for a bincode encoder; handled as the v2 format bump above.
- **Eccentricity bound (OD-07.3)**: v1 supports only closed orbits `e ∈ [0, 1)`.
  Parabolic/hyperbolic trajectories are rejected explicitly, never clamped.
- An orbit declaration is static data; this feature performs no orbital propagation or
  simulation. Custom frames are flagged, never silently treated as simulatable.

## [Unreleased] — Spec completion pass for features 03, 04, 05 (2026-10-16)

Filled in the previously-incomplete task deliverables for three specs. No public
model/PIR behaviour changed; this pass adds tests, examples, docs, and small,
honest frontend improvements.

### `hpdl-sema` (feature 03)

- Added external test suites: `tests/integration_tests.rs` (end-to-end over the public
  `Validator`), `tests/security_tests.rs` (limits + no-panic), `tests/determinism_tests.rs`
  (stable, repeatable diagnostics), and `tests/status_tests.rs` driven by golden fixtures
  `tests/golden/status_thresholds_{valid,invalid}.json`.
- Added runnable examples `examples/basic_validation.rs` and `examples/error_handling.rs`.
- Added `DIAGNOSTIC_CODES.md` (full stable code catalogue incl. the E8000 orbital series)
  and `README.md`.
- Resolved the stale "API alignment" notes on the type checker / constraint solver /
  module validator (they already used the correct integer-based model APIs).

### `hpdl-ron` (feature 04)

- Added `tests/validation_integration.rs` and `tests/fixtures/*.ron`
  (minimal, earth-like, gas-giant, dwarf-planet, and an intentionally invalid zero-mass
  example) exercising parse → `hpdl-sema` validation.
- Added security tests: `tests/security_tests.rs`, `tests/malformed_input_tests.rs`,
  `tests/path_security_tests.rs` (enforced file-size limit, no-panic on garbage, path
  traversal rejection).
- Added examples (`basic_usage`, `validation_workflow`, `error_handling`), `README.md`,
  and `CONVENTIONS.md`.

### `hpdl-syntax` (feature 05)

- Added `parse_and_validate`, routing parsed `.hpdl` through the shared `hpdl-sema`
  validator; new `Error::Validation` variant.
- Added `src/limits.rs` (`ParseLimits`) and `parse_file_with_limits`, enforcing a
  source-size budget on untrusted input (fails closed with `Error::Security`).
- **Fix:** the parser now accepts contextual keywords (`physical`, `traits`, `status`,
  `action`, `module`, `migration`, `body`) as field names — previously `physical:` was
  rejected as "Expected Identifier", which blocked the documented `physical { … }` block.
- Added example `.hpdl` files (`examples/{minimal,earth,gas_giant}.hpdl`),
  `tests/integration_tests.rs`, `GRAMMAR.md` (implemented subset, explicitly marked
  not-yet-ratified), and `README.md`.

### Known deferrals (documented, not silently claimed)

- Native-HPDL↔RON byte/hash equivalence and HPDL extraction of
  modules/statuses/actions/migrations await grammar ratification.
- Performance benchmarks are not added (no reliable perf target in this environment).
- `cargo deny` and `cargo semver-checks` are not installed here; run in CI.

## [Unreleased] — Parser error recovery + benchmarks (2026-10-16)

### `hpdl-syntax`

- **Multi-error recovery:** new `Parser::parse_file_recover` and the
  `parse_source_collect_errors` entry point collect multiple independent syntax errors in a
  single pass, synchronizing at field boundaries (`,`) and the planet-body closing brace.
  New `src/diagnostic.rs` with `ParseError` (message + span) and `ParseErrors`. The strict
  single-error `parse_file` path is unchanged. Recovery is bounded (terminates on
  pathological input) and span-carrying; covered by new tests in `tests/integration_tests.rs`.

### Benchmarks (dependency-free, `harness = false`)

Added indicative, zero-dependency benchmarks (no `criterion`; `std::time::Instant` only)
that report time/throughput across several input sizes so scaling regressions are visible.
They measure, they do not assert machine-specific pass/fail targets:

- `hpdl-syntax/benches/parse_bench.rs` — lex+parse+convert throughput.
- `hpdl-ron/benches/parse_bench.rs` — RON deserialization throughput.
- `hpdl-sema/benches/validate_bench.rs` — `Validator::validate` across statuses+actions.
- `hpdl-pir/benches/encode_bench.rs` — canonical encode + domain-separated hash (the
  determinism-critical path).
- `hpdl-compiler/benches/pipeline_bench.rs` — full `compile_ron_str` pipeline.

Run with `cargo bench -p <crate>`. No new runtime or dev dependencies were introduced.

## [Unreleased] — Feature 08: Planetary Environments (2026-10-16)

### Added — `hpdl-model` crate

- **Environment declaration types** (`src/environment.rs`), all scaled-integer (no floats),
  with validating non-panicking constructors and rustdoc + examples:
  - `GasId` (validated gas identifier) and `Digest` (64-char lowercase hex SHA-256).
  - `Composition` — gas mole fractions in ppm (`BTreeMap<GasId, u32>`) with an explicit
    normalization tolerance; sum validated with checked arithmetic.
  - `AtmosphereProfile` — surface pressure (Pa, `> 0`), composition, optional versioned
    `climate_model` contract reference.
  - `ThermalProfile` — mean/min/max surface temperature in milli-kelvin, `min <= mean <= max`.
  - `Hydrosphere` / `Cryosphere` — surface coverage in ppm (`0..=1_000_000`).
  - `Biosphere` — tri-state `Absent` / `Unknown` / `Present(ComplexityTier)`; absence is a
    first-class value, never a zero score.
  - `AssetRef` / `AssetKind` — content-addressed visual asset references (required digest,
    no fetching or byte verification).
  - `EnvironmentDefinition` — the aggregate.
- **`CelestialBody` environment field**: optional `environment` with a `with_environment`
  builder setter and accessor. Absent = undeclared.
- **New typed errors**: `InvalidEnvironmentName`, `EmptyComposition`,
  `CompositionFractionOutOfRange`, `CompositionSumOutOfTolerance`, `ZeroPressure`,
  `InvalidTemperatureOrder`, `CoverageOutOfRange`, `InvalidAssetDigest`.
- **Golden fixture** `tests/golden/environment_body.json` (atmosphere with a multi-gas
  composition, thermal, hydrosphere, cryosphere, a present biosphere, and an asset);
  roundtrips through the real model types.

### Added — `hpdl-sema` crate

- **`EnvironmentValidator`** (`src/environment_validator.rs`), wired once into the single
  `Validator`. Checks composition bounds, atmosphere pressure, temperature ordering,
  coverage ranges, and asset digests; flags declared climate models as non-executable; an
  `Absent`/`Unknown` biosphere produces no diagnostic.
- **New stable diagnostic codes** (E9100 series): `CompositionOutOfBounds` (E9101),
  `InvalidEnvironmentValue` (E9102), `UnsupportedEnvironmentModule` (E9103),
  `MissingAssetDigest` (E9104). Documented in `DIAGNOSTIC_CODES.md`.

### Changed — PIR format (folded into unreleased v2.0, OD-08.1 option A)

- Adding the `environment` field changed canonical PIR bytes. Because PIR v2.0 is still
  unreleased, this was **folded into v2.0** rather than bumped to v3.0: `PirVersion::CURRENT`
  stays `2.0`, domain separator stays `HPDL-PIR-v2`, `hpdl-pir` stays `0.2.0`. The compiler
  golden vector (`minimal_build.txt`) was regenerated deliberately (`content_hash`
  `6e1610fe…` → `23d96f5d…`; `pir_version` unchanged). Recorded as an amendment to
  `docs/rfcs/0003-pir-v2-orbital-fields.md`. The feature-07 orbital golden fixture
  (`orbital_body.json`) gained the `"environment": null` field.

### Notes

- No climate/chemistry/ecosystem simulation; no asset fetching or digest verification
  (format validation only); no status/score derivation from atmosphere components.

## [Unreleased] — Cross-frontend equivalence tests (feature 03, Task 11) (2026-10-16)

- Added `crates/hpdl-syntax/tests/cross_frontend_tests.rs`: parses equivalent `.hpdl` and
  `.ron` sources and asserts (1) identical `PlanetDefinition` models and (2) identical
  `hpdl-sema` validation outcomes — diagnostic codes, messages, and order — for valid,
  invalid (zero-mass → shared `E2001`), and multi-field cases.
- Placed in `hpdl-syntax` (with `hpdl-ron` as a dev-dependency) rather than `hpdl-sema`, to
  respect the dependency direction: `hpdl-sema` must not depend on a frontend. This is a safe
  dev-dependency because nothing depends on `hpdl-syntax`.
- The test documents the current shared-equivalence surface (`id`/`physical`/`traits`) with an
  explicit boundary assertion; it expands when the native converter wires
  modules/statuses/actions (feature 05).

## [Unreleased] — Feature 09: Status and Vitality (2026-10-16)

### Added — `hpdl-model` crate

- **Status / vitality types** (`src/status_vitality.rs`), integer-only, with validating
  non-panicking constructors and rustdoc + examples:
  - `StatusDimension` — independent dimensions (`Physical`, `NativeEcosystem`,
    `Civilization`, `Supernatural`, `Custom`); absence is "not applicable", never a zero.
  - `EvaluatorRef` (versioned evaluator identity, wraps `ModuleRef`) and `BaselineRef`
    (planet-specific, content-addressed via `Digest`).
  - `CutPoints` with `classify()` — ordered Amendment A1 bands; `[cut_i, cut_{i+1})`, last
    band includes the max; out-of-range scores rejected, never clamped.
  - `EvaluationWindow` (span `> 0` + optional scope).
  - `StatusResult` — the derived-output replay/audit contract (dimension, score, band,
    evaluator version, sorted input hashes, evidence); produced by an evaluator, never
    authored. Lives provisionally in `hpdl-model` under a "runtime contract (pre-runtime-api)"
    label (see OD-09.1 below).
- **Extended `StatusDeclaration`**: optional `dimension`, `baseline`, `evaluator`,
  `cut_points`, `window` via builder setters; `StatusDeclaration::new(name, min, max)`
  unchanged.
- **New typed errors**: `InvalidStatusDimensionName`, `InvalidCutPoints`, `ScoreOutOfRange`,
  `ZeroEvaluationWindow`, `InvalidBaselineRef`.
- **Golden fixture** `tests/golden/status_vitality_body.json` (dimension + baseline +
  evaluator + cut-points + window); roundtrips byte-for-byte. Boundary classifications
  (24/25/49 and scaled sub-integer 24.5/49.9) and the replay invariant are covered by unit
  tests.

### Added — `hpdl-sema` crate

- `StatusValidator::validate_status_vitality`, wired into the single `Validator` via the
  existing status pass: evaluator completeness (window + cut-points required if an evaluator
  is declared → `IncompleteEvaluatorDeclaration`), unsupported evaluator (→
  `UnsupportedEvaluator`, non-executable), and defensive cut-point bounds. Dimensions are
  independent — any subset is valid and produces no diagnostic.
- **New stable diagnostic codes** (E9200): `IncompleteEvaluatorDeclaration` (E9201),
  `UnsupportedEvaluator` (E9202). Documented in `DIAGNOSTIC_CODES.md`.

### Decisions

- **OD-09.1 — `hpdl-runtime-api` kept deferred (option A).** The design named it, but the
  workflow steering marks it deferred, so it is NOT scaffolded. `StatusResult` lives
  provisionally in `hpdl-model` and moves to `hpdl-runtime-api` when runtime integration
  begins.
- **OD-09.2 — no PIR change (N/A).** The extended status fields are not lowered into
  canonical PIR in this feature, so PIR bytes are unchanged and no golden-vector churn
  occurred. Encoding them later is a separate format-version decision.

### Notes

- No evaluation/aggregation engine; no evaluator/baseline fetching or digest verification;
  no single combined "overall vitality" score. Statuses remain derived — nothing authors a
  score; out-of-range is rejected, never clamped.

## [Unreleased] — Feature 10: hpdl-runtime-api activated (OD-10.1 option B) (2026-10-16)

### Added — new crate `hpdl-runtime-api` (published runtime contracts)

Previously **deferred**; activated per ADR 0004. Published contract crate depending only on
`hpdl-pir` + `hpdl-model` (+ `serde`, `sha2`, `thiserror`); strict lints
(`forbid(unsafe_code)`, `deny(missing_docs, unwrap_used, expect_used, indexing_slicing,
float_arithmetic)`). It defines **contracts only** — HPDL still does not execute state.

- `Tick` — monotonic `u64` time step with checked `next()`.
- `ActionId` — validated action-instance identifier.
- `StateHash` — validated 64-char lowercase-hex content hash of canonical state.
- `Capability` — namespaced authority token an action requires to mutate state.
- `ReadWriteSet` — deterministic ordered read/write field paths + declared capabilities, with
  a documented write→capability coverage rule (`uncovered_writes`).
- `EventEnvelope` + `EventOrderKey` — a hash-bound replay record binding action id, tick,
  acting module+version, input/output state hashes, and the read/write set; a dedicated event
  domain separator (`HPDL-EVENT-v1`, never the PIR separator); pure `is_consistent()`,
  `is_replay_duplicate_of()`, and a total `(tick, action_id)` ordering key.

### Tests — mock executor (test double, not a simulation)

- `tests/mock_executor.rs` implements a clearly-labelled **mock** of the external HORIZON
  engine to exercise the design's required behaviors against the real contracts: unauthorized
  action rejection, replay-duplicate detection, deterministic ordering, cross-domain portal
  effects gated by a capability, and event-hash consistency. The crate ships no executor.

### Changed — project metadata

- Workspace `Cargo.toml`: uncommented the `crates/hpdl-runtime-api` member.
- Workflow steering crate-status table: `hpdl-runtime-api` `deferred` → `active (feature 10)`;
  `semver-checks` line now covers `hpdl-pir` and `hpdl-runtime-api`.
- Decision recorded in `docs/rfcs/0004-activate-hpdl-runtime-api.md`.

### Notes / still open

- This change activates the **runtime-contract** half of feature 10. The definition-side half
  (extending `ActionDeclaration` with capability declarations in `hpdl-model` and the
  corresponding `hpdl-sema` validation, OD-10 FR1/FR2/FR4) remains to be done as separate
  tasks. No PIR bytes changed (OD-10.2 N/A). `StatusResult` (feature 09) may be migrated from
  `hpdl-model` into `hpdl-runtime-api` as follow-up.

## [Unreleased] — Feature 10: action capabilities (definition-side) (2026-10-16)

Completes feature 10's definition-side half and relocates the shared capability vocabulary
to the dependency root (the "move down" decision).

### Added — `hpdl-model` crate

- `Capability` (namespaced authority token) and `ReadWriteSet` (ordered reads/writes +
  capabilities with a `uncovered_writes` coverage rule and field-path validation) in
  `src/capability.rs`. New errors `InvalidCapability` and `InvalidFieldPath`.
- `ActionDeclaration` gained an optional capability set (`with_capabilities` builder +
  `capabilities()` accessor) and `read_write_set()`. `new(name, reads, writes)` and the
  status-write prohibition are unchanged.

### Added — `hpdl-sema` crate

- `ActionValidator::validate_capabilities`, wired into the single `Validator`: malformed field
  path → `MalformedFieldPath` (E9301); mutating write not covered by a declared capability →
  `UncoveredWrite` (E9302). Documented in `DIAGNOSTIC_CODES.md` (E9300 block).

### Changed — `hpdl-runtime-api` crate

- `Capability` and `ReadWriteSet` now live in `hpdl-model` and are **re-exported** from
  `hpdl-runtime-api` (`pub use hpdl_model::{Capability, ReadWriteSet}`) so both the authored
  declaration and the runtime event record share one vocabulary with no duplication. The
  crate's own `capability.rs`/`rwset.rs` were removed; `EventEnvelope` is unchanged.

### Changed — fixtures/tests

- The sema integration "comprehensive valid" definition and the `earth_like.ron` fixture now
  declare the covering capability (`state.atmosphere.write`) for their atmosphere write — the
  new coverage check correctly flagged them otherwise. `CONVENTIONS.md` documents the
  `Capability("...")` RON newtype form.

### Notes

- No PIR change (OD-10.2 N/A): capabilities are model/validation surface, not PIR payload.
- Feature 10 is now complete on both halves; `design.md` and `tasks.md` were added to the spec.

## [Unreleased] — Feature 11: World Laws & Custom Reality (2026-10-17)

Implements domain 16 (world laws, magic, alternative physics) on the definition +
validation side, making the previously-stubbed `validate_world_laws` real and giving the
reserved `E6003`/`E6004` diagnostic codes behavior. HPDL ships **no built-in or universal
magic** and runs no alternative-physics solver — a declared law is a contract, never
executed.

### Added — `hpdl-model` crate

- **World-law declaration types** (`src/world_law.rs`), integer/enum-only (no floats),
  with validating non-panicking constructors, bounds, and rustdoc + examples:
  - `LawDomain` — a validated, namespaced domain token (1–64 chars, dotted segments).
    The reserved `physics` first-segment prefix marks a *physical* domain; anything else
    (e.g. `arcane.aether`) is a *custom* dimension. No built-in closed domain list
    (OD-11.1).
  - `LawMode` — exactly two forms: `Additive`, or `Replaces(BTreeSet<LawDomain>)` (the
    single authoritative law for the named domains). Composition is intersection, never
    last-write-wins.
  - `MagicBridge` — a declared custom-dimension → physical-dimension contract (ordered
    `sources`, one physical `target`, versioned bridge `ModuleRef`). A contract only; HPDL
    performs no unit conversion (OD-11.3: many sources → one physical target).
  - `WorldLaw` — module ref + mode + claimed `BTreeSet<LawDomain>` + optional
    `MagicBridge`, with `authoritative_domains()`, `touches_physical()`, `is_magic()`.
- **`CelestialBody` world-law field**: `world_laws: Vec<WorldLaw>` with `with_world_law` /
  `with_world_laws` builders and a `world_laws()` accessor. The field is `#[serde(skip)]`
  (see OD-11.2 below) so it is carried in-memory for the validator but never serialized.
- **New typed errors**: `InvalidLawDomain`, `EmptyLawDomains`, `TooManyLawDomains`,
  `EmptyBridgeSources`, `TooManyBridgeSources`, `NonPhysicalBridgeTarget`.
- **Bounds** (`MAX_WORLD_LAWS`, `MAX_DOMAINS_PER_LAW`, `MAX_BRIDGE_SOURCES`) and the
  reserved `PHYSICAL_NAMESPACE` constant, exported from the crate root.

### Added — `hpdl-sema` crate

- **`ModuleValidator::validate_world_laws`** implemented (stub removed) and already wired
  into the single `Validator`. Enforces:
  - duplicate law module reference within a body → `E6002` (reused, OD-11.5);
  - one authoritative law per exclusive domain → `E6003 MultipleWorldLawsForDomain`;
  - magic reaching a physical domain with no declared bridge → `E6004 MissingMagicBridge`;
  - a well-formed law whose module has no registered implementation →
    `E6005 UnsupportedWorldLaw` (**warning**, never blocks a valid declaration; mirrors
    E8004/E9103/E9202).
- **New stable diagnostic code** `E6005 UnsupportedWorldLaw`. The reserved `E6003`/`E6004`
  codes now have behavior (not a breaking change). `DIAGNOSTIC_CODES.md` updated.

### Tests

- `hpdl-model`: 11 `world_law` unit tests + 3 doctests; roundtrip tests
  `world_law_body_carries_laws_in_memory` and `world_laws_are_not_serialized_this_feature`
  (freezes the OD-11.2 no-serialization decision).
- `hpdl-sema`: 10 `module_validator` unit tests (additive, replacement, magic±bridge,
  purely-arcane, duplicate, disjoint-compose) + 4 end-to-end `integration_tests`
  (including strict-mode promotion of the `E6005` warning) + the `E6005` code-stability
  test.

### Decisions

- **OD-11.1 → open namespaced domain tokens** with a reserved `physics` physical prefix; no
  closed domain enum.
- **OD-11.2 → world laws NOT lowered into PIR this feature (resolved: keep out).**
  The `world_laws` field is `#[serde(skip)]`, so canonical PIR (bincode) sees no new field:
  **PIR bytes, hashes, golden vectors, and `PirVersion::CURRENT` (2.0) are unchanged.**
  (`#[serde(skip_serializing_if)]` was rejected — it desynchronizes bincode's non-self-
  describing encode/decode and corrupts the format; `#[serde(skip)]` is symmetric and
  byte-safe.) A serialized frontend surface for world laws is deferred to features 04/05.
- **OD-11.3 → many bridge sources → one physical target**, one bridge module.
- **OD-11.4 → all law modules non-executable** at this stage (`E6005`, warning); the module
  registry / trust roots are deferred to feature 14.
- **OD-11.5 → reuse `E6002`** for duplicate law refs; `E6003` strictly for the
  one-domain-two-authoritative-laws conflict.
- Recorded in `docs/rfcs/0005-world-laws-and-magic.md`.

### Notes

- No world-law / magic execution, no alternative-physics solver, no bridge arithmetic, no
  module registry, no built-in or universal magic. No PIR byte change (OD-11.2). No floats;
  no panicking paths; bounded, checked construction on untrusted input.

## [Unreleased] — Feature 12: Module Runtime Contracts (Vitality & Process) (2026-10-17)

Implements domain 12 (vitality / process contracts) in `hpdl-runtime-api` — the
runtime-contract counterpart to feature 09's status/vitality declarations. HPDL still ships
**contracts, not an engine**: no executor, solver, scheduler, or state evolution. Behaviors
are exercised by a mock executor test double. No PIR change.

### Added — `hpdl-runtime-api` crate

- **`ProcessId`** — a validated process-run instance id (same grammar as `ActionId`, distinct
  type; new `Error::InvalidProcessId`).
- **`ProcessContract`** — the run contract for a versioned process module: a `ModuleRef`, the
  permitted `ReadWriteSet`, and an activation `Tick`. `permits_write(path)` (reusing the
  read/write capability-coverage rule) and `is_active_at(tick)` (inclusive, OD-12.5).
- **`EvaluatorContract`** — the run contract for a versioned vitality evaluator: `EvaluatorRef`
  + `StatusDimension` + `BaselineRef` + `EvaluationWindow` (reusing the feature-09 declaration
  vocabulary). Computes nothing; the engine derives the result.
- **`ProcessReceipt`** + **`ProcessOrderKey`** — a deterministic, hash-bound record of one
  process run at one tick (process id, tick, `ModuleRef`, input/output `StateHash`,
  `ReadWriteSet`, binding). Binding uses a dedicated **`HPDL-PROCESS-v1`** domain separator —
  distinct from `HPDL-EVENT-v1` and `HPDL-PIR-v2` — with a pure `is_consistent()`,
  `is_replay_duplicate_of()`, and a total `(tick, process_id)` `order_key()`.
- **New error contracts**: `UnsupportedModule { name, version }`, `ProcessInactiveAtTick`,
  `UnauthorizedProcessEffect`. The crate provides the error *shapes* + predicates; the engine
  raises them and the module registry (feature 14) decides "supported".
- **Internal `hashing` module**: the length-prefixed-feed and lowercase-hex helpers (formerly
  private in `event.rs`) are factored out and shared by `EventEnvelope` and `ProcessReceipt`,
  with no behavior change to `EventEnvelope` (its binding tests pass unchanged).
- Re-exports more model vocabulary (`ModuleRef`, `EvaluatorRef`, `BaselineRef`,
  `EvaluationWindow`, `StatusDimension`) so runtime consumers have one import point.

### Changed — `StatusResult` relocated (OD-12.4; BREAKING for `hpdl-model`, pre-1.0)

- **`StatusResult` moved from `hpdl-model` to `hpdl-runtime-api`** and is now
  `hpdl_runtime_api::StatusResult`. It is a runtime *output* contract; feature 09 had placed
  it in `hpdl-model` only provisionally because `hpdl-runtime-api` was deferred (OD-09.1). It is
  **not** re-exported from `hpdl-model` — doing so would invert the dependency direction
  (`runtime-api → model`), which the architecture steering forbids.
- The feature-09 status *declaration* types stay in `hpdl-model`: `StatusDimension`,
  `EvaluatorRef`, `BaselineRef`, `CutPoints`, `EvaluationWindow`, `Digest`. `StatusResult`'s
  unit tests + doctest moved with it.

### Tests

- `hpdl-runtime-api` unit tests: `ProcessId` validation; `ProcessContract` coverage +
  inclusive activation; `ProcessReceipt` consistency / tampering / replay-duplicate / total
  ordering / cross-domain-binding-distinctness; relocated `StatusResult` sorting + replay
  invariant; `hashing` helpers. Plus `tests/mock_process_executor.rs` — a labelled **mock**
  (not shipped, not a simulation) exercising inactive-at-tick, unauthorized-effect,
  deterministic ordering, replay-duplicate idempotence, and receipt-hash consistency.
- Existing `tests/mock_executor.rs` and `EventEnvelope` binding tests pass unchanged
  (shared-hashing refactor is behavior-preserving).

### Decisions

- **OD-12.1 → dedicated `ProcessId`.** **OD-12.2 → distinct `ProcessReceipt`** with
  `HPDL-PROCESS-v1`. **OD-12.3 → `ModuleRef`** identity (events keep flat module fields).
  **OD-12.4 → move `StatusResult`, no `hpdl-model` re-export** (preserve dependency direction).
  **OD-12.5 → activation inclusive.** Recorded in `docs/rfcs/0006-module-runtime-contracts.md`.

### Notes

- No executor / solver / scheduler; no state evolution; no module registry (feature 14); no
  migration execution (feature 13). No PIR byte change (verified via `hpdl-pir` golden hashes).
  No floats; no panicking paths; bounded, checked construction.

## [Unreleased] — Feature 13: Schema Migration Execution Contracts (2026-10-17)

### Added — `hpdl-runtime-api` crate

- **Migration execution contracts**: `MigrationProposal` (world id, source/target
  definition + schema hashes, implementation module + digest, activation tick, nonce,
  governance-policy reference; forward-only enforced), `MigrationReceipt` (hash-bound,
  `is_consistent()`, idempotent-retry identity, total ordering key), `MigrationResult`
  (`Committed` / `RejectedPreCommit`), `MigrationOrderKey`, and a `MigrationExecutor`
  trait (interface only — no engine shipped).
- **`MigrationId`**: validated newtype (same grammar as `ActionId`/`ProcessId`),
  deterministically derived from a proposal's binding fields so retries are idempotent.
- **`GovernancePolicyRef`**: a versioned policy id + policy digest reference (no keys,
  thresholds, or signatures — those live in the external simulation repo).
- **Dedicated `HPDL-MIGRATION-v1` binding separator**, distinct from the PIR, event, and
  process separators; reuses the shared `hashing` helpers.
- **Typed failure contracts**: `InvalidMigrationId`, `SchemaDowngrade`,
  `DuplicateMigrationId`, `PreCommitInvariantFailure`, `MigrationInactiveAtTick`,
  `UnauthorizedMigration`.
- Re-exports `BodyId` and `Digest` from `hpdl-model` for the migration surface.

### Tests

- Unit tests (migration + ids) and a `tests/mock_migration_executor.rs` test double
  exercising downgrade rejection, inactive-at-tick, pre-commit failure (prior state
  authoritative), duplicate-id idempotence, unauthorized rejection, deterministic
  ordering, and receipt tamper detection.

### Decisions

- OD-13.1 proposal lives in runtime-api; OD-13.2 dedicated `MigrationId`; OD-13.3 new
  `HPDL-MIGRATION-v1` separator; OD-13.4 derived migration id; OD-13.5 authorization
  referenced, not enforced. The three steering-flagged authorization items (approval-expiry
  basis, revoked-key handling, emergency/quorum policy) are surfaced in ADR 0007, NOT
  decided in code.

### Notes

- No PIR change; `hpdl-model` unchanged; dependency direction preserved. HPDL ships
  contracts, not an engine — execution stays external and is only mocked in tests.


## [Unreleased] — Feature 14: Immutable Standard-Library Packages & Lockfile (2026-10-17)

### Added — `hpdl-model` crate

- **`PackageId`** (namespaced name + **exact** version; mutable labels like `latest`,
  `^1`, `~1`, `>=1`, `*` rejected) and **`PackageManifest`** (id + content `Digest` +
  deduplicated/bounded dependency set; self-dependency rejected). New `Error` variants
  `MutablePackageVersion`, `TooManyDependencies`, `SelfDependency`.

### Added — `hpdl-compiler` crate

- **Authoritative `Lockfile`** (`lock_format_version = 1` header + transitive-closure
  `package = name version digest [deps...]` entries, `BTreeMap`-backed, deterministic
  `to_text()`), superseding the provisional `BuildLock` (which still parses).
- **`PackageResolver`**: offline, deterministic transitive-closure resolution with
  integrity verification (`verify_integrity`) and dependency-cycle rejection; sorted
  `ResolvedPackage` output; `resolve_packages` bridge from body module refs.
- **`CompileOptions.package_lock`** (`with_package_lock`): when supplied, the package
  closure is resolved and verified; PIR bytes for a given definition are unchanged.
- New `Error` variants: `LockFormat`, `UnresolvedPackage`, `PackageVersionMismatch`,
  `PackageIntegrity`, `DependencyCycle`.

### Tests

- Model/lockfile/resolver unit tests + `tests/packages.rs` (offline build, dependency
  conflict, unresolved/missing-transitive package, same-locked-source-same-PIR,
  lock-does-not-change-PIR, mixed versions) + a frozen `tests/golden/stdlib.lock` fixture.

### Decisions

- OD-14.1 reuse `hpdl_model::Digest` (no parallel `ContentDigest`); OD-14.2 authoritative
  `Lockfile` supersedes provisional `BuildLock`; OD-14.3 resolve body-level module refs
  only this feature; OD-14.4 trust roots / package authenticity surfaced in ADR 0008 and
  NOT decided in code (integrity verified, authenticity deferred).

### Notes

- No PIR change; no registry server, network, or archive download. The lock format version
  is the compatibility surface; future format changes require an ADR + golden update.


## [Unreleased] — Feature 15: HPDL CLI (`hpdl`) (2026-10-17)

### Added — `hpdl-cli` crate

- **`hpdl` subcommands**: `check`, `build`, `inspect`, `export`, plus `--help`/`-h` and
  `--version`/`-V`. All compilation delegates to `hpdl-compiler`; the CLI holds no logic.
- **Diagnostic presentation**: deterministic `text` (default) and `json` output
  (`--format`); diagnostics sorted; a hand-built JSON shape with a local escape helper
  (no `serde_json`).
- **Stable exit codes**: `0` success, `1` compilation/diagnostic failure, `2` usage/IO
  error, documented in `--help`.
- **`--lock`** loads an authoritative feature-14 `Lockfile`; **`export --out`** writes
  canonical PIR bytes (hex to stdout otherwise).
- A testable `hpdl_cli::dispatch(args) -> Outcome` library backing a thin `main`.

### Tests

- Unit tests (arg parsing, JSON escaping, error rendering) + `tests/cli.rs` integration
  tests (help/version, unknown subcommand/flag/missing-arg → exit 2, unreadable file,
  check ok/fail, build text+json, inspect summary, export file+hex, invalid lock) + a
  golden `inspect` fixture + a binary smoke test.

### Decisions

- OD-15.1 hand-rolled arg parser (no `clap`); OD-15.2 `text`+`json` formats (no
  `serde_json`); OD-15.3 exit codes `0`/`1`/`2`; OD-15.4 RON frontend diagnostics rendered
  as the compiler surfaces them. Recorded in ADR 0009.

### Notes

- No new third-party runtime dependency; no PIR / canonical-bytes change. Exit codes are a
  stability surface.

## [Unreleased] — ADR 0001 ratified: toolchain, MSRV, CI, semver, licenses (2026-10-17)

### Changed

- **Pinned toolchain** bumped `1.81.0` → **`1.99.1`** (OD-01) in `rust-toolchain.toml`
  and the CI `RUST_VERSION`. `1.99.1` is the patched current stable (1.99.0 + three
  bug fixes).
- **MSRV** raised `1.80` → **`1.85.0`** (OD-02) across every crate's `rust-version`
  and the CI `RUST_MSRV`. The `msrv` CI job builds + tests the workspace at this floor.
- **ADR 0001** flipped `Pending` → `Accepted`; OD-01…OD-05 ratified. OD-03 confirms
  GitHub Actions; OD-04 confirms semver-checks errors-only; OD-05 corrects the license
  allowlist to `Unicode-3.0` (matching `deny.toml`, not the draft's `Unicode-DFS-2016`).

### Notes

- No source or canonical-PIR change; this is a build-configuration ratification only.
  A future change to any ratified value requires a superseding ADR.
