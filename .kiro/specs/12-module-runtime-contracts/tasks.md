# Module Runtime Contracts (Vitality & Process) — Tasks

> Domain 12 (vitality / process contracts) in `hpdl-runtime-api`, runtime contracts only.
> **Implemented.** Decisions resolved (ADR 0006): OD-12.1 dedicated `ProcessId`; OD-12.2
> distinct `ProcessReceipt` with `HPDL-PROCESS-v1` separator; OD-12.3 `ModuleRef` identity;
> **OD-12.4 moved `StatusResult` into `hpdl-runtime-api`, no `hpdl-model` re-export** (preserves
> dependency direction); OD-12.5 activation inclusive. HPDL ships contracts, not an executor —
> behaviors exercised by a mock test double. No PIR change (verified: `hpdl-pir` golden hashes
> unchanged, 31 tests pass).

## Task 1: `ProcessId` + shared hashing helper
**Status:** completed — `ProcessId` added to `ids.rs` (same grammar as `ActionId`, distinct
type; `Error::InvalidProcessId`). The private `feed` / lowercase-hex helpers were factored out
of `event.rs` into an internal `hashing` module; `EventEnvelope` uses them unchanged (its
binding tests still pass). `hashing` unit tests added.

**Files:** `crates/hpdl-runtime-api/src/{ids.rs,error.rs,event.rs,hashing.rs,lib.rs}`.

---

## Task 2: Relocate `StatusResult` (OD-12.4)
**Status:** completed — `StatusResult` moved verbatim to
`crates/hpdl-runtime-api/src/vitality.rs` (constructor, accessors, sorting + replay-invariant
tests, doctest). Removed from `hpdl-model/src/status_vitality.rs` and the `hpdl-model` export
table; the feature-09 *declaration* types (`StatusDimension`, `EvaluatorRef`, `BaselineRef`,
`CutPoints`, `EvaluationWindow`, `Digest`) stay in `hpdl-model`. Re-exported as
`hpdl_runtime_api::StatusResult`. The model module doc + lib table updated; **not** re-exported
from `hpdl-model` (would invert the dependency direction).

**Files:** `crates/hpdl-runtime-api/src/{vitality.rs,lib.rs}`,
`crates/hpdl-model/src/status_vitality.rs`, `crates/hpdl-model/src/lib.rs`.

---

## Task 3: `ProcessContract` + `EvaluatorContract`
**Status:** completed — `crates/hpdl-runtime-api/src/process.rs`: `ProcessContract` (`ModuleRef`
+ `ReadWriteSet` + activation `Tick`; `permits_write`, `is_active_at` inclusive) and
`EvaluatorContract` (`EvaluatorRef` + `StatusDimension` + `BaselineRef` + `EvaluationWindow`).
Rustdoc + doctests; unit tests for coverage (declared/undeclared/uncovered) and inclusive
activation. The model vocabulary (`ModuleRef`, `EvaluatorRef`, `BaselineRef`,
`EvaluationWindow`, `StatusDimension`) re-exported from `hpdl-runtime-api`.

**Files:** `crates/hpdl-runtime-api/src/{process.rs,lib.rs}`.

---

## Task 4: `ProcessReceipt`
**Status:** completed — `ProcessReceipt` (process id, tick, `ModuleRef`, input/output
`StateHash`, `ReadWriteSet`, binding) with the `HPDL-PROCESS-v1` separator and the shared
hashing helper; `is_consistent()`, `is_replay_duplicate_of()`, `order_key()` →
`ProcessOrderKey`. Unit tests: binding consistency, tampering detection, replay-duplicate,
total ordering, different-tick-different-binding, and a cross-domain test proving a process
binding ≠ an event binding for the same identity surface.

**Files:** `crates/hpdl-runtime-api/src/process.rs`.

---

## Task 5: Failure contracts
**Status:** completed — new typed `Error` variants: `InvalidProcessId`, `UnsupportedModule
{ name, version }`, `ProcessInactiveAtTick { requested, activation }`,
`UnauthorizedProcessEffect`. Documented that the crate supplies error *shapes* + predicates;
the engine raises them and the registry (feature 14) decides "supported".

**Files:** `crates/hpdl-runtime-api/src/error.rs`.

---

## Task 6: Mock process-executor tests
**Status:** completed — `tests/mock_process_executor.rs` is a clearly-labelled **test double**
(not shipped, not a simulation) exercising inactive-at-tick rejection, unauthorized-effect
rejection, deterministic receipt ordering by `ProcessOrderKey`, replay-duplicate idempotence,
and receipt-hash consistency, against the real contracts (6 tests, all pass). The existing
`tests/mock_executor.rs` is unchanged and still passes.

**Files:** `crates/hpdl-runtime-api/tests/mock_process_executor.rs`.

---

## Task 7: Docs + CHANGELOG + ADR
**Status:** completed — rustdoc + examples on all new public items (crate-root doc list
updated); CHANGELOG "Feature 12" entry (new contracts + the `StatusResult` relocation with its
compatibility note); ADR `docs/rfcs/0006-module-runtime-contracts.md` recording OD-12.1…12.5
(esp. the relocation and why no `hpdl-model` re-export). No workflow crate-status change
(`hpdl-runtime-api` already active).

**Files:** `CHANGELOG.md`, `docs/rfcs/0006-module-runtime-contracts.md`, source rustdoc.

---

## Deferred / out of scope (explicit, not silently skipped)

- **Executor / solver / scheduler.** HPDL does not run processes or evaluators or evolve
  state; only the mock test double does.
- **Module registry / trust roots / resolution** (feature 14) — the crate provides error
  shapes + predicates; it does not decide "supported".
- **Migration execution receipts / authorization** (feature 13 / governance) — the receipt
  shape is reusable but migration execution is not in scope here.
- **Native `.hpdl` / RON syntax; CLI presentation** (features 04/05/15).
- **Any PIR format change** — runtime contracts are not PIR payload.

## Definition of Done

- [x] `cargo fmt --all --check` (ran `cargo fmt --all`; `--check` exits 0)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` (clean, exit 0)
- [x] `cargo test --workspace` (all suites pass: model 99 unit + 7 roundtrip + 36 doctests;
      runtime-api 26 unit + 6 event-mock + 6 process-mock + 9 doctests; PIR 31 + golden_hash 3
      unchanged; all other crates unchanged)
- [ ] `cargo deny check` — run in CI (tool not installed in dev env)
- [ ] `cargo semver-checks -p hpdl-pir -p hpdl-runtime-api` — run in CI
- [x] No `unwrap`/`expect`/panic/indexing on input paths; checked arithmetic; no floats.
- [x] Dedicated `HPDL-PROCESS-v1` separator, distinct from PIR and event separators (tested).
- [x] No executor/solver/scheduler/registry; unsupported/inactive/over-write are typed errors.
- [x] `StatusResult` relocated (OD-12.4); dependency direction preserved (no
      `hpdl-model → hpdl-runtime-api` edge).
- [x] No PIR byte change (verified: `hpdl-pir` golden hashes pass unchanged).
- [x] CHANGELOG + ADR updated.
