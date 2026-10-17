# Module Runtime Contracts (Vitality & Process) — Requirements

## Overview

This feature covers **domain 12 of the v1 architectural scope: vitality / process
contracts**. It is the *runtime-contract* counterpart to feature 09 (status/vitality
*declarations*): the stable types the external HORIZON engine uses to **run** the modules a
definition references — versioned **processes** that evolve mutable state over ticks, and
versioned **vitality evaluators** that derive `StatusResult`s — plus the deterministic,
hash-bound **receipts/outcomes** they produce.

These contracts live in **`hpdl-runtime-api`** (active since feature 10, ADR 0004 — the
crate the architecture steering assigns "state/module/event/migration-execution contracts").
This feature also executes the move that feature 09 explicitly deferred: `StatusResult`
migrates from its provisional home in `hpdl-model` into `hpdl-runtime-api` (OD-09.1 noted
this would happen "when runtime integration begins" — it has begun).

Primary crate: **`hpdl-runtime-api`**. Supporting change: `hpdl-model` (relocate
`StatusResult`; the status-vitality *declaration* types stay in `hpdl-model`).

> **Critical framing (product / architecture / migrations steering):** HPDL defines
> *contracts*; a separate authoritative simulation engine executes them. "A declaration
> never executes or simulates anything." This feature does **not** add an executor, a solver,
> or any state evolution. It defines the executable *interfaces, receipts, and failure
> contracts* (migrations steering: that layer is owned by `hpdl-runtime-api`) and exercises
> them with a **mock executor test double**, exactly as feature 10 did for events. Unsupported
> or unregistered modules yield explicit activation/validation errors — never silent
> success.

## Goals

1. **Process contract**: a stable `ProcessContract` describing a versioned process module a
   definition references — its identity (name + version), the deterministic read/write set
   it is permitted to touch (reusing `ReadWriteSet` / `Capability`), and the activation tick
   from which it is live. HPDL declares the contract; the engine runs the process.
2. **Vitality-evaluator contract**: a stable `EvaluatorContract` describing a versioned
   evaluator the engine invokes to derive a status — its evaluator module+version, the input
   state hash it reads, the baseline it measures against, and the window — producing a
   `StatusResult`. Reuses the feature-09 declaration vocabulary.
3. **Deterministic receipts**: `ProcessReceipt` (and the relocated `StatusResult`) are the
   hash-bound outputs of running a process / evaluator at a tick: input/output `StateHash`,
   tick, acting module+version, and a binding hash (reusing the event hashing discipline).
   Same inputs ⇒ identical receipt bytes (deterministic replay).
4. **Explicit activation/failure**: a process/evaluator that is unsupported, inactive at the
   requested tick, or whose effects exceed its declared read/write set yields a typed
   `Error` — never silent no-op or silent success.
5. **Relocate `StatusResult`**: move the runtime-facing `StatusResult` from `hpdl-model` into
   `hpdl-runtime-api`, re-exporting from `hpdl-model` if needed for backward compatibility, so
   the runtime-output contracts live together (OD-12.4).
6. **One home for runtime contracts**: everything executable-contract-shaped lives in
   `hpdl-runtime-api`; the crate still ships **no engine**.

## Scope boundaries

- **HPDL does not execute processes or evaluators, or evolve state.** No tick loop, no
  solver, no state mutation, no status computation. HPDL defines the contract and the receipt
  shape; the external engine executes and the crate's tests use a **mock** executor double.
- **No scheduling / ordering engine.** Processes declare an activation tick and a read/write
  set; the engine orders and runs them. This feature specifies the deterministic
  *ordering key* and the receipt, not a scheduler.
- **No module registry / resolution.** "Which module implements this contract, and is it
  trusted" is a registry concern (feature 14). An unsupported/unregistered contract is a
  typed error here; the registry itself is out of scope.
- **No migration execution.** Migration *execution* receipts are related (migrations steering
  assigns them to this crate) but are owned by feature-13/governance work; this feature is
  limited to process + vitality run contracts. The receipt shape is designed to be reusable.
- **No new frontend syntax, no PIR change.** Runtime contracts are runtime artifacts
  (independently versioned state-schema surface), not PIR payload (as in feature 10).

## Functional Requirements

### FR1: Process contract (`ProcessContract`)

**FR1.1**: Introduce `ProcessContract` in `hpdl-runtime-api`: a versioned process module
reference (`module_name` + `module_version`), a `ReadWriteSet` (the deterministic read/write
field paths + capabilities it is permitted to touch), and an activation `Tick`.

**FR1.2**: Construction validates the module name/version shape (reuse the `ModuleRef`
discipline or an equivalent validated token) and that the read/write set is well-formed.
Bounds are enforced; no panics on untrusted input.

