//! Process and vitality **run contracts** (HPDL feature 12, domain 12).
//!
//! These are the stable shapes the external HORIZON engine uses to *run* the
//! modules a definition references:
//!
//! - [`ProcessContract`] — a versioned process module permitted to touch a
//!   declared [`ReadWriteSet`] from an activation [`Tick`] onward.
//! - [`EvaluatorContract`] — a versioned vitality evaluator the engine invokes to
//!   derive a [`crate::StatusResult`] for a [`StatusDimension`] against a
//!   [`BaselineRef`] over an [`EvaluationWindow`].
//! - [`ProcessReceipt`] — the deterministic, hash-bound record of one process run
//!   at one tick, with a dedicated `HPDL-PROCESS-v1` binding separator.
//!
//! HPDL ships **contracts, not an engine**. Nothing here evolves state, runs a
//! process, or derives a status; the crate's tests use a **mock executor** double.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::hashing::{feed, to_lower_hex};
use crate::{
    BaselineRef, EvaluationWindow, EvaluatorRef, ModuleRef, ProcessId, ReadWriteSet, StateHash,
    StatusDimension, Tick,
};

/// Domain separator for process-receipt binding hashes.
///
/// Distinct from the PIR separator (`HPDL-PIR-v2`) and the event separator
/// (`HPDL-EVENT-v1`) so a process receipt, an event binding, and a PIR content
/// hash can never collide across domains.
const PROCESS_DOMAIN_SEPARATOR: &[u8] = b"HPDL-PROCESS-v1";

/// A contract for a versioned process module a definition references.
///
/// A process is a module that evolves mutable state over ticks. The contract
/// declares *which* module, *what* it is permitted to touch (a [`ReadWriteSet`]
/// with capabilities), and *from when* it is live (an activation [`Tick`]). HPDL
/// declares the contract; the engine runs the process.
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::{Capability, ModuleRef, ProcessContract, ReadWriteSet, Tick};
///
/// let rws = ReadWriteSet::new(
///     ["state.atmosphere.co2"],
///     ["state.atmosphere.co2"],
///     [Capability::new("state.atmosphere.write").unwrap()],
/// ).unwrap();
/// let p = ProcessContract::new(
///     ModuleRef::new("hpdl.process.carbon-cycle", "1.0").unwrap(),
///     rws,
///     Tick::new(100),
/// );
/// assert!(p.permits_write("state.atmosphere.co2"));
/// assert!(!p.permits_write("state.hydrosphere.level"));
/// assert!(p.is_active_at(Tick::new(100)));   // inclusive
/// assert!(!p.is_active_at(Tick::new(99)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessContract {
    module: ModuleRef,
    read_write: ReadWriteSet,
    activation_tick: Tick,
}

impl ProcessContract {
    /// Construct a `ProcessContract`. The [`ModuleRef`] and [`ReadWriteSet`] are
    /// already validated by their own constructors.
    pub fn new(module: ModuleRef, read_write: ReadWriteSet, activation_tick: Tick) -> Self {
        Self {
            module,
            read_write,
            activation_tick,
        }
    }

    /// The versioned process module reference.
    pub fn module(&self) -> &ModuleRef {
        &self.module
    }

    /// The permitted read/write set.
    pub fn read_write(&self) -> &ReadWriteSet {
        &self.read_write
    }

    /// The activation tick (first live tick).
    pub fn activation_tick(&self) -> Tick {
        self.activation_tick
    }

    /// Whether a write to `path` is covered by the declared read/write set.
    ///
    /// A write that is not covered is an **unauthorized process effect** at the
    /// contract level — the runtime analogue of the definition-side uncovered
    /// write. Coverage reuses [`ReadWriteSet`]'s capability rule: the path must be
    /// a declared write *and* be backed by a declared capability.
    pub fn permits_write(&self, path: &str) -> bool {
        let declared = self.read_write.writes().any(|w| w == path);
        if !declared {
            return false;
        }
        // Covered iff it is not among the uncovered writes.
        !self.read_write.uncovered_writes().contains(&path)
    }

