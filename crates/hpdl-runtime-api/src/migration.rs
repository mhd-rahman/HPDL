//! Schema-migration **execution contracts** (HPDL feature 13, domain 13).
//!
//! These are the stable shapes the external HORIZON engine + validator network use
//! to *run* an authorized, atomic, forward-only schema migration:
//!
//! - [`MigrationProposal`] — everything the migrations-steering protocol step 1
//!   names (world id, source/target definition + schema hashes, implementation
//!   module + digest, activation tick, nonce, governance-policy reference), with a
//!   deterministically **derived** [`MigrationId`].
//! - [`MigrationReceipt`] — the hash-bound record of one committed migration at one
//!   tick, bound under a dedicated `HPDL-MIGRATION-v1` separator.
//! - [`MigrationExecutor`] — the dry-run → validate → atomic-commit interface the
//!   engine implements. The crate ships the trait + receipt/error *shapes* only.
//!
//! HPDL ships **contracts, not an engine**. Nothing here takes a snapshot, mutates
//! state, verifies a signature, or enforces a quorum; the crate's tests use a
//! **mock executor** double. Authorization is *referenced* via
//! [`GovernancePolicyRef`] and *enforced* externally.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::hashing::{feed, to_lower_hex};
use crate::{BodyId, Digest, Error, MigrationId, ModuleRef, StateHash, Tick};

/// Domain separator for migration-receipt and migration-id binding hashes.
///
/// Distinct from the PIR (`HPDL-PIR-v2`), event (`HPDL-EVENT-v1`), and process
/// (`HPDL-PROCESS-v1`) separators so a migration binding can never collide with a
/// PIR content hash, an event binding, or a process receipt.
const MIGRATION_DOMAIN_SEPARATOR: &[u8] = b"HPDL-MIGRATION-v1";

/// Maximum length of a governance-policy id.
const MAX_POLICY_ID_LEN: usize = 128;

/// A reference to a versioned governance / migration-authorization policy.
///
/// This is a *reference* only: a policy id plus the content hash of the policy it
/// names. It carries **no** operator keys, validator set, threshold, signature
/// suite, or rotation/revocation rules — those live in the external
/// `MigrationAuthorizationPolicy` (migrations steering ownership: the simulation
/// repo). The engine resolves this reference, enforces the policy, and raises
/// [`Error::UnauthorizedMigration`] on failure.
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::{Digest, GovernancePolicyRef};
///
/// let p = GovernancePolicyRef::new("federated.v1", Digest::new("a".repeat(64)).unwrap()).unwrap();
/// assert_eq!(p.policy_id(), "federated.v1");
/// assert!(GovernancePolicyRef::new("", Digest::new("a".repeat(64)).unwrap()).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernancePolicyRef {
    policy_id: String,
    policy_hash: Digest,
}

impl GovernancePolicyRef {
    /// Construct a validated `GovernancePolicyRef`.
    ///
    /// Returns [`Error::UnauthorizedMigration`] if `policy_id` is empty or longer
    /// than 128 characters. (`policy_hash` is already validated by [`Digest`].)
    pub fn new(policy_id: impl Into<String>, policy_hash: Digest) -> Result<Self, Error> {
        let policy_id = policy_id.into();
        if policy_id.is_empty() || policy_id.len() > MAX_POLICY_ID_LEN {
            return Err(Error::UnauthorizedMigration(format!(
                "malformed policy id {policy_id:?}: must be 1–128 chars"
            )));
        }
        Ok(Self {
            policy_id,
            policy_hash,
        })
    }

    /// The policy id.
    pub fn policy_id(&self) -> &str {
        &self.policy_id
    }

    /// The content hash of the referenced policy.
    pub fn policy_hash(&self) -> &Digest {
        &self.policy_hash
    }
}

