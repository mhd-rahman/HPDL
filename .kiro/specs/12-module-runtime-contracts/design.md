# Module Runtime Contracts (Vitality & Process) — Design

Implements domain 12 (vitality / process contracts) in **`hpdl-runtime-api`** — the
runtime-contract counterpart to feature 09's declarations. HPDL ships **contracts, not an
engine**: this feature adds no executor, solver, scheduler, or state evolution. It defines
the executable *interfaces, receipts, and failure contracts* the migrations/architecture
steering assigns to this crate, and exercises them with a **mock executor** test double.

## 1. Crate placement

| Concern | Crate | Module |
|---|---|---|
| `ProcessId` (validated process-instance id) | `hpdl-runtime-api` | `ids.rs` (extended) |
| `ProcessContract` (module + rwset + activation tick) | `hpdl-runtime-api` | `process.rs` |
| `EvaluatorContract` (evaluator + dimension + baseline + window) | `hpdl-runtime-api` | `process.rs` |
| `ProcessReceipt` (hash-bound process-run record) | `hpdl-runtime-api` | `process.rs` |
| `StatusResult` (relocated from `hpdl-model`) | `hpdl-runtime-api` | `vitality.rs` |
| New error variants | `hpdl-runtime-api` | `error.rs` |
| Shared hashing helper (`feed`, hex) | `hpdl-runtime-api` | `event.rs` → small `hashing.rs` |
| Mock executor (test double) | `hpdl-runtime-api` | `tests/mock_process_executor.rs` |

**Reuse, don't duplicate.** `ProcessContract`/`EvaluatorContract`/`ProcessReceipt` reuse
`ReadWriteSet`, `Capability`, `ModuleRef`, `EvaluatorRef`, `BaselineRef`, `EvaluationWindow`,
`StatusDimension`, `Digest`, `Tick`, `StateHash` — all already in `hpdl-model` or
`hpdl-runtime-api`. No parallel types.

## 2. Types (hpdl-runtime-api)

```text
ProcessId(String)                  // same grammar as ActionId (1..=128, [A-Za-z0-9_\-.:])
                                   // distinct type so process runs ≠ action events (OD-12.1)

ProcessContract {
    module: ModuleRef,             // versioned process module (OD-12.3: ModuleRef, not flat)
    read_write: ReadWriteSet,      // permitted reads/writes + capabilities
    activation_tick: Tick,
}
    - new(module, read_write, activation_tick)  (ReadWriteSet already validated by model)
    - permits_write(path) -> bool               // covered by the declared write/capability set
    - is_active_at(tick) -> bool                // tick >= activation_tick (OD-12.5 inclusive)
    - module(), read_write(), activation_tick() accessors

EvaluatorContract {
    evaluator: EvaluatorRef,       // versioned evaluator module (feature 09)
    dimension: StatusDimension,
    baseline: BaselineRef,
    window: EvaluationWindow,
}
    - new(...) stores validated parts; computes nothing (engine derives the StatusResult)

ProcessReceipt {
    process_id: ProcessId,
    tick: Tick,
    module: ModuleRef,
    input_state: StateHash,
    output_state: StateHash,
    read_write: ReadWriteSet,
    binding: StateHash,            // SHA-256(HPDL-PROCESS-v1 || length-prefixed fields)
}
    - new(...) computes the binding
    - is_consistent() -> bool                   // recompute binding, compare (pure, no panic)
    - is_replay_duplicate_of(other) -> bool     // same (process_id, tick, input_state)
    - order_key() -> ProcessOrderKey { tick, process_id }   // total ordering (FR3.2)

StatusResult { .. }                // relocated verbatim from hpdl-model (OD-12.4)
```

**Hashing (NFR1).** `ProcessReceipt::binding` uses a dedicated separator
`HPDL-PROCESS-v1`, distinct from `HPDL-PIR-v2` and `HPDL-EVENT-v1`, so a process receipt, an
event binding, and a PIR content hash can never collide. The length-prefixed field feed and
lowercase-hex helpers currently private in `event.rs` are factored into a small internal
`hashing` module and shared (no behavior change to `EventEnvelope`).

## 3. `StatusResult` relocation (OD-12.4)

**Decision: move `StatusResult` into `hpdl-runtime-api`; do NOT re-export from `hpdl-model`.**

