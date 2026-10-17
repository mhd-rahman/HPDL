# Schema Migration Execution Contracts — Design

## 1. Scope

Add the migration **execution contracts** to `hpdl-runtime-api`, the crate the migrations
steering assigns "execution interfaces, receipts, failure contracts". HPDL ships contracts, not
an engine: no snapshot store, no state transformation, no signature verification, no quorum
logic. The behaviors are exercised by a **mock executor test double**, mirroring features 10
and 12.

## 2. New module layout (`crates/hpdl-runtime-api/src/`)

```
migration.rs   -- GovernancePolicyRef, MigrationProposal, MigrationResult,
                  MigrationReceipt, MigrationOrderKey, MigrationExecutor
ids.rs         -- add MigrationId (grammar identical to ActionId/ProcessId)
error.rs       -- add migration failure variants
lib.rs         -- `mod migration;` + `pub use migration::{...}` and `pub use ids::MigrationId`
hashing.rs     -- unchanged (reused)
```

`MigrationProposal` lives in runtime-api (OD-13.1), not `hpdl-model`, so no `model →
runtime-api` edge is introduced. The declaration-side `MigrationDeclaration` in `hpdl-model`
stays unchanged.

## 3. Types

### 3.1 `MigrationId` (ids.rs)

Same shape and grammar as `ProcessId` (OD-13.2): 1–128 chars `[A-Za-z0-9_\-.:]`, distinct type,
`Error::InvalidMigrationId`.

### 3.2 `GovernancePolicyRef` (migration.rs)

```rust
pub struct GovernancePolicyRef { policy_id: String, policy_hash: Digest }
```
`policy_id` validated like a module name (non-empty, 1..=128). `policy_hash` is a `Digest`
(64-hex). A *reference* only — no keys, thresholds, signatures.

### 3.3 `MigrationProposal` (migration.rs)

```rust
pub struct MigrationProposal {
    body: BodyId,
    from_definition: Digest, to_definition: Digest,
    from_schema: u32, to_schema: u32,
    implementation: ModuleRef, implementation_digest: Digest,
    activation_tick: Tick,
    nonce: u64,
    policy: GovernancePolicyRef,
}
```
Constructor `new(...) -> Result<Self, Error>`:
- forward-only: `to_schema > from_schema`, else `Error::SchemaDowngrade { from, to }`.
- all newtype fields already validated by their own constructors.

`migration_id()` is **derived** (OD-13.4): SHA-256 over the binding fields under
`HPDL-MIGRATION-v1`, encoded as an `m-` prefix + lowercase hex into a valid `MigrationId`. Same
proposal ⇒ same id ⇒ idempotent retries by construction. `is_active_at(tick)` is inclusive
(`tick >= activation_tick`), matching the process contract.

### 3.4 `MigrationResult` (migration.rs)

```rust
pub enum MigrationResult { Committed, RejectedPreCommit }
```
A committed migration is immutable (forward-fix only). `RejectedPreCommit` records that the
prior state stayed authoritative.

### 3.5 `MigrationReceipt` (migration.rs)

```rust
pub struct MigrationReceipt {
    migration_id: MigrationId,
    tick: Tick,
    old_state: StateHash, new_state: StateHash,
    from_definition: Digest, to_definition: Digest,
    implementation_digest: Digest,
    policy: GovernancePolicyRef,
    result: MigrationResult,
    binding: StateHash,   // SHA-256 hex under HPDL-MIGRATION-v1
}
```
Methods mirror `ProcessReceipt`:
- `new(...)` computes the binding via the shared `feed`/`to_lower_hex` helpers under
  `const MIGRATION_DOMAIN_SEPARATOR: &[u8] = b"HPDL-MIGRATION-v1";`.
- `order_key() -> MigrationOrderKey` (`(tick, migration_id)`).
- `is_consistent()` — recompute + compare; pure, no panic.
- `is_idempotent_retry_of(other)` — same `migration_id && old_state && to_definition`
  (FR4.3). The engine dedupes with this.