/// A proposal to run an atomic, forward-only schema migration.
///
/// Names everything the migrations-steering protocol step 1 requires: the world
/// ([`BodyId`]), the source/target *definition* hashes and *schema* versions, the
/// migration implementation [`ModuleRef`] and its [`Digest`], the activation
/// [`Tick`], a `nonce`, and the [`GovernancePolicyRef`]. Forward-only is enforced
/// at construction (`to_schema > from_schema`).
///
/// The [`MigrationId`] is **derived** from the binding fields, so the same proposal
/// always has the same id and retries are idempotent by construction (OD-13.4).
/// HPDL declares the proposal; the external engine dry-runs, authorizes, and
/// commits it.
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::{
///     BodyId, Digest, GovernancePolicyRef, MigrationProposal, ModuleRef, Tick,
/// };
///
/// let proposal = MigrationProposal::new(
///     BodyId::new("kepler-22b").unwrap(),
///     Digest::new("a".repeat(64)).unwrap(),
///     Digest::new("b".repeat(64)).unwrap(),
///     1, 2,
///     ModuleRef::new("hpdl.migrate.add-ocean", "1.0").unwrap(),
///     Digest::new("c".repeat(64)).unwrap(),
///     Tick::new(1000),
///     42,
///     GovernancePolicyRef::new("operator.v1", Digest::new("d".repeat(64)).unwrap()).unwrap(),
/// ).unwrap();
/// assert!(proposal.is_active_at(Tick::new(1000)));   // inclusive
/// assert!(!proposal.is_active_at(Tick::new(999)));
/// // Deriving the id twice from the same proposal yields the same id.
/// assert_eq!(proposal.migration_id(), proposal.migration_id());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationProposal {
    body: BodyId,
    from_definition: Digest,
    to_definition: Digest,
    from_schema: u32,
    to_schema: u32,
    implementation: ModuleRef,
    implementation_digest: Digest,
    activation_tick: Tick,
    nonce: u64,
    policy: GovernancePolicyRef,
}

impl MigrationProposal {
    /// Construct a validated `MigrationProposal`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SchemaDowngrade`] if `to_schema <= from_schema` (migrations
    /// are forward-only). All newtype fields are already validated by their own
    /// constructors.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        body: BodyId,
        from_definition: Digest,
        to_definition: Digest,
        from_schema: u32,
        to_schema: u32,
        implementation: ModuleRef,
        implementation_digest: Digest,
        activation_tick: Tick,
        nonce: u64,
        policy: GovernancePolicyRef,
    ) -> Result<Self, Error> {
        if to_schema <= from_schema {
            return Err(Error::SchemaDowngrade {
                from: from_schema,
                to: to_schema,
            });
        }
        Ok(Self {
            body,
            from_definition,
            to_definition,
            from_schema,
            to_schema,
            implementation,
            implementation_digest,
            activation_tick,
            nonce,
            policy,
        })
    }

    /// Whether the migration is live at `tick` (inclusive: `tick >= activation`).
    pub fn is_active_at(&self, tick: Tick) -> bool {
        tick.get() >= self.activation_tick.get()
    }

    /// The world this migration applies to.
    pub fn body(&self) -> &BodyId {
        &self.body
    }

    /// The source definition hash.
    pub fn from_definition(&self) -> &Digest {
        &self.from_definition
    }

    /// The target definition hash.
    pub fn to_definition(&self) -> &Digest {
        &self.to_definition
    }

    /// The source schema version.
    pub fn from_schema(&self) -> u32 {
        self.from_schema
    }

    /// The target schema version.
    pub fn to_schema(&self) -> u32 {
        self.to_schema
    }

    /// The migration implementation module.
    pub fn implementation(&self) -> &ModuleRef {
        &self.implementation
    }

    /// The migration implementation content digest.
    pub fn implementation_digest(&self) -> &Digest {
        &self.implementation_digest
    }

    /// The activation tick (first tick at which the migration may commit).
    pub fn activation_tick(&self) -> Tick {
        self.activation_tick
    }

    /// The replay nonce.
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// The governance-policy reference the engine enforces against.
    pub fn policy(&self) -> &GovernancePolicyRef {
        &self.policy
    }
}

