# World Laws & Custom Reality — Requirements

## Overview

This feature covers **domain 16 of the v1 architectural scope: world laws & custom
reality (magic, alternative physics)**. It is the *definition + validation* side of
world laws: the typed model for declaring versioned world-law modules, their domain
ownership, their composition mode, and the magic→physics bridge, plus the single
semantic validator pass that enforces the invariants the steering already names.

Critically, the diagnostic surface for this feature **already exists as stubs**:

- `hpdl-sema` defines `E6003 MultipleWorldLawsForDomain` and `E6004 MissingMagicBridge`
  in `DIAGNOSTIC_CODES.md` and `diagnostic.rs`.
- `ModuleValidator::validate_world_laws` is a stub that returns an empty `Vec` with the
  comment *"Future: when world law support is added to the model."*

This feature **adds that model support and makes the stub real.** It does not invent a
new diagnostic vocabulary; it wires the one that was reserved for it.

Primary crates: `hpdl-model` (the declaration types), `hpdl-sema` (the one validator
pass). PIR impact is a surfaced decision (see OD-11.2).

> **Critical framing (product/architecture steering):** a world law or magic system is an
> *optional, registered, versioned, deterministic module contract*. HPDL declares the
> contract; it does **not** execute it. "Magic affects physical state only through a
> declared bridge." "One authoritative law per exclusive domain." A declared law with no
> registered runtime implementation is **non-executable** — accepted as a declaration,
> flagged, never simulated. There is **no universal magic system**: every world is
> explicit about which laws it adopts and which domains they replace.

## Goals

1. **Typed world-law declarations.** A body can declare a set of world laws, each a
   versioned module reference with a declared *mode* (`additive` or explicit replacement
   of named domains) and the set of physical domains it claims authority over.
2. **One authoritative law per exclusive domain.** When two declared laws both claim
   authority over the same exclusive domain, compilation fails (`E6003`).
3. **Magic bridges are explicit.** A magic system that affects physical state must declare
   a bridge from its custom dimension(s) to a physical dimension. A magic law that
   influences physics without a declared bridge fails (`E6004`).
4. **Custom reality is namespaced, versioned, deterministic.** Custom dimensions (e.g.
   `arcane::Aether`) are namespaced and never convert to SI without a declared bridge;
   this is consistent with the units rule in the language-semantics steering.
5. **Non-executable by default, explicitly.** A declared world law / magic module with no
   registered implementation is accepted structurally but flagged as unsupported
   (reusing the established "declaration, never simulated" pattern — see E8004/E9103/E9202).
   HPDL never behaves as if a law were simulated.
6. **One semantic validator.** All world-law checks run as one pass inside the single
   `hpdl_sema::Validator`, never duplicated per frontend.

## Scope boundaries

- **HPDL does not execute world laws or magic.** No alternative-physics solver runs; no
  state is evolved; no magic effect is applied. HPDL declares and validates the contract;
  the external HORIZON engine (and its registered modules) executes.
- **No universal / built-in magic system.** HPDL ships no default laws. Every law is an
  explicit, versioned module reference the author opts into.
- **No bridge arithmetic here.** A bridge *declares* a mapping contract (custom dimension →
  physical dimension, with the fixed-point lowering discipline that mapping will later
  obey). The actual unit conversion/solver is a runtime module concern; HPDL validates that
  a bridge is *declared* where one is required, not that it computes anything.
- **No new frontend syntax.** The grammar is not ratified; RON/`.hpdl` surface for world
  laws is features 04/05. This feature is model + sema + (optionally) PIR.
- **No registry / trust roots.** Which modules are "registered" and trusted is a registry
  concern (feature 14 / runtime). This feature uses a local notion of "supported" for the
  non-executable flag and defers the registry (see OD-11.4).

## Functional Requirements

### FR1: World-law declaration model (hpdl-model)

**FR1.1**: Introduce a `WorldLaw` declaration type. It carries:
- a versioned `ModuleRef` (reuse the existing type) naming the law module;
- a `LawMode` (see FR2);
- a deterministic, ordered set of claimed physical `LawDomain`s (`BTreeSet`);
- an optional `MagicBridge` (see FR3), `None` when the law touches no physical state.

