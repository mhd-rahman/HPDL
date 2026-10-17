# Actions and Events — Requirements

## Overview

This feature covers **typed action effects, capability-gated authorization, deterministic
event ordering, and signed replay records**. It has two clearly separable halves:

- **Definition-side (declaration + validation):** extend the existing `ActionDeclaration`
  with an explicit `ReadWriteSet` and a **capability requirement set** (what runtime
  capabilities the action needs to touch state), plus the semantic rules that go with them.
  This lives in `hpdl-model` / `hpdl-sema` and can be built now.
- **Runtime-side (execution + replay contracts):** `EventEnvelope`, `ActionId`, `Tick`,
  `StateHash` — the signed event records that bind module/version/input/output hashes, and
  the deterministic-ordering / replay machinery. These are **runtime contracts** consumed by
  the external HORIZON engine, and belong to `hpdl-runtime-api`, which the workflow steering
  marks **deferred** ("Do NOT scaffold crates marked deferred unless the task explicitly
  asks"). See the blocking decision **OD-10.1**.

Primary crates named by the design: `hpdl-model`, **`hpdl-runtime-api`**, `hpdl-pir`.

> **Critical framing:** HPDL defines and compiles; it does **not** execute state. Per the
> product/architecture steering, "a separate authoritative simulation engine evolves mutable
> state." Most of this feature's *required test cases* (unauthorized action, replay duplicate,
> deterministic ordering, cross-domain portal effects, event hash consistency) are **runtime
> execution behaviors**, not definition/validation behaviors. HPDL can define the *contracts*
> and validate the *declarations*; it cannot test execution without the runtime engine. This
> boundary is the heart of OD-10.1 and is reflected throughout.

## Goals

1. **Explicit action effects + capabilities**: an action declares its read set, write set,
   and the **capabilities** it requires to mutate state (least-privilege). Undeclared writes
   and status-score writes fail closed (the latter already enforced).
2. **One semantic validator**: capability/read-write validation added to the single
   `hpdl_sema::Validator` as one pass, never duplicated per frontend.
3. **Deterministic contracts**: the event/replay contract shapes (`EventEnvelope`, `Tick`,
   `StateHash`, `ActionId`) are specified with deterministic, integer, hash-bound fields —
   no floats, no ambient state — so that *if/when executed by the runtime*, replay is
   byte-identical.
4. **Explicit failure**: malformed capabilities, undeclared writes, and status writes produce
   typed diagnostics with stable codes. Unauthorized execution and replay-duplicate detection
   are specified as runtime contracts (enforced by the engine, not HPDL).
5. **Honor the deferred runtime crate**: do not scaffold `hpdl-runtime-api` unless OD-10.1
   explicitly chooses to; specify the runtime contracts regardless.

## Scope boundaries

- **HPDL does not execute actions or evolve state.** No action is run, no event is produced
  by HPDL, no state hash is computed from real state here. HPDL declares the contract and
  validates the declaration; the external engine executes.
