# World Laws & Custom Reality — Design

Implements domain 16 (world laws, magic, alternative physics). This is a
**definition + validation** feature across two crates:

- **`hpdl-model`** — the typed declaration vocabulary: `WorldLaw`, `LawMode`,
  `LawDomain`, `MagicBridge`, plus their validated constructors and new error
  variants, and the `CelestialBody` builder that attaches world laws.
- **`hpdl-sema`** — makes the existing `ModuleValidator::validate_world_laws` stub
  real and wires it into the single `Validator`, using the already-reserved
  `E6003`/`E6004` diagnostic codes (plus one additive `E6005`).

HPDL ships **contracts, not an executor**: a declared law with no registered
implementation is accepted and flagged, never simulated. There is no built-in magic.

## 1. Crate placement

| Concern | Crate | Module |
|---|---|---|
| `LawDomain` (validated namespaced token) | `hpdl-model` | `world_law.rs` |
| `LawMode` (`Additive` / `Replaces`) | `hpdl-model` | `world_law.rs` |
| `MagicBridge` (custom→physical dimension contract) | `hpdl-model` | `world_law.rs` |
| `WorldLaw` (module ref + mode + domains + optional bridge) | `hpdl-model` | `world_law.rs` |
| World-law field + builder on the body | `hpdl-model` | `celestial_body.rs` (extended) |
| New model errors | `hpdl-model` | `error.rs` |
| World-law validation (stub → real) | `hpdl-sema` | `module_validator.rs` (extended) |
| Any new diagnostic code (E6005) | `hpdl-sema` | `diagnostic.rs` |

**Why `ModuleRef` is reused:** a world law and a magic bridge both *name a versioned
module*. `ModuleRef` already validates name/version with the right discipline; a parallel
ref type would drift. (Same reasoning as feature 08 reusing `ModuleRef` for climate models.)

## 2. Model types (hpdl-model)

```text
LawDomain(String)                  // 1..=64, [A-Za-z0-9_\-.], non-empty dotted segments
                                   // e.g. "physics.gravity", "arcane.aether"

LawMode =
    | Additive                     // adds behavior; claims no exclusive domain authority
    | Replaces(BTreeSet<LawDomain>)// the single authoritative law for each named domain

MagicBridge {
    sources: BTreeSet<LawDomain>,  // custom (magic) dimensions, e.g. {arcane.aether}
    target: LawDomain,             // physical dimension, e.g. physics.energy
    module: ModuleRef,             // versioned bridge module (performs lowering at runtime)
}
    - new(sources, target, module) validates: non-empty sources, every token well-formed,
      target is a *physical* namespace (OD-11.1 taxonomy), module ref valid.
    - HPDL never computes the mapping; it records the contract.

WorldLaw {
    module: ModuleRef,             // the law module
    mode: LawMode,
    domains: BTreeSet<LawDomain>,  // physical/custom domains this law claims authority over
    bridge: Option<MagicBridge>,   // Some iff the law reaches physical state from magic
}
    - new(module, mode, domains, bridge) validates every token + the bridge if present.
    - authoritative_domains() -> &BTreeSet<LawDomain>: domains the law is authoritative for
      (the Replaces set, plus any domain it exclusively claims). Used by the validator.
    - touches_physical() -> bool: any claimed/target domain in the physical namespace.
    - is_magic() -> bool: any source domain in a custom (non-physical) namespace.

CelestialBody (extended): + world_laws: Vec<WorldLaw>
    - builder: with_world_law / with_world_laws (deterministic, insertion order preserved
      in the Vec; the validator derives ordered sets from it).
```