**FR1.2**: Introduce a `LawDomain` — a validated, namespaced token naming a physical domain
a law may claim authority over (e.g. `physics.gravity`, `physics.thermodynamics`,
`arcane.aether`). Grammar: 1–64 chars, `[A-Za-z0-9_\-.]`, non-empty dotted segments.
Reject malformed domain tokens with a typed model error.

**FR1.3**: A body declares its world laws additively on `CelestialBody` via a builder
(`with_world_laws` / `with_world_law`), stored as a deterministic ordered collection.
Absence = the body declares no world laws (vacuum-standard physics is itself *not* assumed;
absence means "no custom law declared", consistent with the no-Earth-defaults rule).

**FR1.4**: World-law names/module refs within a body are deduplicated deterministically; a
law referenced twice is a conflict surfaced by the validator (reuse the existing module
duplicate rule where applicable — see FR5).

### FR2: Composition mode (additive vs replacement)

**FR2.1**: `LawMode` is an enum with exactly two forms per the steering:
- `Additive` — the law adds behavior without displacing any existing domain authority;
- `Replaces(BTreeSet<LawDomain>)` — the law is the single authoritative law for each named
  domain, explicitly displacing any default/standard behavior there.

**FR2.2**: **One authoritative law per exclusive domain.** Composition is intersection, not
last-write-wins (language-semantics steering). If two declared laws both claim authority
over the same domain — whether via `Replaces` on the same domain, or two laws both marked
authoritative for one domain — validation fails with `E6003 MultipleWorldLawsForDomain`.
The validator reports the conflicting domain and both laws.

**FR2.3**: An `Additive` law that nonetheless claims authority over a domain already owned by
a `Replaces` law is a contradiction and fails (same `E6003`). Compatible additive laws over
disjoint domains compose cleanly.

### FR3: Magic → physics bridge

**FR3.1**: Introduce a `MagicBridge` declaration mapping one or more custom (namespaced)
dimensions to a physical dimension. It names: the source custom dimension(s) (e.g.
`arcane.aether`), the target physical dimension (e.g. `energy`, `pressure`), and the
versioned bridge module (`ModuleRef`) that will perform the lowering at runtime.

**FR3.2**: **Magic affects physical state only through a declared bridge.** A `WorldLaw`
whose declared domains include a physical domain *and* whose source dimensions are custom
(magic) MUST declare a `MagicBridge`. A magic law that claims a physical domain with no
bridge fails with `E6004 MissingMagicBridge`. A purely-arcane law that claims only custom
domains (no physical effect) needs no bridge.

**FR3.3**: A bridge is a *contract*, not a computation. HPDL validates that the bridge is
declared, well-formed (valid dimensions, valid module ref), and that its target is a
recognized physical dimension; it performs no conversion. The fixed-point lowering the
bridge will obey (quantum, rounding, overflow policy per the determinism steering) is
recorded as contract metadata for the runtime, not executed here.

### FR4: Non-executable declarations (explicit)

**FR4.1**: A declared `WorldLaw` or `MagicBridge` whose module has no registered
implementation is **accepted structurally but flagged** as unsupported/non-executable,
reusing the established pattern (`E8004`, `E9103`, `E9202`). This feature adds an analogous
code in the E6000 world-law block (see FR5.3). HPDL never silently behaves as if the law ran.