impl MigrationProposal {
    /// The deterministically derived migration id.
    ///
    /// SHA-256 over the proposal's binding fields under the `HPDL-MIGRATION-v1`
    /// separator, encoded as `m-` + lowercase hex. Because the id is a pure
    /// function of the proposal, the same proposal always yields the same id and
    /// retries are idempotent by construction (OD-13.4). Pure; never panics.
    pub fn migration_id(&self) -> MigrationId {
        let mut hasher = Sha256::new();
        hasher.update(MIGRATION_DOMAIN_SEPARATOR);
        feed(&mut hasher, b"id");
        feed(&mut hasher, self.body.as_str().as_bytes());
        feed(&mut hasher, self.from_definition.as_str().as_bytes());
        feed(&mut hasher, self.to_definition.as_str().as_bytes());
        hasher.update(self.from_schema.to_be_bytes());
        hasher.update(self.to_schema.to_be_bytes());
        feed(&mut hasher, self.implementation.name().as_bytes());
        feed(&mut hasher, self.implementation.version().as_bytes());
        feed(&mut hasher, self.implementation_digest.as_str().as_bytes());
        hasher.update(self.activation_tick.get().to_be_bytes());
        hasher.update(self.nonce.to_be_bytes());
        feed(&mut hasher, self.policy.policy_id().as_bytes());
        feed(&mut hasher, self.policy.policy_hash().as_str().as_bytes());
        let digest = hasher.finalize();
        let id = format!("m-{}", to_lower_hex(&digest));
        // `id` is always `m-` + 64 lowercase hex chars (valid grammar). The
        // `unwrap_or_else` fallback (`m-0`, also valid) exists only to satisfy the
        // crate's no-unwrap lint on the impossible error path; it never fires.
        MigrationId::new(id).unwrap_or_else(|_| MigrationId::zero_fallback())
    }
}

/// The result of attempting to apply a migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationResult {
    /// The migration committed atomically; the new state is authoritative. A
    /// committed migration is immutable and corrected only by a NEW forward
    /// migration (forward-fix only).
    Committed,
    /// A pre-commit invariant failed; the prior state stayed authoritative and
    /// nothing was mutated.
    RejectedPreCommit,
}

impl MigrationResult {
    /// A single discriminant byte for the binding hash.
    fn discriminant(self) -> u8 {
        match self {
            Self::Committed => 1,
            Self::RejectedPreCommit => 2,
        }
    }
}

/// The total ordering key for a [`MigrationReceipt`]: `(tick, migration_id)`.
///
/// Deterministic and total — never hashmap iteration or wall-clock. The engine
/// orders committed migrations by this key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MigrationOrderKey {
    /// The commit tick.
    pub tick: Tick,
    /// The migration id.
    pub migration_id: MigrationId,
}

/// A deterministic, hash-bound record of one migration apply at one tick.
///
/// Binds the migration id, tick, old/new state hashes, old/new definition hashes,
/// implementation digest, policy reference, and result. The [`binding`] is SHA-256
/// over the canonical bytes of those fields under the dedicated `HPDL-MIGRATION-v1`
/// separator, so a replaying engine can verify integrity.
///
/// This is a **contract**: the engine produces receipts; HPDL does not run
/// migrations. A `Committed` receipt is immutable (forward-fix only); a
/// `RejectedPreCommit` receipt records that the prior state stayed authoritative.
///
/// [`binding`]: MigrationReceipt::binding
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::{
///     Digest, GovernancePolicyRef, MigrationId, MigrationReceipt, MigrationResult,
///     StateHash, Tick,
/// };
///
/// let r = MigrationReceipt::new(
///     MigrationId::new("m-1").unwrap(),
///     Tick::new(1000),
///     StateHash::from_hex("a".repeat(64)).unwrap(),
///     StateHash::from_hex("b".repeat(64)).unwrap(),
///     Digest::new("c".repeat(64)).unwrap(),
///     Digest::new("d".repeat(64)).unwrap(),
///     Digest::new("e".repeat(64)).unwrap(),
///     GovernancePolicyRef::new("op.v1", Digest::new("f".repeat(64)).unwrap()).unwrap(),
///     MigrationResult::Committed,
/// );
/// assert!(r.is_consistent());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReceipt {
    migration_id: MigrationId,
    tick: Tick,
    old_state: StateHash,
    new_state: StateHash,
    from_definition: Digest,
    to_definition: Digest,
    implementation_digest: Digest,
    policy: GovernancePolicyRef,
    result: MigrationResult,
    /// SHA-256 (hex) binding over the canonical bytes of all other fields.
    binding: StateHash,
}

