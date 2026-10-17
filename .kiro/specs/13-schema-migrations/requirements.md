# Schema Migration Execution Contracts — Requirements

## Overview

This feature covers **domain-13 work: schema migration execution contracts**. It is the
*runtime-contract* counterpart to the migration *declaration* that already lives in
`hpdl-model` (`MigrationDeclaration`): the stable types the external HORIZON engine uses to
**run** an authorized, atomic, forward-only schema migration — a versioned
**`MigrationProposal`**, a deterministic hash-bound **`MigrationReceipt`**, a
**`MigrationExecutor`** contract (interface, not an engine), and the typed **failure
contracts** a pre-commit/authorization failure produces.

These contracts live in **`hpdl-runtime-api`** (the migrations steering explicitly assigns
"Execution interfaces, receipts, failure contracts" to this crate). This feature builds on
feature 12's receipt discipline (dedicated domain separator, length-prefixed binding,
`is_consistent()`, total ordering key, replay-duplicate identity, mock-executor test double).

Primary crate: **`hpdl-runtime-api`**. Supporting change: `hpdl-model` (an optional additive
`MigrationId` grammar check is kept in runtime-api to avoid churn; the declaration type is
unchanged).

> **Critical framing (migrations / architecture / product steering):** HPDL defines
> *contracts*; a separate authoritative simulation engine executes them, enforces
> authorization, takes snapshots, transforms state, and replays. "A declaration never
> executes or simulates anything." This feature does **not** add a migration engine, a
> snapshot store, a transformation function, signature verification, or quorum enforcement.
> It defines the executable *interfaces, receipts, and failure contracts* and exercises them
> with a **mock executor test double**, exactly as features 10 and 12 did. A migration that is
> unauthorized, a downgrade, a duplicate, or whose pre-commit invariants fail yields an
> explicit typed error — never a silent success.

## Goals

1. **Migration proposal contract**: a stable `MigrationProposal` naming everything the
   migrations-steering protocol step 1 requires — the world/planet id, the source and target
   *definition* and *schema* hashes, the migration implementation module (`ModuleRef`) and its
   digest, the activation `Tick`, a nonce, and the governance-policy reference. Forward-only
   (`to_schema > from_schema`) is enforced at construction.
2. **Deterministic migration receipt**: a `MigrationReceipt` — the hash-bound record of one
   committed migration at one tick: migration id, tick, old/new state hashes, old/new
   definition hashes, implementation digest, policy reference, result, and a binding hash under
   a dedicated `HPDL-MIGRATION-v1` domain separator (distinct from PIR, event, and process
   separators). Same inputs ⇒ identical receipt bytes (deterministic replay).
3. **Executor contract (interface only)**: a `MigrationExecutor` trait describing the
   dry-run → validate-invariants → atomic-commit-at-tick-boundary protocol the engine
   implements. The crate ships the *trait* and the receipt/error shapes; it ships **no**
   implementation. The mock test double implements the trait to exercise the behaviors.
4. **Explicit failure contracts**: typed `Error` variants for a schema downgrade, a duplicate
   migration id (idempotent retry), a pre-commit invariant failure (prior state stays
   authoritative), a policy-incompatible/invalid/missing approval, and an inactive-at-tick
   migration. Each is explicit — **never** silent history rewrite or silent success.
5. **Forward-fix-only / idempotence**: a committed receipt is immutable; a duplicate migration
   id is an idempotent retry (same id + same old/new hashes ⇒ same receipt). A
   successful-but-wrong migration is corrected only by a NEW forward migration — the contract
   provides no "undo".
6. **One home for runtime contracts**: everything executable-contract-shaped lives in
   `hpdl-runtime-api`; the crate still ships **no engine**.

## Scope boundaries

- **HPDL does not execute migrations, take snapshots, transform state, verify signatures, or
  enforce quorum.** No snapshot store, no state transformation, no cryptographic
  signature verification, no validator-set attestation logic. The `MigrationExecutor` trait
  and `MigrationReceipt` are contracts; the external engine and validator network implement
  and enforce them. The crate's tests use a **mock** executor double.
