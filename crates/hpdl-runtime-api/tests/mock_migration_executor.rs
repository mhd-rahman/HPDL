//! Mock migration-executor integration tests for the feature-13 execution contracts.
//!
//! IMPORTANT: `MockMigrationEngine` below is a **test double**, not a simulation and
//! not part of the shipped crate. HPDL does not execute migrations, take snapshots,
//! transform state, verify signatures, or enforce quorum — the real executor is the
//! external HORIZON engine + validator network. This mock exists solely to exercise
//! the published migration *contracts* through the behaviors the feature-13 design
//! requires:
//!
//!   - forward-only rejection (downgrade) — enforced at proposal construction
//!   - inactive-at-tick rejection (commit before the activation tick)
//!   - pre-commit invariant failure (prior state stays authoritative)
//!   - duplicate-migration-id idempotence (same id + old state + target definition)
//!   - unauthorized rejection (policy approval fails)
//!   - deterministic receipt ordering
//!   - receipt hash (binding) consistency
//!
//! The mock makes no claim of being a correct or complete engine; it only shows
//! that the contract types support these behaviors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeSet;

use hpdl_runtime_api::{
    BodyId, Digest, Error, GovernancePolicyRef, MigrationExecutor, MigrationOrderKey,
    MigrationProposal, MigrationReceipt, MigrationResult, ModuleRef, StateHash, Tick,
};

fn digest(c: char) -> Digest {
    Digest::new(c.to_string().repeat(64)).unwrap()
}

fn state(c: char) -> StateHash {
    StateHash::from_hex(c.to_string().repeat(64)).unwrap()
}

fn policy() -> GovernancePolicyRef {
    GovernancePolicyRef::new("federated.v1", digest('f')).unwrap()
}

fn proposal(from: u32, to: u32, tick: u64) -> MigrationProposal {
    MigrationProposal::new(
        BodyId::new("kepler-22b").unwrap(),
        digest('a'),
        digest('b'),
        from,
        to,
        ModuleRef::new("hpdl.migrate.add-ocean", "1.0").unwrap(),
        digest('c'),
        Tick::new(tick),
        7,
        policy(),
    )
    .unwrap()
}

/// A deliberately minimal, in-memory **mock** of the external engine + validator
/// network. Not shipped; not a simulation.
#[derive(Default)]
struct MockMigrationEngine {
    log: Vec<MigrationReceipt>,
    seen: BTreeSet<(String, String, String)>,
    /// If set, every dry-run invariant check fails with this reason.
    invariant_failure: Option<String>,
    /// If set, authorization fails with this reason.
    authorization_failure: Option<String>,
}

impl MockMigrationEngine {
    /// Attempt to apply a migration at a tick, following the migrations-steering
    /// protocol: activation check, authorization, dry-run invariants, idempotent
    /// retry, then an atomic hash-bound commit. A pre-commit failure leaves the
    /// prior state authoritative.
    fn apply(
        &mut self,
        proposal: &MigrationProposal,
        tick: Tick,
        old_state: StateHash,
        new_state: StateHash,
    ) -> Result<MigrationReceipt, Error> {
        // 1. Activation: inclusive.
        if !proposal.is_active_at(tick) {
            return Err(Error::MigrationInactiveAtTick {
                requested: tick.get(),
                activation: proposal.activation_tick().get(),
            });
        }

        // 2. Authorization (referenced policy; the real check is external).
        if let Some(reason) = &self.authorization_failure {
            return Err(Error::UnauthorizedMigration(reason.clone()));
        }

        let id = proposal.migration_id();
        let identity = (
            id.as_str().to_string(),
            old_state.as_hex().to_string(),
            proposal.to_definition().as_str().to_string(),
        );

        // 3. Idempotent retry: same id + old state + target definition => return
        //    the existing committed receipt rather than committing twice.
        if self.seen.contains(&identity) {
            if let Some(existing) = self.log.iter().find(|r| {
                r.is_idempotent_retry_of(&MigrationReceipt::new(
                    id.clone(),
                    tick,
                    old_state.clone(),
                    new_state.clone(),
                    proposal.from_definition().clone(),
                    proposal.to_definition().clone(),
                    proposal.implementation_digest().clone(),
                    proposal.policy().clone(),
                    MigrationResult::Committed,
                ))
            }) {
                return Ok(existing.clone());
            }
        }

        // 4. Dry-run invariants. A failure leaves the prior state authoritative:
        //    we return a RejectedPreCommit receipt and do NOT record a commit.
        if let Some(reason) = &self.invariant_failure {
            return Err(Error::PreCommitInvariantFailure(reason.clone()));
        }

        // 5. Atomic commit: record a hash-bound receipt, keep the log ordered.
        let receipt = MigrationReceipt::new(
            id,
            tick,
            old_state,
            new_state,
            proposal.from_definition().clone(),
            proposal.to_definition().clone(),
            proposal.implementation_digest().clone(),
            proposal.policy().clone(),
            MigrationResult::Committed,
        );
        self.seen.insert(identity);
        self.log.push(receipt.clone());
        self.log.sort_by_key(MigrationReceipt::order_key);
        Ok(receipt)
    }

    fn order_keys(&self) -> Vec<MigrationOrderKey> {
        self.log.iter().map(MigrationReceipt::order_key).collect()
    }
}