impl MigrationReceipt {
    /// Construct a `MigrationReceipt`, computing its binding hash from the fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        migration_id: MigrationId,
        tick: Tick,
        old_state: StateHash,
        new_state: StateHash,
        from_definition: Digest,
        to_definition: Digest,
        implementation_digest: Digest,
        policy: GovernancePolicyRef,
        result: MigrationResult,
    ) -> Self {
        let binding = Self::compute_binding(
            &migration_id,
            tick,
            &old_state,
            &new_state,
            &from_definition,
            &to_definition,
            &implementation_digest,
            &policy,
            result,
        );
        Self {
            migration_id,
            tick,
            old_state,
            new_state,
            from_definition,
            to_definition,
            implementation_digest,
            policy,
            result,
            binding,
        }
    }

    /// The total ordering key for this receipt, `(tick, migration_id)`.
    pub fn order_key(&self) -> MigrationOrderKey {
        MigrationOrderKey {
            tick: self.tick,
            migration_id: self.migration_id.clone(),
        }
    }

    /// Whether the recorded binding matches a freshly-computed binding. A `false`
    /// result means the record was tampered with or mis-serialized. Pure, no
    /// floats, never panics.
    pub fn is_consistent(&self) -> bool {
        let recomputed = Self::compute_binding(
            &self.migration_id,
            self.tick,
            &self.old_state,
            &self.new_state,
            &self.from_definition,
            &self.to_definition,
            &self.implementation_digest,
            &self.policy,
            self.result,
        );
        recomputed == self.binding
    }

    /// Whether this receipt is an idempotent retry of `other`: the same migration
    /// id applied to the same old state producing the same target definition. A
    /// replaying engine uses this to dedupe retries (migrations are idempotent via
    /// a unique migration id).
    pub fn is_idempotent_retry_of(&self, other: &MigrationReceipt) -> bool {
        self.migration_id == other.migration_id
            && self.old_state == other.old_state
            && self.to_definition == other.to_definition
    }
}

impl MigrationReceipt {
    /// The migration id.
    pub fn migration_id(&self) -> &MigrationId {
        &self.migration_id
    }

    /// The commit tick.
    pub fn tick(&self) -> Tick {
        self.tick
    }

    /// The old (pre-migration) state hash.
    pub fn old_state(&self) -> &StateHash {
        &self.old_state
    }

    /// The new (post-migration) state hash.
    pub fn new_state(&self) -> &StateHash {
        &self.new_state
    }

    /// The source definition hash.
    pub fn from_definition(&self) -> &Digest {
        &self.from_definition
    }

    /// The target definition hash.
    pub fn to_definition(&self) -> &Digest {
        &self.to_definition
    }

    /// The migration implementation digest.
    pub fn implementation_digest(&self) -> &Digest {
        &self.implementation_digest
    }

    /// The governance-policy reference the migration was authorized against.
    pub fn policy(&self) -> &GovernancePolicyRef {
        &self.policy
    }

    /// The result (committed or rejected pre-commit).
    pub fn result(&self) -> MigrationResult {
        self.result
    }

    /// The binding hash (hex).
    pub fn binding(&self) -> &StateHash {
        &self.binding
    }