    /// Whether the process is live at `tick` (`tick >= activation_tick`,
    /// inclusive — see spec 12, OD-12.5).
    pub fn is_active_at(&self, tick: Tick) -> bool {
        tick.get() >= self.activation_tick.get()
    }
}

/// A contract for a versioned vitality evaluator the engine invokes.
///
/// Declares *which* evaluator derives a status, for which independent
/// [`StatusDimension`], against which [`BaselineRef`], over which
/// [`EvaluationWindow`]. It computes nothing — the engine produces the
/// [`crate::StatusResult`]; this is the stable input shape a result is bound to.
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::{
///     BaselineRef, EvaluationWindow, EvaluatorContract, EvaluatorRef, ModuleRef,
///     StatusDimension,
/// };
/// use hpdl_model::Digest;
///
/// let c = EvaluatorContract::new(
///     EvaluatorRef::new(ModuleRef::new("hpdl.vitality.eco", "1.0").unwrap()),
///     StatusDimension::NativeEcosystem,
///     BaselineRef::new("preindustrial", Digest::new("a".repeat(64)).unwrap()).unwrap(),
///     EvaluationWindow::new(1000, Some("surface".to_string())).unwrap(),
/// );
/// assert_eq!(c.dimension(), &StatusDimension::NativeEcosystem);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorContract {
    evaluator: EvaluatorRef,
    dimension: StatusDimension,
    baseline: BaselineRef,
    window: EvaluationWindow,
}

impl EvaluatorContract {
    /// Construct an `EvaluatorContract` from already-validated parts.
    pub fn new(
        evaluator: EvaluatorRef,
        dimension: StatusDimension,
        baseline: BaselineRef,
        window: EvaluationWindow,
    ) -> Self {
        Self {
            evaluator,
            dimension,
            baseline,
            window,
        }
    }

    /// The versioned evaluator reference.
    pub fn evaluator(&self) -> &EvaluatorRef {
        &self.evaluator
    }

    /// The dimension this evaluator measures.
    pub fn dimension(&self) -> &StatusDimension {
        &self.dimension
    }

    /// The baseline measured against.
    pub fn baseline(&self) -> &BaselineRef {
        &self.baseline
    }

    /// The evaluation window.
    pub fn window(&self) -> &EvaluationWindow {
        &self.window
    }
}

/// The total ordering key for process receipts: `(tick, process_id)`.
///
/// Deterministic and total — receipts order first by tick, then by process id,
/// never by hashmap iteration or wall-clock. Two receipts sharing a
/// `(tick, process_id)` are the same process run (see
/// [`ProcessReceipt::is_replay_duplicate_of`]).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProcessOrderKey {
    /// The tick the run occurred in.
    pub tick: Tick,
    /// The process-run instance id.
    pub process_id: ProcessId,
}

/// A deterministic, hash-bound record of one process run at one tick.
///
/// Binds the process instance, tick, acting module, input/output state hashes, and
/// the read/write set touched. The [`ProcessReceipt::binding`] is SHA-256 over the
/// canonical bytes of those fields under the dedicated `HPDL-PROCESS-v1`
/// separator, so a replaying engine can verify integrity.
///
/// This is a **contract**: the engine produces receipts; HPDL does not run
/// processes.
///
/// # Example
///
/// ```
/// use hpdl_runtime_api::{
///     Capability, ModuleRef, ProcessId, ProcessReceipt, ReadWriteSet, StateHash, Tick,
/// };
///
/// let rws = ReadWriteSet::new(
///     ["state.atmosphere.co2"],
///     ["state.atmosphere.co2"],
///     [Capability::new("state.atmosphere.write").unwrap()],
/// ).unwrap();
/// let r = ProcessReceipt::new(
///     ProcessId::new("carbon:tick-100").unwrap(),
///     Tick::new(100),
///     ModuleRef::new("hpdl.process.carbon-cycle", "1.0").unwrap(),
///     StateHash::from_hex("a".repeat(64)).unwrap(),
///     StateHash::from_hex("b".repeat(64)).unwrap(),
///     rws,
/// );
/// assert!(r.is_consistent());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessReceipt {
    process_id: ProcessId,
    tick: Tick,
    module: ModuleRef,
    input_state: StateHash,
    output_state: StateHash,
    read_write: ReadWriteSet,
    /// SHA-256 (hex) binding over the canonical bytes of all other fields.
    binding: StateHash,
}