**FR4.2**: The non-executable flag is a *warning-or-info-level* declaration note where the
law is otherwise well-formed (consistent with E8004/E9103 "accepted as a declaration, never
simulated"), and an *error* only when an invariant (FR2/FR3) is violated. The exact severity
is confirmed in design; it must not block a valid-but-not-yet-runtime declaration.

### FR5: Semantic validation (hpdl-sema — the one validator)

**FR5.1**: Replace the `ModuleValidator::validate_world_laws` stub with a real pass that
runs once inside the single `Validator`. It consumes the `WorldLaw` declarations on the body.

**FR5.2**: The pass enforces:
- domain-token well-formedness (malformed `LawDomain` → a typed diagnostic);
- **one authoritative law per exclusive domain** → `E6003 MultipleWorldLawsForDomain`;
- **magic bridge required for physical effect** → `E6004 MissingMagicBridge`;
- duplicate law/module references within a body (reuse `E6002 ConflictingModuleVersions`
  where it fits, or document a world-law-specific code — confirmed in design).

**FR5.3**: Diagnostics use the **existing, reserved** stable codes `E6003` and `E6004`
(already in `diagnostic.rs` / `DIAGNOSTIC_CODES.md`). If an "unsupported world-law module"
(FR4) code is needed, it is added as a new stable E6000 entry (e.g. `E6005
UnsupportedWorldLaw`) and documented — never by overloading an existing code.

### FR6: Determinism

**FR6.1**: All collections that can reach output or hashes use `BTreeSet`/`BTreeMap` (claimed
domains, replaced domains, bridge source dimensions). No `HashMap` iteration into canonical
output. No wall-clock, locale, or filesystem order.

**FR6.2**: No `f32`/`f64` anywhere in the model or validation. Any bridge lowering metadata
(quantum, rounding) is expressed as integers/enums per the fixed-point discipline. Custom
dimensions never implicitly coerce to SI (language-semantics steering).

**FR6.3**: Diagnostics are produced in the existing stable sorted order.

## Non-Functional Requirements

### NFR1 — Determinism: ordered collections; no floats; no ambient inputs; stable diagnostic
ordering; identical model ⇒ identical PIR bytes (if encoded, per OD-11.2).

### NFR2 — Security / untrusted input: validate every domain/dimension token length and
charset before use; bound the number of world laws, claimed domains, and bridge dimensions
per body; checked arithmetic; **no `unwrap`/`expect`/panic/indexing** on input paths; fail
closed with typed errors.

### NFR3 — Reliability: preserve the strict lints in `hpdl-model`/`hpdl-sema`
(`forbid(unsafe_code)`, `deny(missing_docs)`, no-unwrap/expect/indexing, no float
arithmetic). The new validation performs no float arithmetic.

### NFR4 — Documentation: rustdoc + examples on every new public item; `DIAGNOSTIC_CODES.md`
updated (E6003/E6004 descriptions refined to match the implemented behavior, plus any new
E6005); CHANGELOG updated; language-semantics "World laws & magic" section cross-checked.

### NFR5 — Golden fixtures: a golden fixture freezes a body declaring (a) an additive law,
(b) a replacement law, and (c) a magic law with a bridge, and roundtrips byte-for-byte.
Negative fixtures freeze the two-laws-one-domain and magic-without-bridge rejections.

## Constraints

- `hpdl-model` depends only on `thiserror` (+ `serde`); `hpdl-sema` depends on `hpdl-model`.
- Reuse `ModuleRef` for law/bridge module references; do not introduce a parallel ref type.
- `hpdl-pir` must not depend on `hpdl-compiler` / `hpdl-sema`.
- No world-law execution; no bridge arithmetic; no registry; no new frontend syntax.
- No universal/default magic system; every law is explicitly declared.

## Dependency & versioning implications (surfaced up front)

- **PIR format — DECISION REQUIRED (OD-11.2).** Adding world-law fields to the
  bincode-encoded `CelestialBody` changes the canonical byte layout (bincode writes a
  presence byte for every `Option`, so even an always-`None` field shifts bytes — exactly
  the reasoning that bumped PIR to **2.0** for feature 07's orbital fields). If world laws
  are encoded into PIR this feature, that is a deliberate format decision (fold into the
  unreleased 2.x vs. bump) with a reviewed golden-vector update and an ADR. If world laws
  are kept out of PIR for now (model + sema only), OD-11.2 is N/A and no golden vector
  changes. **This must be chosen, not defaulted silently.**
- **SemVer.** `hpdl-model`/`hpdl-sema` are pre-1.0; the additions are additive. No published
  contract-crate API changes unless OD-11.2 chooses to encode into PIR (then `hpdl-pir`
  format version moves and `cargo semver-checks -p hpdl-pir` applies).
- **Diagnostic stability.** `E6003`/`E6004` are already published as reserved codes; giving
  them behavior is not a breaking change. Any new code (E6005) is additive.

## Open decisions / approvals required — ALL RESOLVED (ADR 0005)

- **OD-11.1 — Domain taxonomy — RESOLVED: open namespaced tokens.** `LawDomain` is a validated
  token with a reserved `physics` physical prefix; no built-in closed domain enum.
- **OD-11.2 — PIR encoding — RESOLVED: keep world laws OUT of PIR (N/A, no byte change).** The
  `CelestialBody.world_laws` field is `#[serde(skip)]`, so canonical PIR (bincode) sees no new
  field — `PirVersion::CURRENT` (2.0), the `HPDL-PIR-v2` separator, and all PIR golden hashes
  are unchanged (verified). (`skip_serializing_if` was rejected: it breaks bincode's
  non-self-describing encode/decode.) Encoding later is a separate deliberate format decision.
- **OD-11.3 — Bridge expressiveness — RESOLVED: many sources → one physical target**, one
  bridge module. Richer N-to-M bridges deferred.
- **OD-11.4 — "Supported" vs registry — RESOLVED: all law modules non-executable now** (`E6005`,
  **warning** — never blocks a valid declaration); the registry / trust roots are deferred to
  feature 14.
- **OD-11.5 — Reuse vs new duplicate code — RESOLVED: reuse `E6002`** for a duplicated law
  reference; `E6003` strictly for the one-domain-two-authoritative-laws conflict.

## Success Criteria

1. ✅ `WorldLaw`, `LawMode`, `LawDomain`, `MagicBridge` exist in `hpdl-model` with validated
   constructors and typed errors; `CelestialBody` declares world laws additively; existing
   invariants/tests still hold.
2. ✅ `ModuleValidator::validate_world_laws` is implemented (stub removed) and wired into the
   single `Validator`; it enforces one-authoritative-law-per-domain and bridge-required-for-
   physical-effect.
3. ✅ Two laws claiming one exclusive domain → `E6003`; a magic law touching physics with no
   bridge → `E6004`; both exercised by tests.
4. ✅ A declared law with no registered implementation is accepted structurally and flagged
   non-executable (never simulated); severity does not block a valid declaration.
5. ✅ No universal/default magic; no world-law execution; no bridge arithmetic; no floats; no
   panics on malformed input; checked arithmetic; ordered collections.
6. ✅ Golden fixture (additive + replacement + magic-with-bridge) roundtrips byte-for-byte;
   negative fixtures freeze the two rejection cases.
7. ✅ OD-11.2 resolved via ADR; any PIR golden-vector / format-version change deliberate.
8. ✅ CI passes; `DIAGNOSTIC_CODES.md`, CHANGELOG, and the language-semantics cross-check
   updated.

## Out of Scope

- Executing world laws / alternative physics; running magic effects; evolving state
  (external HORIZON engine + registered modules).
- Bridge unit-conversion arithmetic / solvers (runtime modules).
- Module registry, trust roots, "is this module implemented" resolution (feature 14).
- Native `.hpdl` / RON surface syntax for world laws (features 04 / 05).
- CLI presentation of world-law diagnostics (feature 15).
- A built-in/standard magic system or default alternative-physics law (never shipped).

## References

- Steering: `product.md` (domain 16; "no universal magic system"), `architecture.md`
  (contracts vs implementations), `language-semantics.md` ("World laws & magic";
  custom dimensions never coerce to SI without a bridge), `determinism.md` (fixed-point
  lowering metadata).
- Existing stubs this feature makes real: `hpdl-sema::module_validator`
  (`validate_world_laws`), `hpdl-sema::DIAGNOSTIC_CODES.md` (E6003/E6004).
- Precedent for "declaration, never simulated" flags: `E8004` (orbital),
  `E9103` (environment module), `E9202` (evaluator).
- PIR format-version precedent: feature 07 orbital-fields bump to 2.0
  (`docs/rfcs/ADR-0003-pir-v2-orbital-fields.md`), feature 10 OD-10.2 (capabilities kept out
  of PIR).
