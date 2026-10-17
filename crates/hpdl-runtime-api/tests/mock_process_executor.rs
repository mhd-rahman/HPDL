//! Mock process-executor integration tests for the feature-12 run contracts.
//!
//! IMPORTANT: `MockProcessEngine` below is a **test double**, not a simulation and
//! not part of the shipped crate. HPDL does not execute state — the real executor
//! is the external HORIZON engine. This mock exists solely to exercise the
//! published process/vitality *contracts* through the behaviors the feature-12
//! design requires:
//!
//!   - unauthorized process effect (write outside the declared read/write set)
//!   - inactive-at-tick rejection (run before the activation tick)
//!   - deterministic receipt ordering
//!   - replay-duplicate detection / idempotence
//!   - receipt hash (binding) consistency
//!
//! The mock makes no claim of being a correct or complete engine; it only shows
//! that the contract types support these behaviors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeSet;

use hpdl_runtime_api::{
    Capability, Error, ModuleRef, ProcessContract, ProcessId, ProcessOrderKey, ProcessReceipt,
    ReadWriteSet, StateHash, Tick,
};
use sha2::{Digest, Sha256};

/// Hash arbitrary mock "state bytes" into a `StateHash` (test helper only).
#[allow(clippy::format_collect)]
fn mock_state_hash(bytes: &[u8]) -> StateHash {
    let mut h = Sha256::new();
    h.update(b"MOCK-STATE");
    h.update(bytes);
    let hex: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    StateHash::from_hex(hex).unwrap()
}

/// A deliberately minimal, in-memory **mock** of the external engine. Not shipped;
/// not a simulation.
#[derive(Default)]
struct MockProcessEngine {
    log: Vec<ProcessReceipt>,
    seen: BTreeSet<(u64, String, String)>,
}

impl MockProcessEngine {
    /// Attempt to "run" a process at a tick: check activation, authorize each
    /// effect against the declared read/write set, reject replay duplicates,
    /// record a hash-bound receipt, and keep the log deterministically ordered.
    fn run(
        &mut self,
        contract: &ProcessContract,
        process_id: ProcessId,
        tick: Tick,
        input: StateHash,
        output: StateHash,
        effects: ReadWriteSet,
    ) -> Result<ProcessReceipt, Error> {
        // 1. Activation: the process must be live at this tick.
        if !contract.is_active_at(tick) {
            return Err(Error::ProcessInactiveAtTick {
                requested: tick.get(),
                activation: contract.activation_tick().get(),
            });
        }

        // 2. Authorization: every effect write must be permitted by the contract.
        for w in effects.writes() {
            if !contract.permits_write(w) {
                return Err(Error::UnauthorizedProcessEffect(w.to_string()));
            }
        }

        // 3. Replay-duplicate detection via the contract's identity.
        let identity = (
            tick.get(),
            process_id.as_str().to_string(),
            input.as_hex().to_string(),
        );
        if self.seen.contains(&identity) {
            // Already recorded — idempotent: return the existing receipt.
            if let Some(existing) = self.log.iter().find(|r| {
                r.is_replay_duplicate_of(&ProcessReceipt::new(
                    process_id.clone(),
                    tick,
                    contract.module().clone(),
                    input.clone(),
                    output.clone(),
                    effects.clone(),
                ))
            }) {
                return Ok(existing.clone());
            }
        }

        // 4. Record a hash-bound receipt.
        let receipt = ProcessReceipt::new(
            process_id,
            tick,
            contract.module().clone(),
            input,
            output,
            effects,
        );
        self.seen.insert(identity);
        self.log.push(receipt.clone());

        // 5. Keep the log deterministically ordered by the contract key.
        self.log.sort_by_key(ProcessReceipt::order_key);

        Ok(receipt)
    }

    fn order_keys(&self) -> Vec<ProcessOrderKey> {
        self.log.iter().map(ProcessReceipt::order_key).collect()
    }
}

fn atmosphere_contract(activation: u64) -> ProcessContract {
    let rws = ReadWriteSet::new(
        ["state.atmosphere.co2"],
        ["state.atmosphere.co2"],
        [Capability::new("state.atmosphere.write").unwrap()],
    )
    .unwrap();
    ProcessContract::new(
        ModuleRef::new("hpdl.process.carbon-cycle", "1.0").unwrap(),
        rws,
        Tick::new(activation),
    )
}

fn atmosphere_effect() -> ReadWriteSet {
    ReadWriteSet::new(
        ["state.atmosphere.co2"],
        ["state.atmosphere.co2"],
        [Capability::new("state.atmosphere.write").unwrap()],
    )
    .unwrap()
}