**FR1.3**: `ProcessContract` exposes `permits_write(path)` — whether a given write path is
covered by its declared read/write set (reusing `ReadWriteSet::uncovered_writes` /coverage).
A process effect outside its declared writes is an **unauthorized effect** at the contract
level (the runtime analogue of the definition-side uncovered-write check).

**FR1.4**: `ProcessContract` exposes `is_active_at(tick)` — whether the process is live at a
given tick (`tick >= activation_tick`). An inactive process yields an explicit error at run
time (engine), specified here as a contract.

### FR2: Vitality-evaluator contract (`EvaluatorContract`)

**FR2.1**: Introduce `EvaluatorContract`: the evaluator module+version (reuse feature-09
`EvaluatorRef`), the `StatusDimension` it measures, the `BaselineRef` it measures against,
and the `EvaluationWindow`. These are the feature-09 *declaration* types, re-exported, so the
runtime consumes the same vocabulary with no duplication.

**FR2.2**: An `EvaluatorContract` validates that its parts are well-formed. It does **not**
compute a status — producing a `StatusResult` is the engine's job. The contract is the stable
input shape the engine binds a result to.

### FR3: Deterministic receipts

**FR3.1**: Introduce `ProcessReceipt`: the hash-bound record of one process run at one tick —
the `ActionId`/process-instance id, `Tick`, acting `module_name`+`module_version`, input and
output `StateHash`, the `ReadWriteSet` touched, and a binding hash. It reuses the event
hashing discipline (SHA-256 + a dedicated domain separator distinct from the PIR and event
separators) and provides a pure `is_consistent()` check. No floats; no panics.

**FR3.2 (deterministic ordering)**: Process receipts within/across ticks order by a
documented total key (e.g. `(tick, process_id)`), never by hashmap iteration or wall-clock.

**FR3.3 (replay identity)**: Two receipts with the same `(process_id, tick, input StateHash)`
are the same run (replay duplicate) — the engine detects/rejects duplicates using this
identity (same pattern as `EventEnvelope::is_replay_duplicate_of`).

**FR3.4 (relocated `StatusResult`)**: `StatusResult` — the evaluator output contract from
feature 09 — moves into `hpdl-runtime-api` as the vitality receipt. Its replay invariant
(same evaluator version + sorted input hashes + window ⇒ identical result) is preserved.

### FR4: Explicit activation & failure contracts

**FR4.1**: New typed `Error` variants in `hpdl-runtime-api` for: unsupported/unregistered
process or evaluator module, process inactive at the requested tick, and a process effect
outside its declared read/write set. Each is explicit and typed — **never** a silent no-op.

**FR4.2**: The crate does not decide *which* modules are supported (registry = feature 14);
it provides the error *shapes* and the predicates (`is_active_at`, write-coverage) the engine
uses to raise them.

### FR5: Determinism & security

**FR5.1**: No `f32`/`f64`; ordered collections (`BTreeSet`/`BTreeMap`); a documented total
ordering key; no wall-clock/locale/hashmap iteration in any contract bytes or hash input.

**FR5.2**: Validate all ids/module tokens/hash shapes before use; checked arithmetic on ticks;
no `unwrap`/`expect`/panic/indexing on input paths; bound read/write set sizes.

## Non-Functional Requirements

### NFR1 — Determinism: receipts are integer/hash-bound and byte-reproducible; the binding
uses a dedicated domain separator (`HPDL-PROCESS-v1`), never the PIR (`HPDL-PIR-v2`) or event
(`HPDL-EVENT-v1`) separators, so cross-domain hashes can never collide.