- **Authorization policy is referenced, not enforced.** A `GovernancePolicyRef` names a
  versioned `MigrationAuthorizationPolicy` by id + policy hash; the policy's operator keys,
  validator set, threshold, signature suite, rotation and revocation rules, and the actual
  signature checking are the external simulation repo's job (migrations steering ownership
  table). This feature provides the *reference* and the *approval-failure error shapes*.
- **No PIR change.** Migration receipts/proposals are runtime artifacts (state-schema-versioned
  surface), not PIR payload — mirrors features 10/11/12.
- **No new frontend syntax.** The `MigrationDeclaration` already in `hpdl-model` is unchanged;
  no `.hpdl`/RON surface is added here (features 04/05).

## Functional Requirements

### FR1: Migration id (`MigrationId`)

**FR1.1**: Introduce `MigrationId` in `hpdl-runtime-api`: a validated identifier, same grammar
as `ActionId`/`ProcessId` (1–128 chars, `[A-Za-z0-9_\-.:]`), distinct type so a migration is
never conflated with an action event or a process run.

### FR2: Migration proposal (`MigrationProposal`)

**FR2.1**: Introduce `MigrationProposal` binding, per migrations-steering protocol step 1: a
`BodyId` (the world/planet), `from_definition`/`to_definition` `Digest`s,
`from_schema`/`to_schema` `u32` schema versions, the implementation `ModuleRef` and its
implementation `Digest`, an activation `Tick`, a `u64` nonce, and a `GovernancePolicyRef`.

**FR2.2**: Construction enforces forward-only (`to_schema > from_schema`) and that all ids,
digests and module tokens are well-formed. A downgrade/equal-schema proposal is
`Error::SchemaDowngrade`. Bounds enforced; no panics on untrusted input.

**FR2.3**: `MigrationProposal` exposes a stable `migration_id()` derived deterministically from
its binding fields (so the same proposal always has the same id and retries are idempotent),
plus accessors for every field.

### FR3: Governance policy reference (`GovernancePolicyRef`)

**FR3.1**: Introduce `GovernancePolicyRef`: a versioned policy id (same grammar as a module
name) and a policy `Digest` (`policy_hash`). It is a *reference*, not the policy; it carries no
keys, thresholds, or signatures. Validated at construction.

### FR4: Deterministic migration receipt (`MigrationReceipt`)

**FR4.1**: Introduce `MigrationReceipt`: migration id, tick, old/new `StateHash`, old/new
definition `Digest`, implementation `Digest`, `GovernancePolicyRef`, a `MigrationResult`
(`Committed` / `RejectedPreCommit`), and a binding hash over all other fields under
`HPDL-MIGRATION-v1`. Reuses the shared `hashing` helpers; no floats; no panics; pure
`is_consistent()`.

**FR4.2 (deterministic ordering)**: `MigrationReceipt` orders by a documented total key
`(tick, migration_id)` — never hashmap iteration or wall-clock.

**FR4.3 (idempotent retry identity)**: Two receipts with the same `(migration_id, old_state,
new_definition)` are the same migration (idempotent retry); the engine uses this to reject /
dedupe duplicates (`is_idempotent_retry_of`), the migration analogue of
`ProcessReceipt::is_replay_duplicate_of`.

**FR4.4 (forward-fix only)**: The receipt is immutable and carries no reverse transition. A
`RejectedPreCommit` result records that the prior state stayed authoritative; a wrong-but-
committed migration is corrected only by a new forward `MigrationProposal`.

### FR5: Executor contract (`MigrationExecutor`)

**FR5.1**: Introduce a `MigrationExecutor` trait: `dry_run(&proposal, snapshot_state) ->
Result<(), Error>` (validate invariants against a snapshot hash) and
`commit(&proposal, old_state) -> Result<MigrationReceipt, Error>` (atomic commit at the
activation tick). The crate provides the trait + error shapes only; it ships no implementation.
Documented as the interface the external engine implements and the mock double exercises.

