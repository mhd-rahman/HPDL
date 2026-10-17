# Schema Migration Execution Contracts — Tasks

> Scope: add migration **execution contracts** to `hpdl-runtime-api`. HPDL ships contracts,
> not an engine. No snapshot store, state transformation, signature verification, or quorum
> logic. Decisions OD-13.1…OD-13.5 are resolved in ADR 0007; the three steering UNRESOLVED
> authorization items are surfaced there, not decided in code.

- [x] 1. Add `MigrationId` to `ids.rs`
  - Validated newtype, `ActionId` grammar (1–128, `[A-Za-z0-9_\-.:]`), `Error::InvalidMigrationId`.
  - Unit tests mirror `ProcessId`.
  - **Files:** `crates/hpdl-runtime-api/src/ids.rs`, `error.rs`, `lib.rs`.

- [x] 2. Add migration failure variants to `error.rs`
  - `InvalidMigrationId`, `SchemaDowngrade { from, to }`, `DuplicateMigrationId`,
    `PreCommitInvariantFailure`, `MigrationInactiveAtTick { requested, activation }`,
    `UnauthorizedMigration`. Per-field doc comments (missing_docs is denied).
  - **Files:** `crates/hpdl-runtime-api/src/error.rs`.

- [x] 3. `GovernancePolicyRef` + `MigrationProposal` in `migration.rs`
  - Validated constructors; forward-only enforced (`SchemaDowngrade`); derived `migration_id()`
    under `HPDL-MIGRATION-v1`; inclusive `is_active_at`; accessors; rustdoc + doctests.
  - **Files:** `crates/hpdl-runtime-api/src/migration.rs`.

- [x] 4. `MigrationResult` + `MigrationReceipt` + `MigrationOrderKey`
  - Hash-bound binding via shared `hashing` helpers; `is_consistent()`;
    `is_idempotent_retry_of`; `order_key()`; rustdoc + doctests.
  - **Files:** `crates/hpdl-runtime-api/src/migration.rs`.

- [x] 5. `MigrationExecutor` trait (interface only)
  - `dry_run` + `commit`; documented that the crate ships no impl; mock exercises it.
  - **Files:** `crates/hpdl-runtime-api/src/migration.rs`, `lib.rs` exports.

- [x] 6. Mock migration executor integration test
  - `tests/mock_migration_executor.rs`: downgrade, inactive-at-tick, pre-commit failure
    (prior state authoritative), duplicate-id idempotence, unauthorized, deterministic
    ordering, hash consistency/tamper, cross-domain separation.
  - **Files:** `crates/hpdl-runtime-api/tests/mock_migration_executor.rs`.

- [x] 7. ADR 0007 + CHANGELOG + README index
  - Record OD-13.1…OD-13.5; surface the three UNRESOLVED authorization items. Add CHANGELOG
    section. Update `docs/rfcs/README.md` index.
  - **Files:** `docs/rfcs/0007-schema-migration-execution.md`, `CHANGELOG.md`, `docs/rfcs/README.md`.

- [x] 8. Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D warnings`,
  `cargo test --workspace`, `cargo semver-checks -p hpdl-runtime-api` (where available).

## Out of scope (explicit)
- Migration engine / snapshot store / state transformation / signature verification / quorum
  enforcement / `MigrationAuthorizationPolicy` type (external simulation repo).
- Native `.hpdl` / RON syntax; CLI presentation (feature 15).
- Any PIR format change.