- `StatusResult` is a *runtime output* contract — feature 09 placed it in `hpdl-model` only
  because `hpdl-runtime-api` was deferred (OD-09.1 option A, explicitly "moves to
  `hpdl-runtime-api` when runtime integration begins"). It has begun.
- Re-exporting it from `hpdl-model` would require `hpdl-model` to depend on
  `hpdl-runtime-api`, **inverting** the dependency direction (`runtime-api → model`). That is
  forbidden by the architecture steering. So the type moves, and `hpdl-model` no longer
  exports it.
- The feature-09 *declaration* types stay in `hpdl-model`: `StatusDimension`, `EvaluatorRef`,
  `BaselineRef`, `CutPoints`, `EvaluationWindow`, `Digest`. `StatusResult`'s unit tests (the
  replay-invariant and sorting tests) move with it into `hpdl-runtime-api`.
- Consumers that referenced `hpdl_model::StatusResult` switch to
  `hpdl_runtime_api::StatusResult`. In this workspace the only references are feature-09's own
  model tests and the `lib.rs` export table; both are updated. (If an external consumer existed
  this would be a breaking `hpdl-model` change — acceptable pre-1.0 and documented.)

## 4. Validation & failure flow

`ProcessContract`/`EvaluatorContract`/`ProcessReceipt` validate on construction (ids, module
tokens, hash shapes) via the existing model/runtime constructors — nothing panics on
untrusted input. The engine uses the contract predicates to raise typed errors:

| Condition | Predicate / check | Error (engine raises; crate provides shape) |
|---|---|---|
| Process effect outside declared writes | `permits_write` / `ReadWriteSet::uncovered_writes` | `Error::UnauthorizedProcessEffect` |
| Process not live at tick | `is_active_at` | `Error::ProcessInactiveAtTick` |
| Unsupported / unregistered module | (registry, feature 14) | `Error::UnsupportedModule` |
| Replay duplicate | `is_replay_duplicate_of` | engine rejects (idempotence) |

The crate does **not** decide which modules are supported — it provides the error shapes and
predicates; the registry is feature 14.

## 5. The required behaviors and where they live

Like feature 10, the "run" behaviors are *execution* behaviors exercised against the real
contracts by a **mock process executor** test double (`tests/mock_process_executor.rs`),
never by HPDL:

| Behavior | Where tested | Kind |
|---|---|---|
| unauthorized process effect | mock (uncovered write rejected) | runtime (mock) |
| inactive-at-tick rejection | mock (`is_active_at`) | runtime (mock) |
| deterministic receipt ordering | mock (sort by `ProcessOrderKey`) | runtime (mock) |
| replay-duplicate detection | mock (`is_replay_duplicate_of`) | runtime (mock) |
| receipt hash consistency | `ProcessReceipt::is_consistent` unit + mock | contract |
| vitality replay invariant | `StatusResult` unit (relocated) | contract |

## 6. Determinism & security

- `ReadWriteSet` collections are already `BTreeSet`-backed; receipt ordering uses the total
  `(tick, process_id)` key; no hashmap iteration into any hash input or output.
- No `f32`/`f64`; checked `Tick` arithmetic; validated ids/hashes; bounded sets. The crate's
  strict lints (`forbid(unsafe_code)`, `deny(missing_docs, unwrap_used, expect_used,
  indexing_slicing, float_arithmetic)`) are preserved — the hex/feed helpers already avoid
  panicking indexing.

## 7. PIR / SemVer impact

- **No PIR change.** Process/vitality contracts and receipts are runtime artifacts
  (state-schema-versioned), not PIR payload — mirrors feature 10 (OD-10.2) and feature 11
  (OD-11.2). PIR golden vectors untouched; `PirVersion::CURRENT` unchanged.
- **`hpdl-runtime-api`** gains additive public types (plus the `StatusResult` relocation);
  pre-1.0, tracked by `cargo semver-checks -p hpdl-runtime-api` in CI.
- **`hpdl-model`** loses the `StatusResult` export (OD-12.4) — a deliberate pre-1.0 change,
  documented in the CHANGELOG and ADR.

## 8. Decisions (resolved — ADR 0006)

- **OD-12.1 → dedicated `ProcessId`** (same grammar as `ActionId`); process runs and action
  events are distinct.
- **OD-12.2 → distinct `ProcessReceipt`** with its own `HPDL-PROCESS-v1` separator; shared
  hashing helpers factored out, no `EventEnvelope` behavior change.
- **OD-12.3 → `ModuleRef`** for process/evaluator identity; `EventEnvelope` keeps its existing
  flat module fields.
- **OD-12.4 → move `StatusResult` to `hpdl-runtime-api`, no `hpdl-model` re-export** (preserve
  dependency direction).
- **OD-12.5 → activation inclusive** (`tick >= activation_tick`).

## 9. What this design deliberately does NOT do

- No executor / solver / scheduler; HPDL does not run processes or evaluators or evolve state
  (the mock is a test double only).
- No module registry / trust roots (feature 14); no migration execution (feature 13).
- No PIR change; no new frontend syntax; no `f32`/`f64`; no panicking paths.
