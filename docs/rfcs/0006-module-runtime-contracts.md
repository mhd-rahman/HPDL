# ADR 0006: Module Runtime Contracts (Vitality & Process)

**Status:** Accepted
**Date:** 2026-10-17
**Author:** HPDL Team
**Feature:** `12-module-runtime-contracts` (OD-12.1 … OD-12.5)

## Summary

Implements domain 12 (vitality / process contracts) in `hpdl-runtime-api` — the
runtime-contract counterpart to feature 09's status/vitality *declarations*. Adds
`ProcessContract`, `EvaluatorContract`, and `ProcessReceipt` (with a dedicated
`HPDL-PROCESS-v1` binding separator and a `ProcessId`), and relocates `StatusResult` from
`hpdl-model` into `hpdl-runtime-api`. HPDL still ships **contracts, not an engine**: no
executor, solver, scheduler, or state evolution — the behaviors are exercised by a mock
executor test double.

## Context

- The architecture/migrations steering assign "state/module/event/migration-execution
  contracts" and "execution interfaces, receipts, failure contracts" to `hpdl-runtime-api`,
  which was activated in feature 10 (ADR 0004).
- Feature 09 defined the status/vitality *declaration* types in `hpdl-model` and placed the
  runtime-facing `StatusResult` there **provisionally**, explicitly noting (OD-09.1, option A)
  that it "moves to `hpdl-runtime-api` when runtime integration begins." It has begun.
- Feature 10 established the pattern this feature reuses: hash-bound records with a dedicated
  domain separator, a total ordering key, replay-duplicate identity, and a **mock executor**
  test double standing in for the external HORIZON engine.

## Decisions

- **OD-12.1 — Dedicated `ProcessId`.** A process-run instance id is a distinct type from
  `ActionId` (same grammar: 1–128 chars, `[A-Za-z0-9_\-.:]`). A process run and an action
  event are different concepts and must not be conflated, even though they share a validation
  rule.

- **OD-12.2 — Distinct `ProcessReceipt` with its own domain separator.** `ProcessReceipt` is
  not a reuse of `EventEnvelope`; it binds with `HPDL-PROCESS-v1`, distinct from the event
  separator (`HPDL-EVENT-v1`) and the PIR separator (`HPDL-PIR-v2`), so a process receipt, an
  event binding, and a PIR content hash can never collide across domains (determinism
  steering). The length-prefixed-feed and lowercase-hex helpers that were private in
  `event.rs` are factored into an internal `hashing` module and shared by both binding types
  with no behavior change to `EventEnvelope` (verified: its existing binding tests still pass).

- **OD-12.3 — `ModuleRef` for process/evaluator identity.** `ProcessContract` and
  `EvaluatorContract` use the model's validated `ModuleRef` (name+version). `EventEnvelope`
  keeps its existing flat `module_name`/`module_version` fields unchanged (no gratuitous churn
  to a shipped contract).

- **OD-12.4 — Move `StatusResult` to `hpdl-runtime-api`; do NOT re-export from `hpdl-model`
  (BLOCKING).** `StatusResult` is a runtime *output* contract; it moves to the runtime crate.
  Re-exporting it from `hpdl-model` would force `hpdl-model` to depend on `hpdl-runtime-api`,
  **inverting** the dependency direction (`runtime-api → model`), which the architecture
  steering forbids. So the type moves and `hpdl-model` drops the export. The feature-09
  *declaration* types (`StatusDimension`, `EvaluatorRef`, `BaselineRef`, `CutPoints`,
  `EvaluationWindow`, `Digest`) stay in `hpdl-model`. `StatusResult`'s own tests (sorting +
  replay invariant) and doctest move with it. In this workspace the only references were
  feature-09's in-model test/doc surface and the `lib.rs` export table; both are updated.

- **OD-12.5 — Activation is inclusive.** `ProcessContract::is_active_at(tick)` is
  `tick >= activation_tick`; the activation tick is the first live tick.

## Alternatives considered

- **Keep `StatusResult` in `hpdl-model` and re-export from runtime-api.** Rejected: it is a
  runtime output, and leaving it in the model crate perpetuates the "provisional" placement
  feature 09 explicitly flagged for removal once runtime-api existed. (The reverse — model
  re-exporting from runtime-api — is forbidden by dependency direction.)
- **Reuse `EventEnvelope` for process runs.** Rejected (OD-12.2): a process run and an action
  event are distinct, and sharing a binding domain would conflate them and risk hash collision
  across concepts.
- **Reuse `ActionId` for process ids.** Rejected (OD-12.1): distinct concepts deserve distinct
  types even when the grammar matches.

## Compatibility impact

- **`hpdl-runtime-api`** gains additive public types (`ProcessId`, `ProcessContract`,
  `EvaluatorContract`, `ProcessReceipt`, `ProcessOrderKey`, `StatusResult`) and re-exports more
  of the model vocabulary. Pre-1.0; tracked by `cargo semver-checks -p hpdl-runtime-api` in CI.
- **`hpdl-model`** loses the `StatusResult` export — a deliberate pre-1.0 API change. The
  dependency direction is preserved (no new `hpdl-model → hpdl-runtime-api` edge). Documented
  in the CHANGELOG.
- **No PIR change.** Process/vitality contracts and receipts are runtime artifacts
  (state-schema-versioned), not PIR payload — mirrors feature 10 (OD-10.2) and feature 11
  (OD-11.2). PIR golden vectors and `PirVersion::CURRENT` are unchanged.

## Test consequences

- Contract unit tests in `hpdl-runtime-api`: `ProcessId` validation; `ProcessContract`
  write-coverage (`permits_write`) and inclusive activation (`is_active_at`); `ProcessReceipt`
  binding consistency, tampering detection, replay-duplicate identity, total ordering, and a
  cross-domain test proving a process binding ≠ an event binding for the same identity surface;
  the relocated `StatusResult` sorting + replay-invariant tests; `hashing` helper tests.
- A `tests/mock_process_executor.rs` **mock** (clearly a test double, not a simulation)
  exercises inactive-at-tick rejection, unauthorized-effect rejection, deterministic receipt
  ordering, replay-duplicate idempotence, and receipt-hash consistency against the real
  contracts.
- The existing `tests/mock_executor.rs` and `EventEnvelope` binding tests still pass unchanged,
  confirming the shared-hashing refactor is behavior-preserving.
- **No golden PIR vector changes.**

## Rationale

This feature does the honest, steering-faithful thing: it gives the runtime its process and
vitality *run contracts* in the crate the architecture assigns them to, finishes the
`StatusResult` relocation feature 09 explicitly deferred, and reuses feature 10's hashing /
ordering / mock-executor discipline — all while refusing to add an executor, a solver, a
scheduler, a registry, or any PIR change. Execution stays external and is only *mocked* in
tests, clearly labelled as such.
