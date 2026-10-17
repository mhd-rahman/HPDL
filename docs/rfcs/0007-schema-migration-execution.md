# 0007 — Schema Migration Execution Contracts

**Status:** Accepted
**Date:** 2026-10-17
**Feature:** `13-schema-migrations` (OD-13.1 … OD-13.5)

## Context

The migrations/governance steering assigns "Execution interfaces, receipts, failure contracts"
to `hpdl-runtime-api`, while "Declaration (versions, prerequisites, invariants)" stays in
`hpdl-model` (where `MigrationDeclaration` already lives) and "Snapshots, authorization
enforcement, transformation, replay" live in the external simulation repo.

Features 10 and 12 established the pattern this feature reuses: hash-bound records under a
dedicated domain separator, length-prefixed binding feeds via a shared `hashing` module, a
total ordering key, a replay/retry identity predicate, and a **mock executor** test double
standing in for the external HORIZON engine. HPDL ships contracts, not an engine.

## Decision

Add the migration *execution* contracts to `hpdl-runtime-api`: `MigrationId`,
`GovernancePolicyRef`, `MigrationProposal`, `MigrationResult`, `MigrationReceipt`,
`MigrationOrderKey`, and a `MigrationExecutor` trait, plus typed failure variants. No engine,
snapshot store, state transformation, signature verification, or quorum logic is shipped;
behaviors are exercised by `tests/mock_migration_executor.rs`.

- **OD-13.1 — `MigrationProposal` lives in `hpdl-runtime-api`.** It is an *execution* contract.
  Placing it in `hpdl-model` would either duplicate declaration vocabulary or (if the receipt
  types moved to the model) invert the dependency direction. The declaration
  (`MigrationDeclaration`) stays in `hpdl-model`; the proposal/receipt/executor live in
  runtime-api. No new `model → runtime-api` edge.
- **OD-13.2 — Dedicated `MigrationId`.** Distinct type, same grammar as `ActionId`/`ProcessId`
  (1–128 chars, `[A-Za-z0-9_\-.:]`). A migration, an action event, and a process run are
  distinct concepts (OD-10.1 / OD-12.1 precedent).
- **OD-13.3 — New `HPDL-MIGRATION-v1` binding separator.** Distinct from `HPDL-PIR-v2`,
  `HPDL-EVENT-v1`, and `HPDL-PROCESS-v1`, so a migration binding can never collide with a PIR
  content hash, an event binding, or a process receipt. Shared `feed`/`to_lower_hex` helpers
  reused with no behavior change to the existing bindings.
- **OD-13.4 — `migration_id` is derived, not supplied.** It is SHA-256 over the proposal's
  binding fields under `HPDL-MIGRATION-v1`, encoded `m-` + lowercase hex. Because the id is a
  pure function of the proposal, retries are idempotent by construction and an id/binding
  mismatch is impossible.
- **OD-13.5 — Authorization is referenced, not enforced.** `GovernancePolicyRef` names a
  versioned policy by id + hash; the crate provides the reference and the
  `UnauthorizedMigration` error *shape*. The external simulation repo / validator network owns
  the `MigrationAuthorizationPolicy`, operator/validator keys, thresholds, signature suites,
  and rotation/revocation rules, and performs the actual checks.

## Rationale

This gives the runtime its migration *execution contracts* in the crate the steering assigns
them to, reuses the feature-10/12 hashing/ordering/mock discipline, and refuses to add an
engine, snapshot store, transformation, signature verification, or quorum logic. A downgrade,
duplicate id, pre-commit failure, inactive-at-tick, or unauthorized migration is an explicit
typed error — never a silent success or history rewrite (forward-fix only).

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| Put `MigrationProposal` in `hpdl-model` | It is an execution contract; the declaration already exists in the model, and moving receipts there would invert the dependency direction. |
| Reuse `ProcessId`/`ActionId` for the migration id | Distinct concepts deserve distinct types even with identical grammar (OD-12.1 precedent). |
| Reuse the process/event separator for the receipt binding | Would allow cross-domain hash collision between a migration, a process run, and an event. |
| Supplied `migration_id` | Allows id/binding mismatch and non-idempotent retries; a derived id eliminates both. |
| Model the full `MigrationAuthorizationPolicy` here | Enforcement, keys, and quorum belong to the external simulation repo (steering ownership). |

## Compatibility impact

- **`hpdl-runtime-api`** gains additive public items (`MigrationId`, `GovernancePolicyRef`,
  `MigrationProposal`, `MigrationResult`, `MigrationReceipt`, `MigrationOrderKey`,
  `MigrationExecutor`) and re-exports `BodyId`/`Digest` from the model. Pre-1.0; tracked by
  `cargo semver-checks -p hpdl-runtime-api`.
- **`hpdl-model`** is unchanged; the dependency direction is preserved.
- **No PIR change.** Migration proposals/receipts are runtime artifacts, not PIR payload —
  mirrors features 10/11/12. `PirVersion::CURRENT` and golden vectors unchanged.

## Test consequences

- Unit tests in `migration.rs` (policy-ref validation, forward-only, inclusive activation,
  deterministic/derived id, receipt consistency + tamper detection, committed-vs-rejected
  binding difference, idempotent-retry identity, ordering key) and `ids.rs` (`MigrationId`).
- `tests/mock_migration_executor.rs` exercises downgrade rejection, inactive-at-tick,
  pre-commit failure (prior state authoritative), duplicate-id idempotence, unauthorized
  rejection, deterministic ordering, and tamper detection through the real contracts.
- Existing event/process bindings and golden PIR vectors are unchanged.

## Unresolved — surfaced, NOT decided in code

The migrations steering flags three governance questions that this contract layer deliberately
does **not** resolve; they require their own ADR when the external authorization layer is
designed:

- **Approval-expiry basis** — must be simulation ticks (wall-clock breaks replay) or the
  validity result must be recorded in the signed event.
- **Revoked-key handling** for pending and in-flight approvals.
- **Emergency policy** when a validator quorum is unreachable (e.g. world pause).

`hpdl-runtime-api` provides only the `GovernancePolicyRef` and the `UnauthorizedMigration`
error shape; the resolution of these items lives with the external simulation repo and a future
ADR.
