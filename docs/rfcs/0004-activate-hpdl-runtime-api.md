# ADR 0004: Activate `hpdl-runtime-api`

**Status:** Accepted
**Date:** 2026-10-16
**Author:** HPDL Team
**Feature:** `10-actions-and-events` (OD-10.1, option B)

## Summary

`hpdl-runtime-api` was previously **deferred** ("deferred until runtime integration" in the
workflow steering crate-status table; commented out in the workspace `Cargo.toml`). Feature 10
(actions and events) is substantially runtime-facing: its core contract types — `EventEnvelope`,
`ActionId`, `Tick`, `StateHash`, `ReadWriteSet` — are exactly the "state/module/event/
migration-execution contracts" the architecture steering assigns to `hpdl-runtime-api`. This
ADR records the decision to **activate** that crate now and begin runtime-contract integration.

## Context

- The design doc for feature 10 names `hpdl-runtime-api` as a primary crate.
- The workflow steering says: "Do NOT scaffold crates marked deferred unless the task
  explicitly asks." This task explicitly asks.
- Feature 09 placed one runtime-facing type (`StatusResult`) provisionally in `hpdl-model`
  under a "pre-runtime-api" label precisely because the crate was deferred. Activating the
  crate gives these contracts their correct home.

## Decision

1. **Create the `hpdl-runtime-api` crate** as a published contract crate:
   - Dependencies limited to `hpdl-pir` (for the `CanonicalHash` hashing discipline) and
     `hpdl-model` (selected stable types), plus `serde` and `thiserror`. It must **not** depend
     on `hpdl-compiler`, `hpdl-sema`, `hpdl-ron`, or `hpdl-syntax` (no reverse edges).
   - Strict lints: `#![forbid(unsafe_code)]`, `#![deny(missing_docs, clippy::unwrap_used,
     clippy::expect_used, clippy::indexing_slicing, clippy::float_arithmetic)]`.
   - Semver-stable surface; subject to `cargo semver-checks -p hpdl-runtime-api` going forward.

2. **Implement the event/replay contract types** there: `Tick` (monotonic `u64`), `StateHash`
   (SHA-256 + a state-specific domain separator, reusing the PIR hashing approach), `ActionId`
   (validated id), `ReadWriteSet` (deterministic ordered read/write field paths), and
   `EventEnvelope` (binds `action_id`, `tick`, acting module+version, input/output `StateHash`,
   and the read/write set; provides a pure hash/consistency check).

3. **Do NOT implement an executor.** HPDL still does not evolve state. The required execution
   behaviors (deterministic ordering, replay-duplicate detection, event-hash consistency,
   capability authorization) are exercised in tests by a **mock executor** that is clearly
   labelled as a test double standing in for the external HORIZON engine. The crate ships the
   contracts and pure verification helpers only.

4. **Update project metadata:** uncomment the `crates/hpdl-runtime-api` workspace member and
   change the workflow steering status from `deferred` to `active (feature 10)`.

## Alternatives considered

- **(A) Keep deferred; definition-side only** (as feature 09 did for `StatusResult`).
  Rejected for this feature because the *majority* of feature 10's new types are runtime
  contracts; parking them all in `hpdl-model` would overload the model crate and misplace the
  published runtime surface. Keeping the crate deferred would also leave the design's required
  behaviors (ordering/replay/authorization) with nowhere coherent to live.

## Compatibility impact

- **New crate, additive.** No existing crate's public API changes. No PIR format change: the
  event/replay contracts are *runtime* artifacts (independently versioned state-schema
  surface), not PIR payload. OD-10.2 is therefore N/A unless capabilities are later encoded
  into PIR.
- `hpdl-runtime-api` starts at `0.1.0`. As a pre-1.0 contract crate its surface may still
  evolve, but changes are tracked and semver-checked.

## Test consequences

- Contract-level unit tests in the crate: id/hash/tick validity, `EventEnvelope` hash
  consistency, deterministic ordering key, replay-duplicate identity.
- A `tests/mock_executor.rs` integration test implements a **mock** in-memory executor/replay
  harness (explicitly a test double, not a simulation) to exercise: deterministic ordering,
  replay-duplicate rejection, event-hash consistency, and unauthorized-action rejection against
  the declared capabilities.
- No golden PIR vector changes.

## Rationale

Activating the crate now is the honest, faithful-to-the-design choice for a feature whose
substance is runtime contracts. It respects the steering's gate (the task explicitly asks),
keeps the published surface minimal and semver-stable, and — critically — does **not** cross
the line into claiming HPDL executes state: execution remains external and is only *mocked* in
tests, clearly labelled as such.