### FR6: Explicit failure contracts

**FR6.1**: New typed `Error` variants: `InvalidMigrationId`, `SchemaDowngrade { from, to }`,
`DuplicateMigrationId(String)`, `PreCommitInvariantFailure(String)`, `MigrationInactiveAtTick
{ requested, activation }`, and `UnauthorizedMigration(String)` (policy-incompatible / missing /
invalid / expired / duplicate approval — the engine supplies the reason). Each explicit and
typed — never a silent no-op or history rewrite.

**FR6.2**: The crate does not *decide* approval validity (that is the simulation / validator
network's job); it provides the error *shapes* and the predicates (forward-only,
inactive-at-tick, idempotent-retry identity) the engine uses to raise them.

### FR7: Determinism & security

**FR7.1**: No `f32`/`f64`; `u32` schema versions and `u64` nonce; ordered fields; a documented
total ordering key; no wall-clock/locale/hashmap iteration in any contract bytes or hash input.

**FR7.2**: Validate all ids/digests/module tokens before use; checked arithmetic on ticks and
schema versions; no `unwrap`/`expect`/panic/indexing on input paths.

## Non-Functional Requirements

### NFR1 — Determinism: receipts are integer/hash-bound and byte-reproducible; the binding uses
a dedicated domain separator (`HPDL-MIGRATION-v1`), never the PIR (`HPDL-PIR-v2`), event
(`HPDL-EVENT-v1`), or process (`HPDL-PROCESS-v1`) separators, so cross-domain hashes can never
collide.

