//! Core identifier and value types: [`Tick`], [`ActionId`], [`StateHash`].

use serde::{Deserialize, Serialize};

use crate::Error;

/// A monotonic integer time step — the unit of deterministic event ordering.
///
/// Ticks are a `u64`. Ordering within and across ticks is total and documented
/// (see [`crate::EventOrderKey`]); there is no wall-clock involvement.
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::Tick;
///
/// let t = Tick::new(42);
/// assert_eq!(t.get(), 42);
/// assert!(t.next().unwrap().get() == 43);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Tick(u64);

impl Tick {
    /// Create a tick from a raw value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The raw tick value.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The next tick, using checked arithmetic.
    ///
    /// Returns `None` on overflow (never panics).
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// A validated identifier for an action instance.
///
/// Valid ids are 1–128 characters from `[A-Za-z0-9_\-.:]` (the colon allows
/// namespaced ids such as `terraform:step-3`).
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::ActionId;
///
/// assert_eq!(ActionId::new("terraform:step-3").unwrap().as_str(), "terraform:step-3");
/// assert!(ActionId::new("bad id!").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActionId(String);

impl ActionId {
    /// Construct a validated `ActionId`.
    ///
    /// Returns [`Error::InvalidActionId`] on an empty, too-long, or
    /// invalid-character id.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if s.is_empty()
            || s.len() > 128
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
        {
            return Err(Error::InvalidActionId(s));
        }
        Ok(Self(s))
    }

    /// The underlying id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated identifier for a process-run instance.
///
/// Valid ids are 1–128 characters from `[A-Za-z0-9_\-.:]` — the same grammar as
/// [`ActionId`], but a distinct type so a *process run* is never conflated with an
/// *action event* (see spec 12, OD-12.1).
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::ProcessId;
///
/// assert_eq!(ProcessId::new("hydrology:tick").unwrap().as_str(), "hydrology:tick");
/// assert!(ProcessId::new("bad id!").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProcessId(String);

impl ProcessId {
    /// Construct a validated `ProcessId`.
    ///
    /// Returns [`Error::InvalidProcessId`] on an empty, too-long, or
    /// invalid-character id.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if s.is_empty()
            || s.len() > 128
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
        {
            return Err(Error::InvalidProcessId(s));
        }
        Ok(Self(s))
    }

    /// The underlying id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated identifier for a schema-migration instance.
///
/// Valid ids are 1–128 characters from `[A-Za-z0-9_\-.:]` — the same grammar as
/// [`ActionId`]/[`ProcessId`], but a distinct type so a *migration* is never
/// conflated with an *action event* or a *process run* (see spec 13, OD-13.2).
///
/// A [`crate::MigrationProposal`] derives its `MigrationId` deterministically from
/// its binding fields, so the same proposal always has the same id and retries are
/// idempotent (OD-13.4).
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::MigrationId;
///
/// assert_eq!(MigrationId::new("m-abc123").unwrap().as_str(), "m-abc123");
/// assert!(MigrationId::new("bad id!").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MigrationId(String);

impl MigrationId {
    /// Construct a validated `MigrationId`.
    ///
    /// Returns [`Error::InvalidMigrationId`] on an empty, too-long, or
    /// invalid-character id.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if s.is_empty()
            || s.len() > 128
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
        {
            return Err(Error::InvalidMigrationId(s));
        }
        Ok(Self(s))
    }

    /// The underlying id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A deterministic, always-valid fallback id (`m-0`).
    ///
    /// Used only on an unreachable error path (deriving an id from a hash can
    /// never produce an invalid grammar) so callers need no `unwrap`/`expect`.
    pub(crate) fn zero_fallback() -> Self {
        Self("m-0".to_string())
    }
}

/// A content hash of canonical state bytes.
///
/// `StateHash` is a validated 64-character lowercase hex string (SHA-256). The
/// *computation* of a state hash from real state is the engine's responsibility,
/// using a state-specific domain separator — this crate deliberately does not own
/// the state-canonicalization algorithm, only the stable value shape. Tests
/// construct `StateHash` values directly or from a mock hasher.
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::StateHash;
///
/// let h = StateHash::from_hex("a".repeat(64)).unwrap();
/// assert_eq!(h.as_hex().len(), 64);
/// assert!(StateHash::from_hex("TOOSHORT").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StateHash(String);

impl StateHash {
    /// Construct a `StateHash` from a 64-char lowercase hex string.
    ///
    /// Returns [`Error::InvalidStateHash`] otherwise.
    pub fn from_hex(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if s.len() != 64
            || !s
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            return Err(Error::InvalidStateHash(s));
        }
        Ok(Self(s))
    }

    /// The all-zero state hash (64 zero hex chars).
    ///
    /// Used only as a deterministic fallback on an unreachable error path; not a
    /// meaningful state.
    pub fn zero() -> Self {
        Self("0".repeat(64))
    }

    /// The underlying lowercase hex string.
    pub fn as_hex(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn tick_basics() {
        let t = Tick::new(10);
        assert_eq!(t.get(), 10);
        assert_eq!(t.next().unwrap(), Tick::new(11));
        assert_eq!(Tick::new(u64::MAX).next(), None); // checked, no panic
    }

    #[test]
    fn action_id_validation() {
        assert!(ActionId::new("terraform:step-3").is_ok());
        assert!(ActionId::new("a.b_c-d").is_ok());
        assert!(ActionId::new("").is_err());
        assert!(ActionId::new("bad id").is_err());
        assert!(ActionId::new("a".repeat(129)).is_err());
    }

    #[test]
    fn process_id_validation() {
        assert!(ProcessId::new("hydrology:tick").is_ok());
        assert!(ProcessId::new("a.b_c-d").is_ok());
        assert!(ProcessId::new("").is_err());
        assert!(ProcessId::new("bad id").is_err());
        assert!(ProcessId::new("a".repeat(129)).is_err());
    }

    #[test]
    fn migration_id_validation() {
        assert!(MigrationId::new("m-abc123").is_ok());
        assert!(MigrationId::new("schema:1-to-2").is_ok());
        assert!(MigrationId::new("").is_err());
        assert!(MigrationId::new("bad id").is_err());
        assert!(MigrationId::new("a".repeat(129)).is_err());
    }

    #[test]
    fn state_hash_validation() {
        assert!(StateHash::from_hex("0123456789abcdef".repeat(4)).is_ok());
        assert!(StateHash::from_hex("a".repeat(64)).is_ok());
        assert!(StateHash::from_hex("A".repeat(64)).is_err()); // uppercase
        assert!(StateHash::from_hex("g".repeat(64)).is_err()); // non-hex
        assert!(StateHash::from_hex("short").is_err());
    }

    #[test]
    fn ordering_is_total_and_stable() {
        // Ticks and ids order deterministically.
        assert!(Tick::new(1) < Tick::new(2));
        assert!(ActionId::new("a").unwrap() < ActionId::new("b").unwrap());
    }
}