- **Only runtime modules mutate state.** An action *declares* required capabilities and
  read/write fields; actual mutation authority is a runtime concern (capability checks at
  execution time are the engine's job, against the declared requirements).
- **No status-score edits.** Actions never write status fields (already enforced; preserved
  and reaffirmed here).
- **No signing implementation.** "Signed events" are specified as a contract (events bind
  module/version/input/output hashes); the cryptographic signing/verification mechanism is a
  runtime/registry concern (features 14 / runtime), not implemented here.
- **No new frontend syntax** (features 04 / 05).

## Functional Requirements

### FR1: ReadWriteSet (definition-side)

**FR1.1**: Introduce a `ReadWriteSet` abstraction wrapping the action's `reads` and `writes`
as deterministic ordered sets (`BTreeSet`). It exposes read/write field paths in sorted order
and is the single structured representation the validator and (future) runtime consume.

**FR1.2**: `ActionDeclaration` is refactored/extended to expose its reads/writes as a
`ReadWriteSet` without changing existing behavior (the status-write prohibition is preserved).
Existing `ActionDeclaration::new(name, reads, writes)` call sites keep working.

### FR2: Capability requirements (definition-side)

**FR2.1**: An action declares a set of required **capabilities** — named, namespaced tokens
describing what runtime authority it needs (e.g. `state.atmosphere.write`,
`portal.cross-domain`). `Capability` is a validated name newtype (namespaced,
`[A-Za-z0-9_\-.]`, 1–64 chars).

**FR2.2**: Capabilities are declared on `ActionDeclaration` additively (builder setter), as a
deterministic `BTreeSet<Capability>`. Absence = the action requires no special capability.

**FR2.3**: The declared write set must be *covered* by declared capabilities per the
documented rule (every state-mutating write path implies a corresponding declared capability).
A write with no backing capability is an **undeclared/unauthorized write** at the declaration
level and fails validation (this is the definition-side analogue of the design's "unauthorized
action" case — see FR5.2 for the runtime analogue).

### FR3: Runtime execution/replay contracts (SUBJECT TO OD-10.1)

**FR3.1**: Specify the following contract types (placement per OD-10.1):
- `ActionId` — a validated, stable identifier for an action instance.
- `Tick` — a monotonic integer time step (`u64`), the unit of deterministic ordering.
- `StateHash` — a content hash of canonical state bytes (reuse the PIR hashing discipline:
  named algorithm + domain separator; 64-char lowercase hex).
- `ReadWriteSet` — shared with FR1 (the runtime consumes the same structured set).
- `EventEnvelope` — a signed replay record binding: `action_id`, `tick`, the acting
  module+version, the input `StateHash`, the output `StateHash`, and the ordered read/write
  set. Deterministic, integer, hash-bound; no floats.

**FR3.2 (deterministic ordering)**: Events within a tick order deterministically by a
documented, total key (e.g. `(tick, action_id)` with a defined tiebreak) — never by hashmap
iteration or wall-clock. Specified here; enforced by the engine.

**FR3.3 (event hash consistency)**: An `EventEnvelope`'s binding is consistent iff recomputing
the hash over its canonical bytes reproduces the recorded value. Specified as a contract; a
pure, no-float verification helper may be provided on the contract type.

**FR3.4 (replay-duplicate / idempotence)**: The contract specifies that two events with the
same `(action_id, tick, input StateHash)` are the *same* event (replay duplicate), so a
replaying engine can detect and reject duplicates. HPDL specifies this identity; the engine
enforces it.

**FR3.5 (unauthorized action)**: The contract specifies that an event is only valid if the
acting module held the capabilities the action declared (FR2). HPDL validates the *declaration*
side; the *execution-time* authorization check is the engine's.

**FR3.6 (cross-domain portal effects)**: A capability like `portal.cross-domain` is a declared
requirement that gates effects crossing world-law/domain boundaries. HPDL validates that such
effects declare the capability; the engine enforces the boundary at runtime.

### FR4: Semantic validation (hpdl-sema, definition-side)

**FR4.1**: An `ActionCapabilityValidator` (or extension of the existing `ActionValidator`)
runs once in the single `Validator`:
- capability names well-formed (`MalformedCapability`);
- every mutating write is covered by a declared capability (`UncoveredWrite`);
- status writes forbidden (reaffirm existing E5000/E4005 behavior);
- read/write field paths well-formed.

**FR4.2**: Diagnostics use stable, documented codes (reuse the E5000 action block where it
fits; add an E9300 block for capability-specific checks).

### FR5: Determinism and replay (contract-level)

**FR5.1**: All contract types are integer- and hash-based; no `f32`/`f64`; ordered collections.

**FR5.2**: Given the same action declaration, same inputs (by `StateHash`), same tick, and
same acting module version, a replaying engine produces an identical `EventEnvelope` — the
replay invariant. HPDL guarantees the *shape* and *determinism rules* that make this possible.

## Non-Functional Requirements

### NFR1: Determinism — no floats; ordered collections; hash-bound events; documented total
ordering key. No wall-clock / locale / hashmap iteration in any canonical or contract bytes.

### NFR2: Security / untrusted input — validate capability/field/id lengths and formats and
hash formats before use; checked arithmetic on ticks; no panicking index/`unwrap`/`expect` on
input paths; bound capability/read/write set sizes.

### NFR3: Reliability — keep the existing strict lints in `hpdl-model`/`hpdl-sema`; the new
validation performs no float arithmetic. If `hpdl-runtime-api` is scaffolded (OD-10.1 B), it
is a published contract crate: minimal deps, semver-stable, `forbid(unsafe_code)`.

### NFR4: Documentation — rustdoc + examples on new public items; `DIAGNOSTIC_CODES.md` and
CHANGELOG updated.

### NFR5: Golden fixtures & replay — a golden fixture freezes an action with capabilities +
read/write set and roundtrips byte-for-byte. A contract-level replay/hash-consistency test
freezes an `EventEnvelope`'s canonical bytes + hash (location gated on OD-10.1).

## Constraints

- `hpdl-model` depends only on `thiserror` (+ `serde`); `hpdl-sema` depends on `hpdl-model`.
- `hpdl-pir` / `hpdl-runtime-api` must not depend on `hpdl-compiler`.
- `hpdl-runtime-api → hpdl-pir (+ selected stable model types)` only.
- No action execution; no state evolution; no signing implementation; no new frontend syntax.

## Dependency & versioning implications (surfaced up front)

- **Runtime-contract crate — BLOCKING DECISION (OD-10.1).** This feature is *substantially*
  runtime-facing: `EventEnvelope`, `ActionId`, `Tick`, `StateHash`, and the replay/ordering
  contracts are the runtime surface the design names `hpdl-runtime-api` for — which is
  deferred. Unlike feature 09 (where only one small type was runtime-facing), the *majority*
  of feature 10's new types belong to the runtime crate. See OD-10.1 options.
- **PIR impact — DECISION REQUIRED (OD-10.2), likely N/A for the definition side.** Adding a
  capability set to the bincode-encoded `ActionDeclaration` would change canonical PIR bytes
  (as in features 07/08). If capabilities are encoded, that is a PIR revision (fold into
  unreleased v2.0 vs. bump) handled deliberately with golden-vector updates. If capabilities
  are kept out of PIR for now, OD-10.2 is N/A. The event/replay contracts are runtime
  artifacts, independently versioned (state-schema versioning), not PIR.
- **SemVer.** `hpdl-model`/`hpdl-sema` pre-1.0; additive. `hpdl-runtime-api` (if scaffolded)
  is a new published-contract crate with its own semver + `cargo semver-checks`.

## Open decisions / approvals required

- **OD-10.1 `hpdl-runtime-api` — RESOLVED: Option B (activate the crate).** Approved to begin
  runtime integration. `hpdl-runtime-api` is scaffolded as a published contract crate
  (depends on `hpdl-pir` + selected stable `hpdl-model` types), the event/replay contracts
  (`EventEnvelope`, `ActionId`, `Tick`, `StateHash`, `ReadWriteSet`) are implemented there,
  the workflow crate-status table is updated (`deferred` → active) and the workspace
  `Cargo.toml` member is uncommented, and the decision is recorded in an ADR
  (`docs/rfcs/0004-activate-hpdl-runtime-api.md`). **Execution is still external:** HPDL does
  not run actions; the required *execution* behaviors (ordering, replay-duplicate detection,
  authorization) are exercised against the real contract types by a clearly-labelled **mock
  executor in tests** that stands in for the HORIZON engine. Feature 09's `StatusResult` may
  also move here as follow-up (tracked separately).
- **OD-10.2 PIR revision for capabilities** (fold into v2.0 vs. bump vs. keep out of PIR),
  only relevant if the capability set is encoded into PIR this feature.
- **OD-10.3 Capability model:** confirm the capability token grammar and the write→capability
  coverage rule (is coverage path-prefix based, exact, or registry-defined?). Default: a
  documented prefix/namespacing rule; registry-defined capabilities deferred to feature 14.
- **OD-10.4 StateHash algorithm:** reuse PIR's SHA-256 + domain separator discipline, with a
  state-specific domain separator. Confirm before stabilizing the runtime contract.
- **OD-10.5 Signing:** "signed events" — confirm whether signatures are in-scope contract
  fields now or deferred to the runtime/registry. Default: specify the hash-binding; defer
  cryptographic signatures.

## Success Criteria

1. ✅ `ReadWriteSet` + `Capability` exist in `hpdl-model`; `ActionDeclaration` exposes a
   read/write set and declares required capabilities; existing invariants/tests hold.
2. ✅ `hpdl_sema::Validator` validates capability well-formedness and write→capability
   coverage, reaffirms the status-write prohibition, with stable typed codes.
3. ✅ The runtime execution/replay contracts (`EventEnvelope`, `ActionId`, `Tick`, `StateHash`)
   are specified with deterministic, integer, hash-bound fields; their *implementation and
   placement* follow OD-10.1.
4. ✅ No action is executed and no state is evolved by HPDL; the required *execution* test
   cases are documented as runtime-engine responsibilities, with the definition-side analogues
   (undeclared/uncovered write, declaration-level authorization) tested here.
5. ✅ No panics on malformed input; no floats; checked arithmetic.
6. ✅ Golden fixture for an action with capabilities + read/write set roundtrips; contract-level
   hash-consistency/replay test located per OD-10.1.
7. ✅ OD-10.1 (and OD-10.2 if applicable) resolved via ADRs; any PIR golden-vector change
   deliberate.
8. ✅ CI passes; CHANGELOG + `DIAGNOSTIC_CODES.md` updated.

## Out of Scope

- Executing actions, producing events, computing real state hashes, enforcing authorization or
  replay-duplicate rejection at runtime (external HORIZON engine).
- Cryptographic signing/verification of events (runtime / feature 14).
- Capability registry / trust roots (feature 14).
- Native `.hpdl` / RON syntax for capabilities (features 05 / 04).
- CLI presentation (feature 15).

## References

- Design Doc: `docs/design/10-actions-and-events.md`
- Existing action work: `hpdl-model::ActionDeclaration`, `hpdl-sema::action_validator`,
  `hpdl-sema::status_validator` (status-write prohibition, E4005/E5000).
- Deferred runtime crate: workflow steering crate-status table; `.kiro/specs/09-status-and-vitality`
  (OD-09.1 precedent for a runtime-facing type kept pre-runtime-api).
- Determinism & hashing: `docs/rfcs/0003-pir-v2-orbital-fields.md`, `hpdl-pir` hashing.