impl ProcessReceipt {
    /// Construct a `ProcessReceipt`, computing its binding hash from the fields.
    pub fn new(
        process_id: ProcessId,
        tick: Tick,
        module: ModuleRef,
        input_state: StateHash,
        output_state: StateHash,
        read_write: ReadWriteSet,
    ) -> Self {
        let binding = Self::compute_binding(
            &process_id,
            tick,
            &module,
            &input_state,
            &output_state,
            &read_write,
        );
        Self {
            process_id,
            tick,
            module,
            input_state,
            output_state,
            read_write,
            binding,
        }
    }

    /// The total ordering key for this receipt, `(tick, process_id)`.
    pub fn order_key(&self) -> ProcessOrderKey {
        ProcessOrderKey {
            tick: self.tick,
            process_id: self.process_id.clone(),
        }
    }

    /// Whether the recorded binding matches a freshly-computed binding. A `false`
    /// result means the record was tampered with or mis-serialized. Pure, no
    /// floats, never panics.
    pub fn is_consistent(&self) -> bool {
        let recomputed = Self::compute_binding(
            &self.process_id,
            self.tick,
            &self.module,
            &self.input_state,
            &self.output_state,
            &self.read_write,
        );
        recomputed == self.binding
    }

    /// Whether this receipt is a replay duplicate of `other`: the same process
    /// instance at the same tick operating on the same input state. A replaying
    /// engine uses this to reject duplicates (idempotence).
    pub fn is_replay_duplicate_of(&self, other: &ProcessReceipt) -> bool {
        self.process_id == other.process_id
            && self.tick == other.tick
            && self.input_state == other.input_state
    }

    /// The process-run instance id.
    pub fn process_id(&self) -> &ProcessId {
        &self.process_id
    }

    /// The tick.
    pub fn tick(&self) -> Tick {
        self.tick
    }

    /// The acting process module.
    pub fn module(&self) -> &ModuleRef {
        &self.module
    }

    /// The input state hash.
    pub fn input_state(&self) -> &StateHash {
        &self.input_state
    }

    /// The output state hash.
    pub fn output_state(&self) -> &StateHash {
        &self.output_state
    }

    /// The read/write set the process touched.
    pub fn read_write(&self) -> &ReadWriteSet {
        &self.read_write
    }

    /// The binding hash (hex).
    pub fn binding(&self) -> &StateHash {
        &self.binding
    }

