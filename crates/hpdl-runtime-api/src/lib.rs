//! # hpdl-runtime-api
//!
//! Published **runtime contracts** for HPDL: the stable types the external
//! HORIZON simulation engine uses to execute actions and record a deterministic,
//! replayable event history. Activated in feature 10 (ADR 0004).
//!
//! ## What this crate is — and is not
//!
//! This crate defines **contracts**, not an engine. HPDL does not evolve state;
//! a separate authoritative simulation engine does. These types describe:
//!
//! - [`Tick`] — a monotonic integer time step (the unit of deterministic ordering).
//! - [`ActionId`] — a validated identifier for an action instance.
//! - [`StateHash`] — a content hash of canonical state bytes (SHA-256 with a
//!   state-specific domain separator, reusing the PIR hashing discipline).
//! - [`ReadWriteSet`] — the deterministic, ordered read/write field paths an
//!   action touches.
//! - [`Capability`] — a namespaced authority token an action requires to mutate state.
//! - [`EventEnvelope`] — a replay record that binds an action instance, its tick,
//!   the acting module+version, input/output state hashes, and the read/write set,
//!   with a pure hash-consistency check.
//! - [`ProcessContract`] / [`EvaluatorContract`] — the run contracts for versioned
//!   process and vitality-evaluator modules (feature 12): which module, what it may
//!   touch, from when, and against which baseline/window.
//! - [`ProcessReceipt`] — the hash-bound record of one process run at one tick
//!   (dedicated `HPDL-PROCESS-v1` binding separator), with ordering and
//!   replay-duplicate identity.
//! - [`StatusResult`] — the derived vitality-evaluator output (relocated from
//!   `hpdl-model` in feature 12).
//!
//! The engine is responsible for *executing* actions, *producing* events,
//! enforcing *authorization*, ordering events, and rejecting replay duplicates.
//! This crate gives it the stable, deterministic, integer- and hash-based shapes
//! that make byte-identical replay possible. The crate's own tests use a **mock
//! executor** (a clearly-labelled test double) to exercise those behaviors; the
//! crate ships no executor.
//!
//! ## Design constraints
//!
//! - **No `f32`/`f64`.** Ticks and ids are integers; everything else is bytes/hashes.
//! - **Deterministic.** Ordered collections; a documented total ordering key; no
//!   wall-clock, locale, or hashmap iteration in contract bytes.
//! - **Minimal, semver-stable surface.** Depends only on `hpdl-pir` and selected
//!   `hpdl-model` types.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::float_arithmetic
)]

mod error;
mod event;
mod hashing;
mod ids;
mod migration;
mod process;
mod vitality;

pub use error::Error;
pub use event::{EventEnvelope, EventOrderKey};
pub use ids::{ActionId, MigrationId, ProcessId, StateHash, Tick};
pub use migration::{
    GovernancePolicyRef, MigrationExecutor, MigrationOrderKey, MigrationProposal, MigrationReceipt,
    MigrationResult,
};
pub use process::{EvaluatorContract, ProcessContract, ProcessOrderKey, ProcessReceipt};
pub use vitality::StatusResult;

// Declaration-level vocabulary lives in `hpdl-model` (the dependency root) and is
// re-exported here so runtime consumers have a single import point for the whole
// contract surface. `Capability`/`ReadWriteSet` since feature 10 (ADR 0004); the
// status-vitality declaration types and `ModuleRef` since feature 12 (ADR 0006),
// used by the process/evaluator run contracts.
pub use hpdl_model::{
    BaselineRef, BodyId, Capability, Digest, EvaluationWindow, EvaluatorRef, ModuleRef,
    ReadWriteSet, StatusDimension,
};