Binding feed order (fixed, documented): separator, migration_id, tick (BE u64), old_state,
new_state, from_definition, to_definition, implementation_digest, policy_id, policy_hash,
result discriminant byte.

### 3.6 `MigrationExecutor` trait (migration.rs)

```rust
pub trait MigrationExecutor {
    fn dry_run(&self, proposal: &MigrationProposal, snapshot_state: &StateHash)
        -> Result<(), Error>;
    fn commit(&self, proposal: &MigrationProposal, old_state: &StateHash)
        -> Result<MigrationReceipt, Error>;
}
```
Interface only; **no impl shipped**. The mock test double implements it to exercise the
protocol (dry-run → validate → atomic commit at tick boundary → hash-bound receipt).

## 4. Error additions (error.rs)

| Case | Predicate / where | Error |
|---|---|---|
| Malformed migration id | `MigrationId::new` | `InvalidMigrationId(String)` |
| Schema downgrade / equal | `MigrationProposal::new` | `SchemaDowngrade { from, to }` |
| Duplicate migration id (retry) | engine, via `is_idempotent_retry_of` | `DuplicateMigrationId(String)` |
| Pre-commit invariant failure | engine `dry_run`/`commit` | `PreCommitInvariantFailure(String)` |
| Migration not live at tick | `is_active_at` | `MigrationInactiveAtTick { requested, activation }` |
| Invalid/missing/expired/duplicate/policy-incompatible approval | engine + validator net | `UnauthorizedMigration(String)` |

The crate raises `InvalidMigrationId` and `SchemaDowngrade` itself (construction); the rest are
*shapes* the external engine/validator network raises. Nothing silently rewrites history.

## 5. Determinism, security, and the forbidden list

- Dedicated `HPDL-MIGRATION-v1` separator — never PIR/event/process separators (NFR1).
- No `f32`/`f64`; `u32` schema versions, `u64` nonce/tick; checked arithmetic.
- Ordered, length-prefixed binding feed; no hashmap iteration; no wall-clock.
- No `unwrap`/`expect`/panic/indexing on input; bound string fields.
- **Not** added: migration engine, snapshot store, state transformation, signature
  verification, quorum/threshold logic, the authoritative `MigrationAuthorizationPolicy` type,
  any PIR change, any frontend syntax.

## 6. Mock executor test double (`tests/mock_migration_executor.rs`)

Loudly labelled a test double, not a simulation, not shipped. `MockMigrationEngine` keeps a
`log: Vec<MigrationReceipt>` and a `seen: BTreeSet<(String, String, String)>` (id, old_state,
to_definition). `apply(...)`:
1. `is_active_at` → `MigrationInactiveAtTick`.
2. a configurable invariant check → `PreCommitInvariantFailure` → receipt result
   `RejectedPreCommit`, prior state authoritative, not added to committed log.
3. idempotent-retry via `is_idempotent_retry_of` → return the existing receipt.
4. a configurable approval check → `UnauthorizedMigration`.
5. record a hash-bound `Committed` receipt; keep the log sorted by `order_key`.

Tests: downgrade rejected at construction; inactive-at-tick rejected; pre-commit failure leaves
prior state authoritative; duplicate id is idempotent; unauthorized rejected; deterministic
ordering; receipt-hash consistency + tamper detection; cross-domain binding ≠ a process binding
for the same surface.

## 7. Compatibility & tests summary

- Additive to `hpdl-runtime-api` (pre-1.0, `cargo semver-checks`); no PIR change; dependency
  direction preserved; `hpdl-model` untouched.
- Unit tests in `migration.rs` + `ids.rs`; integration `tests/mock_migration_executor.rs`;
  existing event/process tests still pass unchanged.
- ADR 0007 records OD-13.1…OD-13.5 and surfaces the three steering UNRESOLVED authorization
  items (approval-expiry basis; revoked-key handling; emergency/quorum-unreachable) without
  deciding them in code.
