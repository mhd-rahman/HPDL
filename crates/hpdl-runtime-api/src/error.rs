//! Typed errors for the runtime contract types.

use thiserror::Error;

/// Errors produced by `hpdl-runtime-api` constructors and verifiers.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// An action identifier was empty, too long (> 128 chars), or contained
    /// characters outside `[A-Za-z0-9_\-.:]`.
    #[error("invalid action id {0:?}: must be 1–128 chars, alphanumeric, '-', '_', '.', or ':'")]
    InvalidActionId(String),

    /// A state-hash hex string was not exactly 64 lowercase hex characters.
    #[error("invalid state hash {0:?}: must be exactly 64 lowercase hex characters (SHA-256)")]
    InvalidStateHash(String),

    /// A process identifier was empty, too long (> 128 chars), or contained
    /// characters outside `[A-Za-z0-9_\-.:]`.
    #[error("invalid process id {0:?}: must be 1–128 chars, alphanumeric, '-', '_', '.', or ':'")]
    InvalidProcessId(String),

    /// A referenced process or evaluator module has no registered, supported
    /// implementation.
    ///
    /// The crate provides this error *shape*; which modules are supported is a
    /// registry concern (feature 14). Raised by the engine, never a silent no-op.
    #[error("unsupported module {name:?}@{version:?}: no registered implementation")]
    UnsupportedModule {
        /// The module name.
        name: String,
        /// The module version.
        version: String,
    },

    /// A process was asked to run at a tick before its activation tick.
    #[error("process inactive at tick {requested}: activation tick is {activation}")]
    ProcessInactiveAtTick {
        /// The tick the engine attempted to run the process at.
        requested: u64,
        /// The process's declared activation tick.
        activation: u64,
    },

    /// A process effect targeted a write path not covered by its declared
    /// read/write set — an unauthorized effect at the contract level.
    #[error(
        "unauthorized process effect: write {0:?} is not covered by the declared read/write set"
    )]
    UnauthorizedProcessEffect(String),

    /// A migration identifier was empty, too long (> 128 chars), or contained
    /// characters outside `[A-Za-z0-9_\-.:]`.
    #[error(
        "invalid migration id {0:?}: must be 1–128 chars, alphanumeric, '-', '_', '.', or ':'"
    )]
    InvalidMigrationId(String),

    /// A migration proposal was not forward-only: the target schema version was
    /// not strictly greater than the source schema version.
    #[error(
        "schema downgrade rejected: to_schema ({to}) must be greater than from_schema ({from})"
    )]
    SchemaDowngrade {
        /// The source schema version.
        from: u32,
        /// The target schema version.
        to: u32,
    },

    /// A migration with this id was already committed with the same old/new
    /// identity — a duplicate (non-idempotent) retry the engine rejects.
    ///
    /// The crate provides this error *shape* and the
    /// [`crate::MigrationReceipt::is_idempotent_retry_of`] predicate; the engine
    /// decides when to raise it. Idempotent retries (same id + same old-state +
    /// same target definition) are accepted and return the existing receipt.
    #[error("duplicate migration id {0:?}: a different migration already used this id")]
    DuplicateMigrationId(String),

    /// A migration's pre-commit invariant check failed against the dry-run
    /// snapshot. The prior state stays authoritative (migrations steering: a
    /// pre-commit failure never mutates state). Raised by the engine.
    #[error("migration pre-commit invariant failure: {0}")]
    PreCommitInvariantFailure(String),

    /// A migration was asked to commit at a tick before its activation tick.
    #[error("migration inactive at tick {requested}: activation tick is {activation}")]
    MigrationInactiveAtTick {
        /// The tick the engine attempted to commit the migration at.
        requested: u64,
        /// The migration's declared activation tick.
        activation: u64,
    },

    /// A migration's approvals were missing, invalid, expired, duplicated, or
    /// incompatible with the referenced governance policy.
    ///
    /// The crate provides this error *shape* and the
    /// [`crate::GovernancePolicyRef`]; the external simulation repo / validator
    /// network enforces the policy and supplies the reason. Never a silent
    /// success or history rewrite.
    #[error("unauthorized migration: {0}")]
    UnauthorizedMigration(String),
}
