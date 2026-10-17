//! Mock-executor integration tests for the runtime contracts.
//!
//! IMPORTANT: `MockEngine` below is a **test double**, not a simulation and not
//! part of the shipped crate. HPDL does not execute state — the real executor is
//! the external HORIZON engine. This mock exists solely to exercise the published
//! `hpdl-runtime-api` *contracts* through the behaviors the feature-10 design
//! requires:
//!
//!   - unauthorized action (write not covered by a declared capability)
//!   - replay duplicate detection / idempotence
//!   - deterministic event ordering
//!   - cross-domain portal effects gated by a capability
//!   - event hash (binding) consistency
//!
//! The mock makes no claim of being a correct or complete engine; it only
//! demonstrates that the contract types support these behaviors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeSet;

use hpdl_runtime_api::{
    ActionId, Capability, EventEnvelope, EventOrderKey, ReadWriteSet, StateHash, Tick,
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

/// Errors the mock engine can reject an action with.
#[derive(Debug, PartialEq, Eq)]
enum MockReject {
    Unauthorized(String),
    ReplayDuplicate,
}

/// A deliberately minimal, in-memory **mock** of the external engine. Not shipped;
/// not a simulation.
#[derive(Default)]
struct MockEngine {
    /// The event log, kept sorted by the contract's total ordering key.
    log: Vec<EventEnvelope>,
    /// Seen (tick, action_id, input-hash) identities, for duplicate detection.
    seen: BTreeSet<(u64, String, String)>,
}

impl MockEngine {
    /// Attempt to "apply" an action: authorize it against its declared read/write
    /// set, reject replay duplicates, record a hash-bound event, and keep the log
    /// deterministically ordered. Returns the recorded event or a rejection.
    fn apply(
        &mut self,
        action_id: ActionId,
        tick: Tick,
        module: (&str, &str),
        input: StateHash,
        output: StateHash,
        rws: ReadWriteSet,
    ) -> Result<EventEnvelope, MockReject> {
        // 1. Authorization: every mutating write must be covered by a capability.
        let uncovered = rws.uncovered_writes();
        if !uncovered.is_empty() {
            return Err(MockReject::Unauthorized(uncovered.join(",")));
        }

        // 2. Replay-duplicate detection via the contract's identity.
        let identity = (
            tick.get(),
            action_id.as_str().to_string(),
            input.as_hex().to_string(),
        );
        if self.seen.contains(&identity) {
            return Err(MockReject::ReplayDuplicate);
        }

        // 3. Record a hash-bound event.
        let event = EventEnvelope::new(action_id, tick, module.0, module.1, input, output, rws);
        self.seen.insert(identity);
        self.log.push(event.clone());

        // 4. Keep the log deterministically ordered by the contract key.
        self.log.sort_by_key(EventEnvelope::order_key);

        Ok(event)
    }

    fn order_keys(&self) -> Vec<EventOrderKey> {
        self.log.iter().map(EventEnvelope::order_key).collect()
    }
}

fn atmosphere_write() -> ReadWriteSet {
    ReadWriteSet::new(
        ["state.temperature.mean"],
        ["state.atmosphere.co2"],
        [Capability::new("state.atmosphere.write").unwrap()],
    )
    .unwrap()
}

#[test]
fn unauthorized_action_is_rejected() {
    let mut engine = MockEngine::default();
    // Write to hydrosphere but only declare an atmosphere capability.
    let rws = ReadWriteSet::new(
        [] as [&str; 0],
        ["state.hydrosphere.level"],
        [Capability::new("state.atmosphere.write").unwrap()],
    )
    .unwrap();

    let result = engine.apply(
        ActionId::new("flood:1").unwrap(),
        Tick::new(1),
        ("hpdl.hydro", "1.0"),
        mock_state_hash(b"s0"),
        mock_state_hash(b"s1"),
        rws,
    );
    assert!(matches!(result, Err(MockReject::Unauthorized(_))));
    assert!(engine.log.is_empty(), "rejected action must not be logged");
}

#[test]
fn authorized_action_is_accepted_and_logged() {
    let mut engine = MockEngine::default();
    let ev = engine
        .apply(
            ActionId::new("terraform:1").unwrap(),
            Tick::new(1),
            ("hpdl.terraform", "1.0"),
            mock_state_hash(b"s0"),
            mock_state_hash(b"s1"),
            atmosphere_write(),
        )
        .expect("authorized action should be accepted");
    assert!(ev.is_consistent());
    assert_eq!(engine.log.len(), 1);
}

#[test]
fn replay_duplicate_is_rejected() {
    let mut engine = MockEngine::default();
    let apply = |engine: &mut MockEngine| {
        engine.apply(
            ActionId::new("terraform:1").unwrap(),
            Tick::new(1),
            ("hpdl.terraform", "1.0"),
            mock_state_hash(b"s0"),
            mock_state_hash(b"s1"),
            atmosphere_write(),
        )
    };
    assert!(apply(&mut engine).is_ok());
    // Replaying the exact same (action, tick, input) is a duplicate.
    assert_eq!(apply(&mut engine), Err(MockReject::ReplayDuplicate));
    assert_eq!(engine.log.len(), 1, "duplicate must not be logged twice");
}

#[test]
fn events_are_deterministically_ordered() {
    // Apply events out of order; the log must come out ordered by (tick, action_id).
    let mut engine = MockEngine::default();
    let specs = [(2, "b"), (1, "b"), (1, "a"), (2, "a")];
    for (i, (tick, id)) in specs.iter().enumerate() {
        engine
            .apply(
                ActionId::new(*id).unwrap(),
                Tick::new(*tick),
                ("hpdl.m", "1.0"),
                mock_state_hash(format!("in{i}").as_bytes()),
                mock_state_hash(format!("out{i}").as_bytes()),
                atmosphere_write(),
            )
            .unwrap();
    }

    let keys = engine.order_keys();
    let expected = vec![
        EventOrderKey {
            tick: Tick::new(1),
            action_id: ActionId::new("a").unwrap(),
        },
        EventOrderKey {
            tick: Tick::new(1),
            action_id: ActionId::new("b").unwrap(),
        },
        EventOrderKey {
            tick: Tick::new(2),
            action_id: ActionId::new("a").unwrap(),
        },
        EventOrderKey {
            tick: Tick::new(2),
            action_id: ActionId::new("b").unwrap(),
        },
    ];
    assert_eq!(keys, expected);
}

#[test]
fn cross_domain_portal_effect_requires_capability() {
    let mut engine = MockEngine::default();

    // A portal effect that writes a magic-domain field.
    let write_path = ["magic.ley.flux"];

    // Without the portal capability → unauthorized.
    let without = ReadWriteSet::new([] as [&str; 0], write_path, []).unwrap();
    assert!(matches!(
        engine.apply(
            ActionId::new("portal:1").unwrap(),
            Tick::new(1),
            ("arcane.portal", "1.0"),
            mock_state_hash(b"p0"),
            mock_state_hash(b"p1"),
            without,
        ),
        Err(MockReject::Unauthorized(_))
    ));

    // With a capability covering the magic.ley namespace → accepted.
    let with = ReadWriteSet::new(
        [] as [&str; 0],
        write_path,
        [Capability::new("magic.ley.write").unwrap()],
    )
    .unwrap();
    let ev = engine
        .apply(
            ActionId::new("portal:1").unwrap(),
            Tick::new(1),
            ("arcane.portal", "1.0"),
            mock_state_hash(b"p0"),
            mock_state_hash(b"p1"),
            with,
        )
        .expect("portal effect with capability should be accepted");
    assert!(ev.is_consistent());
}

#[test]
fn event_hash_consistency_detects_tampering() {
    let mut engine = MockEngine::default();
    let ev = engine
        .apply(
            ActionId::new("terraform:1").unwrap(),
            Tick::new(1),
            ("hpdl.terraform", "1.0"),
            mock_state_hash(b"s0"),
            mock_state_hash(b"s1"),
            atmosphere_write(),
        )
        .unwrap();

    // A faithfully-recorded event is self-consistent.
    assert!(ev.is_consistent());

    // A differently-bound event (same identity surface, different module version)
    // produces a different binding — the hash binds all fields, not just identity.
    let other = EventEnvelope::new(
        ev.action_id().clone(),
        ev.tick(),
        "hpdl.terraform",
        "2.0",
        ev.input_state().clone(),
        ev.output_state().clone(),
        atmosphere_write(),
    );
    assert_ne!(ev.binding(), other.binding());
}