    /// Compute the deterministic binding hash over the canonical bytes of the
    /// bound fields, under the dedicated migration domain separator. Field order
    /// and separators are fixed, so the result is stable.
    #[allow(clippy::too_many_arguments)]
    fn compute_binding(
        migration_id: &MigrationId,
        tick: Tick,
        old_state: &StateHash,
        new_state: &StateHash,
        from_definition: &Digest,
        to_definition: &Digest,
        implementation_digest: &Digest,
        policy: &GovernancePolicyRef,
        result: MigrationResult,
    ) -> StateHash {
        let mut hasher = Sha256::new();
        hasher.update(MIGRATION_DOMAIN_SEPARATOR);
        feed(&mut hasher, migration_id.as_str().as_bytes());
        hasher.update(tick.get().to_be_bytes());
        feed(&mut hasher, old_state.as_hex().as_bytes());
        feed(&mut hasher, new_state.as_hex().as_bytes());
        feed(&mut hasher, from_definition.as_str().as_bytes());
        feed(&mut hasher, to_definition.as_str().as_bytes());
        feed(&mut hasher, implementation_digest.as_str().as_bytes());
        feed(&mut hasher, policy.policy_id().as_bytes());
        feed(&mut hasher, policy.policy_hash().as_str().as_bytes());
        hasher.update([result.discriminant()]);
        let digest = hasher.finalize();
        let hex = to_lower_hex(&digest);
        // `hex` is always 64 lowercase hex chars; fall back to zero on the
        // impossible error path to avoid unwrap/expect per crate lints.
        StateHash::from_hex(hex).unwrap_or_else(|_| StateHash::zero())
    }
}

/// The contract the external HORIZON engine implements to run a migration.
///
/// This is an **interface only** — `hpdl-runtime-api` ships no implementation.
/// The protocol (migrations steering) is: `dry_run` the proposal's invariants
/// against a snapshot, then `commit` atomically at the activation tick boundary. A
/// pre-commit failure leaves the prior state authoritative. The crate's
/// `tests/mock_migration_executor.rs` provides a **mock** implementation to
/// exercise the contract; it is a test double, not a simulation.
///
/// Implementors own snapshotting, state transformation, signature verification,
/// and quorum enforcement — none of which live in this crate.
pub trait MigrationExecutor {
    /// Validate the proposal's invariants against a snapshot state hash without
    /// mutating anything.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PreCommitInvariantFailure`] if an invariant does not hold,
    /// or [`Error::MigrationInactiveAtTick`] / [`Error::UnauthorizedMigration`] as
    /// appropriate. Never mutates state.
    fn dry_run(
        &self,
        proposal: &MigrationProposal,
        snapshot_state: &StateHash,
    ) -> Result<(), Error>;

    /// Commit the migration atomically at its activation tick, returning a
    /// hash-bound [`MigrationReceipt`].
    ///
    /// # Errors
    ///
    /// Returns a typed [`Error`] (inactive-at-tick, unauthorized, duplicate id, or
    /// pre-commit failure). On a pre-commit failure the prior state stays
    /// authoritative.
    fn commit(
        &self,
        proposal: &MigrationProposal,
        old_state: &StateHash,
    ) -> Result<MigrationReceipt, Error>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn digest(c: char) -> Digest {
        Digest::new(c.to_string().repeat(64)).unwrap()
    }

    fn state(c: char) -> StateHash {
        StateHash::from_hex(c.to_string().repeat(64)).unwrap()
    }

    fn policy() -> GovernancePolicyRef {
        GovernancePolicyRef::new("operator.v1", digest('f')).unwrap()
    }

    fn proposal(from: u32, to: u32) -> MigrationProposal {
        MigrationProposal::new(
            BodyId::new("kepler-22b").unwrap(),
            digest('a'),
            digest('b'),
            from,
            to,
            ModuleRef::new("hpdl.migrate.add-ocean", "1.0").unwrap(),
            digest('c'),
            Tick::new(1000),
            42,
            policy(),
        )
        .unwrap()
    }

    fn receipt(result: MigrationResult) -> MigrationReceipt {
        let p = proposal(1, 2);
        MigrationReceipt::new(
            p.migration_id(),
            Tick::new(1000),
            state('1'),
            state('2'),
            digest('a'),
            digest('b'),
            digest('c'),
            policy(),
            result,
        )
    }