### NFR2 — Security / untrusted input: fail closed with typed errors; bound collections; no
panicking paths; no float arithmetic (preserve the crate's strict lints).

### NFR3 — Reliability / contract stability: `hpdl-runtime-api` stays a published,
semver-stable contract crate with minimal deps (`hpdl-pir`, `hpdl-model`, `serde`, `sha2`,
`thiserror`); `forbid(unsafe_code)`; no dependency on parser/compiler/sema.

### NFR4 — Documentation: rustdoc + examples on every new public item; CHANGELOG updated; the
`StatusResult` relocation documented with its backward-compatibility story.

### NFR5 — Tests: contract unit tests + an extended **mock executor** test double exercising:
unsupported/inactive process rejection, out-of-declared-writes rejection, deterministic
receipt ordering, replay-duplicate detection, and receipt-hash consistency — against the real
contracts, never a shipped engine.

## Constraints

- `hpdl-runtime-api → hpdl-pir (+ selected stable hpdl-model types)` only; no reverse edges;
  no dependency on `hpdl-compiler`/`hpdl-sema`/`hpdl-ron`/`hpdl-syntax`.
- Reuse `ReadWriteSet`, `Capability`, `EvaluatorRef`, `BaselineRef`, `EvaluationWindow`,
  `StatusDimension`, `Digest`, `ModuleRef` from `hpdl-model` — do not introduce parallels.
- No executor, no solver, no state evolution, no scheduler, no module registry, no migration
  execution, no new frontend syntax, no PIR change.

## Dependency & versioning implications (surfaced up front)

- **`StatusResult` relocation (OD-12.4).** Moving `StatusResult` out of `hpdl-model` is a
  pre-1.0 API change to `hpdl-model` and an additive change to `hpdl-runtime-api`. To avoid
  breaking existing `hpdl-model` consumers (feature-09 tests, docs), `hpdl-model` can
  **re-export** `hpdl_runtime_api::StatusResult` — but that would make `hpdl-model` depend on
  `hpdl-runtime-api`, **inverting the dependency direction** (runtime-api depends on model).
  That is unacceptable. Options under OD-12.4 resolve this.
- **No PIR change.** Receipts/contracts are runtime artifacts, not PIR payload; no PIR format
  version impact (as in feature 10 OD-10.2 / feature 11 OD-11.2).
- **SemVer.** `hpdl-runtime-api` is a pre-1.0 published contract crate; additions are
  additive and `cargo semver-checks -p hpdl-runtime-api` applies.

## Open decisions / approvals required — ALL RESOLVED (ADR 0006)

- **OD-12.1 — Process identity — RESOLVED: dedicated `ProcessId`** (same grammar as
  `ActionId`, distinct type) so process receipts and action events are never conflated.
- **OD-12.2 — Receipt vs event — RESOLVED: distinct `ProcessReceipt`** with its own
  `HPDL-PROCESS-v1` domain separator (distinct from `HPDL-EVENT-v1` and `HPDL-PIR-v2`); shared
  hashing helpers factored into an internal `hashing` module, no `EventEnvelope` behavior
  change.
- **OD-12.3 — Module reference — RESOLVED: `ModuleRef`** for process/evaluator identity;
  `EventEnvelope` keeps its existing flat module fields.
- **OD-12.4 — `StatusResult` relocation — RESOLVED: moved to `hpdl-runtime-api`, no
  `hpdl-model` re-export** (re-exporting would invert the dependency direction). The feature-09
  *declaration* types stay in `hpdl-model`; the type's tests/doctest moved with it; the only
  workspace references (model lib table + module doc) were updated.
- **OD-12.5 — Activation-tick — RESOLVED: inclusive** (`tick >= activation_tick`;
  `activation_tick` is the first live tick).

## Success Criteria

1. ✅ `ProcessContract`, `EvaluatorContract`, `ProcessReceipt` (and a `ProcessId` per OD-12.1)
   exist in `hpdl-runtime-api` with validated constructors, typed errors, and rustdoc.
2. ✅ `ProcessContract` enforces write-coverage (`permits_write`) and activation
   (`is_active_at`); `ProcessReceipt` has a dedicated-separator binding + `is_consistent()` +
   a total ordering key + replay-duplicate identity.
3. ✅ `StatusResult` lives in `hpdl-runtime-api` (OD-12.4); the dependency direction is
   preserved (no `hpdl-model → hpdl-runtime-api` edge); feature-09 references updated.
4. ✅ Unsupported/inactive/over-write cases are explicit typed errors; nothing silently
   succeeds or no-ops; no engine is shipped.
5. ✅ No floats; no panics on malformed input; checked arithmetic; ordered collections;
   dedicated hash domain separator distinct from PIR and event separators.
6. ✅ A mock-executor test double exercises the required behaviors against the real contracts;
   contract unit tests cover ids/coverage/activation/receipt consistency/ordering/replay.
7. ✅ No PIR byte change; `hpdl-runtime-api` stays minimal and semver-stable.
8. ✅ CI passes; CHANGELOG + ADR updated; OD-12.4 resolved via ADR.

## Out of Scope

- Executing processes/evaluators, evolving state, computing status scores, scheduling, or
  ordering at run time (external HORIZON engine).
- Module registry / trust roots / resolution (feature 14).
- Migration execution receipts/authorization (feature 13 / governance).
- Native `.hpdl` / RON syntax; CLI presentation (features 04/05/15).
- Any PIR format change.

## References

- Steering: `architecture.md` (contracts vs implementations; `hpdl-runtime-api` owns runtime
  contracts), `product.md` (domain 12 "Vitality/process contracts"), `migrations.md`
  (execution interfaces/receipts/failure contracts owned by `hpdl-runtime-api`),
  `determinism.md` (domain-separated hashing; no floats).
- Builds on: feature 10 (`hpdl-runtime-api` activation, ADR 0004; `EventEnvelope`,
  `ReadWriteSet`, `Capability`, hashing discipline, mock-executor pattern) and feature 09
  (`StatusResult`, `EvaluatorRef`, `BaselineRef`, `EvaluationWindow`, `StatusDimension`;
  OD-09.1 deferred the `StatusResult` move this feature performs).