### NFR2 — Security / untrusted input: fail closed with typed errors; bound string fields; no
panicking paths; no float arithmetic (preserve the crate's strict lints).

### NFR3 — Reliability / contract stability: `hpdl-runtime-api` stays a published,
semver-stable contract crate with minimal deps (`hpdl-model`, `hpdl-pir`, `serde`, `sha2`,
`thiserror`); `forbid(unsafe_code)`; no dependency on parser/compiler/sema.

### NFR4 — Documentation: rustdoc + examples on every new public item; CHANGELOG updated; an
ADR records the resolved decisions and surfaces the three steering-flagged UNRESOLVED
authorization items (do not decide them in code).

### NFR5 — Tests: contract unit tests + a **mock migration executor** test double exercising:
forward-only rejection (downgrade), duplicate-id idempotence, pre-commit invariant failure
(prior state authoritative), inactive-at-tick rejection, unauthorized/invalid-approval
rejection, deterministic receipt ordering, and receipt-hash consistency — against the real
contracts, never a shipped engine.

## Constraints

- `hpdl-runtime-api → hpdl-pir (+ selected stable hpdl-model types)` only; no reverse edges; no
  dependency on `hpdl-compiler`/`hpdl-sema`/`hpdl-ron`/`hpdl-syntax`.
- Reuse `ModuleRef`, `BodyId`, `Digest` from `hpdl-model` and `StateHash`, `Tick` from
  `hpdl-runtime-api` — do not introduce parallels.
- No migration engine, snapshot store, state transformation, signature verification, quorum
  enforcement, new frontend syntax, or PIR change.

## Dependency & versioning implications (surfaced up front)

- **SemVer.** `hpdl-runtime-api` is a pre-1.0 published contract crate; the additions are
  additive and `cargo semver-checks -p hpdl-runtime-api` applies.
- **No PIR change.** Receipts/proposals are runtime artifacts, not PIR payload; no PIR
  format-version impact. `PirVersion::CURRENT` and golden vectors unchanged.
- **`hpdl-model` unchanged** for this feature: `MigrationDeclaration` stays as-is. The
  execution-side `MigrationProposal` lives in runtime-api so no new `model → runtime-api` edge
  is introduced and the declaration/execution split in the migrations steering is preserved.

## Open decisions / approvals required — resolved below (ADR 0007); authorization items
remain UNRESOLVED by design

- **OD-13.1 — Where does `MigrationProposal` live? — RESOLVED: `hpdl-runtime-api`.** It is an
  *execution* contract (migrations-steering ownership). The *declaration* (`MigrationDeclaration`)
  stays in `hpdl-model`. This preserves the dependency direction (no `model → runtime-api`).
- **OD-13.2 — Dedicated `MigrationId`? — RESOLVED: yes**, distinct type, `ActionId` grammar
  (OD-12.1 precedent). A migration, an action, and a process run are distinct concepts.
- **OD-13.3 — Receipt binding separator — RESOLVED: new `HPDL-MIGRATION-v1`**, distinct from
  the three existing separators; shared `hashing` helpers reused with no behavior change.
- **OD-13.4 — Is `MigrationProposal::migration_id` derived or supplied? — RESOLVED: derived**
  deterministically from the binding fields, so retries are idempotent by construction and a
  supplied-id/binding mismatch is impossible.
- **OD-13.5 — Authorization enforcement — RESOLVED (scope): referenced, not enforced.** The
  crate provides `GovernancePolicyRef` + `UnauthorizedMigration` error shape; the external repo
  enforces. The three steering UNRESOLVED items (approval-expiry basis; revoked-key handling;
  emergency/quorum-unreachable policy) are **surfaced in ADR 0007 and NOT decided in code.**

## Success Criteria

1. ✅ `MigrationId`, `GovernancePolicyRef`, `MigrationProposal`, `MigrationReceipt`,
   `MigrationResult`, `MigrationOrderKey`, and a `MigrationExecutor` trait exist in
   `hpdl-runtime-api` with validated constructors, typed errors, and rustdoc.
2. ✅ `MigrationProposal` enforces forward-only and derives a deterministic `migration_id`;
   `MigrationReceipt` has a dedicated-separator binding + `is_consistent()` + a total ordering
   key + idempotent-retry identity.
3. ✅ Downgrade / duplicate-id / pre-commit-failure / inactive-at-tick / unauthorized cases are
   explicit typed errors; nothing silently succeeds, no-ops, or rewrites history; no engine is
   shipped.
4. ✅ No floats; no panics on malformed input; checked arithmetic; ordered fields; dedicated
   hash domain separator distinct from PIR, event, and process separators.
5. ✅ A mock-migration-executor test double exercises the required behaviors against the real
   contracts; contract unit tests cover ids/forward-only/receipt consistency/ordering/idempotence.
6. ✅ No PIR byte change; dependency direction preserved; `hpdl-runtime-api` stays minimal and
   semver-stable.
7. ✅ CI passes; CHANGELOG + ADR 0007 updated; the three unresolved authorization items are
   surfaced, not silently resolved.

## Out of Scope

- Executing migrations, taking/validating snapshots, transforming state, verifying signatures,
  enforcing quorum, or replaying at run time (external HORIZON engine + validator network).
- The authoritative `MigrationAuthorizationPolicy` type, key rotation/revocation logic, and
  signature suites (external simulation repo; this feature references the policy only).
- Native `.hpdl` / RON syntax; CLI presentation (feature 15).
- Any PIR format change.

## References

- Steering: `migrations.md` (ownership table — execution interfaces/receipts/failure contracts
  owned by `hpdl-runtime-api`; protocol; forward-fix-only; authorization; the three UNRESOLVED
  items), `architecture.md` (contracts vs implementations; dependency direction),
  `product.md` (definitions and runtime state separately versioned; history preserved),
  `determinism.md` (domain-separated hashing; no floats).
- Builds on: feature 12 (ADR 0006 — `ProcessReceipt`, dedicated separator, `hashing` module,
  total ordering, replay-duplicate identity, mock-executor pattern) and the existing
  `MigrationDeclaration` in `hpdl-model`.
- Design: `docs/design/13-schema-migrations.md`.