    // ----- GovernancePolicyRef -----

    #[test]
    fn policy_ref_rejects_empty_id() {
        assert!(GovernancePolicyRef::new("", digest('f')).is_err());
    }

    #[test]
    fn policy_ref_rejects_overlong_id() {
        assert!(GovernancePolicyRef::new("x".repeat(129), digest('f')).is_err());
    }

    // ----- MigrationProposal -----

    #[test]
    fn forward_only_accepts_upgrade() {
        assert!(MigrationProposal::new(
            BodyId::new("w").unwrap(),
            digest('a'),
            digest('b'),
            1,
            2,
            ModuleRef::new("m", "1.0").unwrap(),
            digest('c'),
            Tick::new(0),
            0,
            policy(),
        )
        .is_ok());
    }

    #[test]
    fn forward_only_rejects_downgrade_and_equal() {
        assert!(matches!(
            MigrationProposal::new(
                BodyId::new("w").unwrap(),
                digest('a'),
                digest('b'),
                3,
                2,
                ModuleRef::new("m", "1.0").unwrap(),
                digest('c'),
                Tick::new(0),
                0,
                policy(),
            ),
            Err(Error::SchemaDowngrade { from: 3, to: 2 })
        ));
        assert!(matches!(
            MigrationProposal::new(
                BodyId::new("w").unwrap(),
                digest('a'),
                digest('b'),
                2,
                2,
                ModuleRef::new("m", "1.0").unwrap(),
                digest('c'),
                Tick::new(0),
                0,
                policy(),
            ),
            Err(Error::SchemaDowngrade { from: 2, to: 2 })
        ));
    }

    #[test]
    fn is_active_at_is_inclusive() {
        let p = proposal(1, 2);
        assert!(p.is_active_at(Tick::new(1000)));
        assert!(p.is_active_at(Tick::new(1001)));
        assert!(!p.is_active_at(Tick::new(999)));
    }

    #[test]
    fn migration_id_is_deterministic_and_valid() {
        let p = proposal(1, 2);
        let id1 = p.migration_id();
        let id2 = p.migration_id();
        assert_eq!(id1, id2);
        assert!(id1.as_str().starts_with("m-"));
        assert_eq!(id1.as_str().len(), 66);
    }

    #[test]
    fn migration_id_differs_for_different_proposals() {
        let a = proposal(1, 2);
        let b = proposal(1, 3);
        assert_ne!(a.migration_id(), b.migration_id());
    }

    // ----- MigrationReceipt -----

    #[test]
    fn receipt_is_consistent() {
        assert!(receipt(MigrationResult::Committed).is_consistent());
        assert!(receipt(MigrationResult::RejectedPreCommit).is_consistent());
    }

    #[test]
    fn tampered_receipt_is_inconsistent() {
        let mut r = receipt(MigrationResult::Committed);
        r.new_state = state('9');
        assert!(!r.is_consistent());
    }

    #[test]
    fn committed_and_rejected_bind_differently() {
        let c = receipt(MigrationResult::Committed);
        let r = receipt(MigrationResult::RejectedPreCommit);
        assert_ne!(c.binding(), r.binding());
    }

    #[test]
    fn idempotent_retry_identity() {
        let a = receipt(MigrationResult::Committed);
        let b = receipt(MigrationResult::Committed);
        assert!(a.is_idempotent_retry_of(&b));

        // Different target definition => not the same migration.
        let p = proposal(1, 2);
        let c = MigrationReceipt::new(
            p.migration_id(),
            Tick::new(1000),
            state('1'),
            state('2'),
            digest('a'),
            digest('e'),
            digest('c'),
            policy(),
            MigrationResult::Committed,
        );
        assert!(!a.is_idempotent_retry_of(&c));
    }

    #[test]
    fn order_key_is_tick_then_id() {
        let r = receipt(MigrationResult::Committed);
        let key = r.order_key();
        assert_eq!(key.tick, Tick::new(1000));
        assert_eq!(&key.migration_id, r.migration_id());
    }
}