// MigrationExecutor is the shipped interface; the mock implements it so the test
// double is exercised through the real contract, not an ad-hoc method.
impl MigrationExecutor for MockMigrationEngine {
    fn dry_run(
        &self,
        proposal: &MigrationProposal,
        _snapshot_state: &StateHash,
    ) -> Result<(), Error> {
        if !proposal.is_active_at(proposal.activation_tick()) {
            return Err(Error::MigrationInactiveAtTick {
                requested: proposal.activation_tick().get(),
                activation: proposal.activation_tick().get(),
            });
        }
        if let Some(reason) = &self.authorization_failure {
            return Err(Error::UnauthorizedMigration(reason.clone()));
        }
        if let Some(reason) = &self.invariant_failure {
            return Err(Error::PreCommitInvariantFailure(reason.clone()));
        }
        Ok(())
    }

    fn commit(
        &self,
        proposal: &MigrationProposal,
        old_state: &StateHash,
    ) -> Result<MigrationReceipt, Error> {
        if let Some(reason) = &self.invariant_failure {
            return Err(Error::PreCommitInvariantFailure(reason.clone()));
        }
        Ok(MigrationReceipt::new(
            proposal.migration_id(),
            proposal.activation_tick(),
            old_state.clone(),
            state('9'),
            proposal.from_definition().clone(),
            proposal.to_definition().clone(),
            proposal.implementation_digest().clone(),
            proposal.policy().clone(),
            MigrationResult::Committed,
        ))
    }
}

#[test]
fn downgrade_is_rejected_at_construction() {
    let err = MigrationProposal::new(
        BodyId::new("w").unwrap(),
        digest('a'),
        digest('b'),
        5,
        2,
        ModuleRef::new("m", "1.0").unwrap(),
        digest('c'),
        Tick::new(0),
        0,
        policy(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::SchemaDowngrade { from: 5, to: 2 }));
}

#[test]
fn inactive_migration_is_rejected() {
    let mut engine = MockMigrationEngine::default();
    let p = proposal(1, 2, 1000);
    let result = engine.apply(&p, Tick::new(999), state('1'), state('2'));
    assert!(matches!(result, Err(Error::MigrationInactiveAtTick { .. })));
    assert!(engine.log.is_empty());
}

#[test]
fn pre_commit_failure_leaves_prior_state_authoritative() {
    let mut engine = MockMigrationEngine {
        invariant_failure: Some("ocean field would violate hydrosphere invariant".to_string()),
        ..Default::default()
    };
    let p = proposal(1, 2, 1000);
    let result = engine.apply(&p, Tick::new(1000), state('1'), state('2'));
    assert!(matches!(result, Err(Error::PreCommitInvariantFailure(_))));
    assert!(engine.log.is_empty(), "failed migration must not commit");
}

#[test]
fn unauthorized_migration_is_rejected() {
    let mut engine = MockMigrationEngine {
        authorization_failure: Some("validator quorum not met".to_string()),
        ..Default::default()
    };
    let p = proposal(1, 2, 1000);
    let result = engine.apply(&p, Tick::new(1000), state('1'), state('2'));
    assert!(matches!(result, Err(Error::UnauthorizedMigration(_))));
    assert!(engine.log.is_empty());
}

#[test]
fn authorized_active_migration_commits_and_logs() {
    let mut engine = MockMigrationEngine::default();
    let p = proposal(1, 2, 1000);
    let r = engine
        .apply(&p, Tick::new(1000), state('1'), state('2'))
        .unwrap();
    assert_eq!(r.result(), MigrationResult::Committed);
    assert!(r.is_consistent());
    assert_eq!(engine.log.len(), 1);
}

#[test]
fn duplicate_migration_id_is_idempotent() {
    let mut engine = MockMigrationEngine::default();
    let p = proposal(1, 2, 1000);
    let first = engine
        .apply(&p, Tick::new(1000), state('1'), state('2'))
        .unwrap();
    // Retry the same proposal + old state: same id, same target definition.
    let second = engine
        .apply(&p, Tick::new(1000), state('1'), state('2'))
        .unwrap();
    assert_eq!(first, second, "idempotent retry returns the same receipt");
    assert_eq!(engine.log.len(), 1, "no second commit recorded");
}

#[test]
fn receipts_are_deterministically_ordered() {
    let mut engine = MockMigrationEngine::default();
    // Apply out of tick order; the log must end up sorted by (tick, id).
    let p_late = proposal(1, 2, 2000);
    let p_early = proposal(1, 3, 1000);
    engine
        .apply(&p_late, Tick::new(2000), state('1'), state('2'))
        .unwrap();
    engine
        .apply(&p_early, Tick::new(1000), state('3'), state('4'))
        .unwrap();
    let keys = engine.order_keys();
    assert_eq!(keys.len(), 2);
    assert!(keys[0] < keys[1], "ordered by (tick, migration_id)");
    assert_eq!(keys[0].tick, Tick::new(1000));
    assert_eq!(keys[1].tick, Tick::new(2000));
}

#[test]
fn tampered_receipt_is_detected() {
    let mut engine = MockMigrationEngine::default();
    let p = proposal(1, 2, 1000);
    let r = engine
        .apply(&p, Tick::new(1000), state('1'), state('2'))
        .unwrap();
    assert!(r.is_consistent());
    // Serialize, flip a field, confirm the binding no longer matches.
    let json = serde_json::to_string(&r).unwrap();
    let tampered = json.replace(&"2".repeat(64), &"9".repeat(64));
    let r2: MigrationReceipt = serde_json::from_str(&tampered).unwrap();
    assert!(!r2.is_consistent(), "tampered receipt must be inconsistent");
}

#[test]
fn executor_trait_dry_run_and_commit() {
    let engine = MockMigrationEngine::default();
    let p = proposal(1, 2, 1000);
    assert!(engine.dry_run(&p, &state('1')).is_ok());
    let r = engine.commit(&p, &state('1')).unwrap();
    assert!(r.is_consistent());
    assert_eq!(r.result(), MigrationResult::Committed);
}