#[test]
fn inactive_process_is_rejected() {
    let mut engine = MockProcessEngine::default();
    let contract = atmosphere_contract(100);
    let result = engine.run(
        &contract,
        ProcessId::new("carbon:50").unwrap(),
        Tick::new(50), // before activation
        mock_state_hash(b"s0"),
        mock_state_hash(b"s1"),
        atmosphere_effect(),
    );
    assert!(matches!(result, Err(Error::ProcessInactiveAtTick { .. })));
    assert!(engine.log.is_empty());
}

#[test]
fn unauthorized_process_effect_is_rejected() {
    let mut engine = MockProcessEngine::default();
    let contract = atmosphere_contract(0);
    // Effect writes a hydrosphere field the contract does not permit.
    let effect = ReadWriteSet::new(
        [] as [&str; 0],
        ["state.hydrosphere.level"],
        [Capability::new("state.hydrosphere.write").unwrap()],
    )
    .unwrap();
    let result = engine.run(
        &contract,
        ProcessId::new("carbon:1").unwrap(),
        Tick::new(1),
        mock_state_hash(b"s0"),
        mock_state_hash(b"s1"),
        effect,
    );
    assert!(matches!(result, Err(Error::UnauthorizedProcessEffect(_))));
    assert!(engine.log.is_empty(), "rejected run must not be logged");
}

#[test]
fn authorized_active_run_is_accepted_and_logged() {
    let mut engine = MockProcessEngine::default();
    let contract = atmosphere_contract(0);
    let r = engine
        .run(
            &contract,
            ProcessId::new("carbon:1").unwrap(),
            Tick::new(1),
            mock_state_hash(b"s0"),
            mock_state_hash(b"s1"),
            atmosphere_effect(),
        )
        .expect("authorized active run should be accepted");
    assert!(r.is_consistent());
    assert_eq!(engine.log.len(), 1);
}

#[test]
fn replay_duplicate_is_idempotent() {
    let mut engine = MockProcessEngine::default();
    let contract = atmosphere_contract(0);
    let run = |engine: &mut MockProcessEngine| {
        engine.run(
            &contract,
            ProcessId::new("carbon:1").unwrap(),
            Tick::new(1),
            mock_state_hash(b"s0"),
            mock_state_hash(b"s1"),
            atmosphere_effect(),
        )
    };
    let first = run(&mut engine).unwrap();
    let again = run(&mut engine).unwrap();
    assert_eq!(first, again, "replay returns the same receipt");
    assert_eq!(engine.log.len(), 1, "duplicate must not be logged twice");
}

#[test]
fn receipts_are_deterministically_ordered() {
    let mut engine = MockProcessEngine::default();
    let contract = atmosphere_contract(0);
    let specs = [(2, "b"), (1, "b"), (1, "a"), (2, "a")];
    for (i, (tick, id)) in specs.iter().enumerate() {
        engine
            .run(
                &contract,
                ProcessId::new(*id).unwrap(),
                Tick::new(*tick),
                mock_state_hash(format!("in{i}").as_bytes()),
                mock_state_hash(format!("out{i}").as_bytes()),
                atmosphere_effect(),
            )
            .unwrap();
    }
    let keys = engine.order_keys();
    let expected = vec![
        ProcessOrderKey {
            tick: Tick::new(1),
            process_id: ProcessId::new("a").unwrap(),
        },
        ProcessOrderKey {
            tick: Tick::new(1),
            process_id: ProcessId::new("b").unwrap(),
        },
        ProcessOrderKey {
            tick: Tick::new(2),
            process_id: ProcessId::new("a").unwrap(),
        },
        ProcessOrderKey {
            tick: Tick::new(2),
            process_id: ProcessId::new("b").unwrap(),
        },
    ];
    assert_eq!(keys, expected);
}

#[test]
fn receipt_hash_consistency_detects_tampering() {
    let mut engine = MockProcessEngine::default();
    let contract = atmosphere_contract(0);
    let r = engine
        .run(
            &contract,
            ProcessId::new("carbon:1").unwrap(),
            Tick::new(1),
            mock_state_hash(b"s0"),
            mock_state_hash(b"s1"),
            atmosphere_effect(),
        )
        .unwrap();
    assert!(r.is_consistent());

    // A differently-bound receipt (different output state) has a different binding.
    let other = ProcessReceipt::new(
        r.process_id().clone(),
        r.tick(),
        r.module().clone(),
        r.input_state().clone(),
        mock_state_hash(b"different-output"),
        atmosphere_effect(),
    );
    assert_ne!(r.binding(), other.binding());
}