    /// Compute the deterministic binding hash over the canonical bytes of the
    /// bound fields, under the dedicated process domain separator. Field order and
    /// separators are fixed; collections iterate in their total (`BTreeSet`)
    /// order, so the result is stable.
    fn compute_binding(
        process_id: &ProcessId,
        tick: Tick,
        module: &ModuleRef,
        input_state: &StateHash,
        output_state: &StateHash,
        read_write: &ReadWriteSet,
    ) -> StateHash {
        let mut hasher = Sha256::new();
        hasher.update(PROCESS_DOMAIN_SEPARATOR);
        feed(&mut hasher, process_id.as_str().as_bytes());
        hasher.update(tick.get().to_be_bytes());
        feed(&mut hasher, module.name().as_bytes());
        feed(&mut hasher, module.version().as_bytes());
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
        // `hex` is always 64 lowercase hex chars; fall back to zero on the
        // impossible error path to avoid unwrap/expect per crate lints.
        StateHash::from_hex(hex).unwrap_or_else(|_| StateHash::zero())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::Capability;

    fn rws() -> ReadWriteSet {
        ReadWriteSet::new(
            ["state.atmosphere.co2"],
            ["state.atmosphere.co2"],
            [Capability::new("state.atmosphere.write").unwrap()],
        )
        .unwrap()
    }

    fn contract(activation: u64) -> ProcessContract {
        ProcessContract::new(
            ModuleRef::new("hpdl.process.carbon-cycle", "1.0").unwrap(),
            rws(),
            Tick::new(activation),
        )
    }

    // ----- ProcessContract -----

    #[test]
    fn permits_declared_covered_write() {
        let p = contract(0);
        assert!(p.permits_write("state.atmosphere.co2"));
    }

    #[test]
    fn rejects_undeclared_write() {
        let p = contract(0);
        assert!(!p.permits_write("state.hydrosphere.level"));
    }

    #[test]
    fn rejects_declared_but_uncovered_write() {
        // Declared write with no backing capability → not permitted.
        let rws = ReadWriteSet::new([] as [&str; 0], ["state.hydrosphere.level"], []).unwrap();
        let p = ProcessContract::new(
            ModuleRef::new("hpdl.process.hydro", "1.0").unwrap(),
            rws,
            Tick::new(0),
        );
        assert!(!p.permits_write("state.hydrosphere.level"));
    }

    #[test]
    fn activation_is_inclusive() {
        let p = contract(100);
        assert!(!p.is_active_at(Tick::new(99)));
        assert!(p.is_active_at(Tick::new(100)));
        assert!(p.is_active_at(Tick::new(101)));
    }

    // ----- ProcessReceipt -----

    fn receipt(tick: u64, id: &str, input: &str) -> ProcessReceipt {
        ProcessReceipt::new(
            ProcessId::new(id).unwrap(),
            Tick::new(tick),
            ModuleRef::new("hpdl.process.carbon-cycle", "1.0").unwrap(),
            StateHash::from_hex(input.repeat(64)).unwrap(),
            StateHash::from_hex("b".repeat(64)).unwrap(),
            rws(),
        )
    }

    #[test]
    fn binding_is_consistent() {
        assert!(receipt(1, "carbon:1", "a").is_consistent());
    }

    #[test]
    fn tampering_breaks_consistency() {
        let mut r = receipt(1, "carbon:1", "a");
        r.binding = StateHash::from_hex("f".repeat(64)).unwrap();
        assert!(!r.is_consistent());
    }

    #[test]
    fn same_fields_same_binding() {
        assert_eq!(
            receipt(5, "p", "c").binding(),
            receipt(5, "p", "c").binding()
        );
    }

    #[test]
    fn different_tick_different_binding() {
        assert_ne!(
            receipt(5, "p", "c").binding(),
            receipt(6, "p", "c").binding()
        );
    }

    #[test]
    fn replay_duplicate_detection() {
        let a = receipt(5, "p", "c");
        let dup = receipt(5, "p", "c");
        let not_dup = receipt(5, "p", "d"); // different input state
        assert!(a.is_replay_duplicate_of(&dup));
        assert!(!a.is_replay_duplicate_of(&not_dup));
    }

    #[test]
    fn order_key_is_total() {
        let e1 = receipt(1, "a", "c").order_key();
        let e2 = receipt(1, "b", "c").order_key();
        let e3 = receipt(2, "a", "c").order_key();
        assert!(e1 < e2); // same tick, id a < b
        assert!(e2 < e3); // tick 1 < tick 2
    }

    /// A process receipt and an event with the same identity surface must NOT
    /// share a binding — the domain separators differ (`HPDL-PROCESS-v1` vs
    /// `HPDL-EVENT-v1`).
    #[test]
    fn process_binding_differs_from_event_domain() {
        use crate::{ActionId, EventEnvelope};
        let r = receipt(1, "x", "a");
        let ev = EventEnvelope::new(
            ActionId::new("x").unwrap(),
            Tick::new(1),
            "hpdl.process.carbon-cycle",
            "1.0",
            StateHash::from_hex("a".repeat(64)).unwrap(),
            StateHash::from_hex("b".repeat(64)).unwrap(),
            rws(),
        );
        assert_ne!(r.binding(), ev.binding());
    }
}
