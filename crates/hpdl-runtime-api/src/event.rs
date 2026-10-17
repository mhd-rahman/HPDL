//! [`EventEnvelope`] — a deterministic, hash-bound replay record, and
//! [`EventOrderKey`] — the total ordering key for events.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::hashing::{feed, to_lower_hex};
use crate::{ActionId, ReadWriteSet, StateHash, Tick};

/// Domain separator for event binding hashes.
///
/// Distinct from the PIR content-hash separator (`HPDL-PIR-v2`) so an event
/// binding and a PIR content hash can never collide across domains.
const EVENT_DOMAIN_SEPARATOR: &[u8] = b"HPDL-EVENT-v1";

/// The total ordering key for events: `(tick, action_id)`.
///
/// Ordering is deterministic and total — events are ordered first by tick, then
/// by action id (both of which have a total `Ord`). This never depends on
/// hashmap iteration or wall-clock time. If two events share a `(tick, action_id)`
/// they are, by definition, the *same* action instance (see
/// [`EventEnvelope::is_replay_duplicate_of`]).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventOrderKey {
    /// The tick the event occurred in.
    pub tick: Tick,
    /// The action instance id.
    pub action_id: ActionId,
}

/// A signed-shaped replay record binding one action's effect at one tick.
///
/// An `EventEnvelope` binds: the action instance ([`ActionId`]), the [`Tick`], the
/// acting runtime module name + version, the input and output [`StateHash`]es, and
/// the [`ReadWriteSet`] the action touched. The [`EventEnvelope::binding`] is a
/// SHA-256 over the canonical bytes of all those fields (with a dedicated event
/// domain separator), so a replaying engine can verify integrity.
///
/// This is a **contract**: the engine produces events; this type gives them a
/// deterministic, verifiable shape. HPDL does not produce events itself.
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::{ActionId, Capability, EventEnvelope, ReadWriteSet, StateHash, Tick};
///
/// let rws = ReadWriteSet::new(
///     ["state.temperature.mean"],
///     ["state.atmosphere.co2"],
///     [Capability::new("state.atmosphere.write").unwrap()],
/// ).unwrap();
///
/// let ev = EventEnvelope::new(
///     ActionId::new("terraform:step-1").unwrap(),
///     Tick::new(7),
///     "hpdl.terraform",
///     "1.0",
///     StateHash::from_hex("a".repeat(64)).unwrap(),
///     StateHash::from_hex("b".repeat(64)).unwrap(),
///     rws,
/// );
///
/// assert!(ev.is_consistent()); // binding matches the bound fields
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    action_id: ActionId,
    tick: Tick,
    module_name: String,
    module_version: String,
    input_state: StateHash,
    output_state: StateHash,
    read_write: ReadWriteSet,
    /// SHA-256 (hex) binding over the canonical bytes of all other fields.
    binding: StateHash,
}