**Domain taxonomy (OD-11.1 → open namespaced tokens).** No closed enum. A token whose first
segment is `physics` (reserved physical namespace) is "physical"; a token whose first segment
is anything else (e.g. `arcane`, `aether`, a world's custom namespace) is "custom". This
keeps alternative physics open-ended while giving the bridge rule something concrete to test.
The reserved physical prefix is documented, not hard-coded as an exhaustive list of domains.

**No floats / no coercion.** Custom dimensions never implicitly convert to SI; the `target`
of a bridge is a declared physical dimension and the *conversion is a runtime module*. Any
lowering metadata the bridge carries (quantum, rounding, overflow) is integer/enum, per the
determinism steering — but see §7: this feature records the bridge *module ref*, and defers
embedding full fixed-point lowering metadata unless OD-11.3 expands it.

## 3. Validation flow (hpdl-sema)

`ModuleValidator::validate_world_laws(definition)` is implemented and called from the single
`Validator` alongside `validate_modules`. For a body's `world_laws`:

1. **Token well-formedness** — every `LawDomain` on every law and bridge is re-checked (the
   model already validates at construction; the validator defends against hand-built /
   deserialized values). Malformed → a typed diagnostic.
2. **Duplicate law refs** — a module referenced by two laws → `E6002 ConflictingModuleVersions`
   (OD-11.5: reuse, don't invent).
3. **One authoritative law per exclusive domain (`E6003`)** — build a
   `BTreeMap<LawDomain, Vec<&WorldLaw>>` of *authoritative* claims (every `Replaces` domain,
   plus exclusive claims). Any domain with more than one authoritative law →
   `E6003 MultipleWorldLawsForDomain`, reporting the domain and the conflicting law module
   names. An `Additive` law that claims a domain already owned by a `Replaces` law is the
   same conflict (FR2.3).
4. **Magic bridge required (`E6004`)** — for each law where `is_magic() && touches_physical()`
   and `bridge.is_none()` → `E6004 MissingMagicBridge`, naming the law and the physical
   domain it reaches without a bridge.
5. **Non-executable flag (`E6005`)** — every well-formed declared law/bridge whose module has
   no registered implementation (OD-11.4: *all* of them at this stage) emits
   `E6005 UnsupportedWorldLaw` at **info/warning** severity: "accepted as a declaration, never
   simulated." This never blocks a valid declaration; it mirrors `E8004`/`E9103`/`E9202`.

Diagnostics flow through the existing deterministic sort (span, code, message).

### 3.1 Diagnostic codes

| Code | State | Meaning |
|---|---|---|
| `E6002 ConflictingModuleVersions` | existing | A law module is referenced more than once. |
| `E6003 MultipleWorldLawsForDomain` | **reserved → implemented** | >1 authoritative law for one exclusive domain. |
| `E6004 MissingMagicBridge` | **reserved → implemented** | A magic law reaches physical state with no declared bridge. |
| `E6005 UnsupportedWorldLaw` | **new, additive** | A declared law/bridge module has no registered implementation; accepted, never simulated. |

`E6003`/`E6004` already exist in `diagnostic.rs` and `DIAGNOSTIC_CODES.md` as reserved
placeholders — this feature gives them behavior (not a breaking change). `E6005` is a new,
additive stable code.

## 4. The invariants, and where each is enforced

| Invariant (steering) | Where | Kind |
|---|---|---|
| One authoritative law per exclusive domain | sema `E6003` | validation |
| Magic affects physics only through a declared bridge | sema `E6004` | validation |
| Modes are exactly `additive` / explicit replacement | `LawMode` type | model (type-level) |
| No universal magic system | no built-in laws shipped | design |
| Custom dimensions never coerce to SI without a bridge | `MagicBridge` required; no float coercion | model + sema |
| Declaration never executes/simulates | `E6005` flag; no executor | design |
| Composition is intersection, not last-write-wins | `E6003` conflict (no silent override) | validation |

## 5. Determinism & security

- `BTreeSet<LawDomain>` for claimed/replaced/source domains; `Vec<WorldLaw>` on the body with
  the validator deriving sorted maps — never `HashMap` iteration into output.
- No `f32`/`f64`; the model keeps the crate's `deny(clippy::float_arithmetic)`.
- Bounds: cap world laws per body, domains per law, and bridge sources (constants in
  `world_law.rs`), returning typed errors — no unbounded work on untrusted/deserialized
  definitions. No `unwrap`/`expect`/indexing on input paths.

## 6. PIR impact (OD-11.2) — RESOLVED: keep out of PIR (no byte change)

**World laws are NOT lowered into canonical PIR this feature.** Implemented by marking
`CelestialBody.world_laws` `#[serde(skip)]`, so bincode sees no new field and PIR bytes/hashes
are byte-identical (verified). Rationale, mirroring feature 10's OD-10.2:

- World-law declarations are *validation surface*; nothing in the current pipeline needs them
  in the hashed PIR payload yet.
- Adding a `world_laws` field to the bincode-encoded `CelestialBody` **would** change the
  byte layout (bincode writes a presence/length prefix even for empty collections), which is
  precisely what forced the **2.0** bump for feature 07's orbital fields. Doing that is a
  deliberate format-version decision with a reviewed golden-vector update — not something to
  slip in as a side effect.
- Therefore: this feature adds the field to the model but **does not serialize it** at all
  (`#[serde(skip)]`), so there is no new field in the PIR byte layout. The golden PIR vectors do
  **not** change, confirmed by the `hpdl-pir` `golden_hash` suite passing unchanged.

> **Implementation note:** `#[serde(skip_serializing_if = "Vec::is_empty")]` was attempted first
> and **broke the format** — bincode is not self-describing, so a conditional skip writes nothing
> on encode but the decoder still reads a length prefix, hitting EOF (two PIR decode tests
> failed). `#[serde(skip)]` is symmetric (encode writes nothing, decode defaults to empty) and is
> the correct byte-safe choice. If world laws are wanted in PIR later, that is a separate,
> deliberate **encode** decision (format-version bump/fold + reviewed golden-vector update + ADR).

## 7. What this design deliberately does NOT do

- **No executor / solver.** HPDL does not run world laws, apply magic, or evolve state. The
  external HORIZON engine and its registered modules do.
- **No bridge arithmetic.** A `MagicBridge` is a declared contract (sources, target, module);
  the actual custom→physical lowering is a runtime module. Full fixed-point lowering metadata
  on the bridge is deferred unless OD-11.3 expands bridge expressiveness.
- **No registry / trust roots.** "Is this module implemented?" is deferred to feature 14; this
  feature treats all law modules as non-executable (always `E6005`).
- **No built-in / universal magic.** No default laws ship.
- **No new frontend syntax** (RON/`.hpdl` surface is features 04/05).
- **No PIR byte change** under the default resolution of OD-11.2.
- **No `f32`/`f64`; no panicking paths.**

## 8. Decisions (resolved — ADR 0005)

- **OD-11.1 → open namespaced tokens** with a reserved `physics` physical prefix; no closed
  domain enum.
- **OD-11.2 → N/A (keep out of PIR this feature), implemented via `#[serde(skip)]`.** PIR
  (bincode) sees no new field, so `PirVersion::CURRENT` (2.0), the `HPDL-PIR-v2` separator, and
  all PIR golden hashes are unchanged (verified by the `hpdl-pir` `golden_hash` suite).
  `#[serde(skip_serializing_if)]` was tried and rejected — it desynchronizes bincode's
  non-self-describing encode/decode (decode reads a length prefix that encode never wrote) and
  broke two PIR decode tests. `#[serde(skip)]` is symmetric and byte-safe.
- **OD-11.3 → many sources → one physical target**, one bridge module. Richer bridges
  deferred.
- **OD-11.4 → all law modules non-executable at this stage** (`E6005`, **warning**); real
  registry deferred to feature 14.
- **OD-11.5 → reuse `E6002`** for duplicate law refs; `E6003` strictly for the
  one-domain-two-authoritative-laws conflict.