impl EventEnvelope {
    /// Construct an `EventEnvelope`, computing its binding hash from the fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        action_id: ActionId,
        tick: Tick,
        module_name: impl Into<String>,
        module_version: impl Into<String>,
        input_state: StateHash,
        output_state: StateHash,
        read_write: ReadWriteSet,
    ) -> Self {
        let module_name = module_name.into();
        let module_version = module_version.into();
        let binding = Self::compute_binding(
            &action_id,
            tick,
            &module_name,
            &module_version,
            &input_state,
            &output_state,
            &read_write,
        );
        Self {
            action_id,
            tick,
            module_name,
            module_version,
            input_state,
            output_state,
            read_write,
            binding,
        }
    }

    /// The total ordering key for this event, `(tick, action_id)`.
    pub fn order_key(&self) -> EventOrderKey {
        EventOrderKey {
            tick: self.tick,
            action_id: self.action_id.clone(),
        }
    }

    /// Whether the recorded binding matches a freshly-computed binding over the
    /// bound fields. A `false` result means the record was tampered with or
    /// mis-serialized. This is a pure, no-float check and never panics.
    pub fn is_consistent(&self) -> bool {
        let recomputed = Self::compute_binding(
            &self.action_id,
            self.tick,
            &self.module_name,
            &self.module_version,
            &self.input_state,
            &self.output_state,
            &self.read_write,
        );
        recomputed == self.binding
    }

    /// Whether this event is a replay duplicate of `other`: the same action
    /// instance at the same tick operating on the same input state. A replaying
    /// engine uses this to detect and reject duplicates (idempotence).
    pub fn is_replay_duplicate_of(&self, other: &EventEnvelope) -> bool {
        self.action_id == other.action_id
            && self.tick == other.tick
            && self.input_state == other.input_state
    }

    /// The action instance id.
    pub fn action_id(&self) -> &ActionId {
        &self.action_id
    }

    /// The tick.
    pub fn tick(&self) -> Tick {
        self.tick
    }

    /// The acting runtime module name.
    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    /// The acting runtime module version.
    pub fn module_version(&self) -> &str {
        &self.module_version
    }

    /// The input state hash.
    pub fn input_state(&self) -> &StateHash {
        &self.input_state
    }

    /// The output state hash.
    pub fn output_state(&self) -> &StateHash {
        &self.output_state
    }

    /// The read/write set the action touched.
    pub fn read_write(&self) -> &ReadWriteSet {
        &self.read_write
    }

    /// The binding hash (hex).
    pub fn binding(&self) -> &StateHash {
        &self.binding
    }

    /// Compute the deterministic binding hash over the canonical bytes of the
    /// bound fields. Field order and separators are fixed; all collections are
    /// iterated in their total (`BTreeSet`) order, so the result is stable.
    #[allow(clippy::too_many_arguments)]
    fn compute_binding(
        action_id: &ActionId,
        tick: Tick,
        module_name: &str,
        module_version: &str,
        input_state: &StateHash,
        output_state: &StateHash,
        read_write: &ReadWriteSet,
    ) -> StateHash {
        let mut hasher = Sha256::new();
        hasher.update(EVENT_DOMAIN_SEPARATOR);
        // Length-prefix each field to avoid ambiguity across concatenations.
        feed(&mut hasher, action_id.as_str().as_bytes());
        hasher.update(tick.get().to_be_bytes());
        feed(&mut hasher, module_name.as_bytes());
        feed(&mut hasher, module_version.as_bytes());
        feed(&mut hasher, input_state.as_hex().as_bytes());
        feed(&mut hasher, output_state.as_hex().as_bytes());
        for r in read_write.reads() {
            feed(&mut hasher, b"r");
            feed(&mut hasher, r.as_bytes());
        }
        for w in read_write.writes() {
            feed(&mut hasher, b"w");
            feed(&mut hasher, w.as_bytes());
        }
        for c in read_write.capabilities() {
            feed(&mut hasher, b"c");
            feed(&mut hasher, c.as_str().as_bytes());
        }
        let digest = hasher.finalize();
        let hex = to_lower_hex(&digest);
        // `hex` is always 64 lowercase hex chars, so this never fails; but we avoid
        // unwrap/expect per crate lints by falling back to a zero hash on the
        // impossible error path.
        StateHash::from_hex(hex).unwrap_or_else(|_| StateHash::zero())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::Capability;

    fn sample_rws() -> ReadWriteSet {
        ReadWriteSet::new(
            ["state.temperature.mean"],
            ["state.atmosphere.co2"],
            [Capability::new("state.atmosphere.write").unwrap()],
        )
        .unwrap()
    }

    fn sample_event(tick: u64, id: &str, input: &str) -> EventEnvelope {
        EventEnvelope::new(
            ActionId::new(id).unwrap(),
            Tick::new(tick),
            "hpdl.terraform",
            "1.0",
            StateHash::from_hex(input.repeat(64)).unwrap(),
            StateHash::from_hex("b".repeat(64)).unwrap(),
            sample_rws(),
        )
    }

    #[test]
    fn binding_is_consistent() {
        let ev = sample_event(1, "terraform:step-1", "a");
        assert!(ev.is_consistent());
    }

    #[test]
    fn tampering_breaks_consistency() {
        let mut ev = sample_event(1, "terraform:step-1", "a");
        // Corrupt the recorded binding.
        ev.binding = StateHash::from_hex("f".repeat(64)).unwrap();
        assert!(!ev.is_consistent());
    }

    #[test]
    fn same_fields_same_binding() {
        let a = sample_event(5, "act", "c");
        let b = sample_event(5, "act", "c");
        assert_eq!(a.binding(), b.binding());
        assert_eq!(a, b);
    }

    #[test]
    fn different_fields_different_binding() {
        let a = sample_event(5, "act", "c");
        let b = sample_event(6, "act", "c"); // different tick
        assert_ne!(a.binding(), b.binding());
    }

    #[test]
    fn replay_duplicate_detection() {
        let a = sample_event(5, "act", "c");
        let dup = sample_event(5, "act", "c"); // same id, tick, input
        let not_dup = sample_event(5, "act", "d"); // different input state
        assert!(a.is_replay_duplicate_of(&dup));
        assert!(!a.is_replay_duplicate_of(&not_dup));
    }

    #[test]
    fn order_key_is_total() {
        let e1 = sample_event(1, "a", "c").order_key();
        let e2 = sample_event(1, "b", "c").order_key();
        let e3 = sample_event(2, "a", "c").order_key();
        assert!(e1 < e2); // same tick, id a < b
        assert!(e2 < e3); // tick 1 < tick 2
    }
}
