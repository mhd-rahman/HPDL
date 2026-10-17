# Actions and Events — Design

Refines `docs/design/10-actions-and-events.md`. Feature 10 has two halves, both now
implemented:

- **Definition-side** (`hpdl-model` + `hpdl-sema`): action **capability** declarations, a
  `ReadWriteSet` abstraction, and validation (field-path well-formedness + write→capability
  coverage). These are *declaration* concepts.
- **Runtime-side** (`hpdl-runtime-api`): the event/replay contracts (`Tick`, `ActionId`,
  `StateHash`, `EventEnvelope`) consumed by the external HORIZON engine. The crate was
  activated per **ADR 0004** (OD-10.1 option B). HPDL ships contracts, not an executor; the
  behaviors are exercised by a **mock executor** test double.

## 1. Crate placement

| Concern | Crate | Module |
|---|---|---|
| `Capability`, `ReadWriteSet` (declaration vocabulary) | `hpdl-model` | `capability.rs` |
| Capability field on actions | `hpdl-model` | `action_declaration.rs` (extended) |
| New model errors (`InvalidCapability`, `InvalidFieldPath`) | `hpdl-model` | `error.rs` |
| Capability / write-coverage validation | `hpdl-sema` | `action_validator.rs` (extended) |
| New diagnostic codes (E9300) | `hpdl-sema` | `diagnostic.rs` |
| `Tick`, `ActionId`, `StateHash`, `EventEnvelope`, `EventOrderKey` | `hpdl-runtime-api` | `ids.rs`, `event.rs` |
| Mock executor (test double) | `hpdl-runtime-api` | `tests/mock_executor.rs` |

**Dependency rationale (why capability vocab lives in `hpdl-model`):** `Capability` and
`ReadWriteSet` appear in *both* the authored declaration and the runtime event record. The
only crate both can share from is the dependency root `hpdl-model`; `hpdl-runtime-api`
re-exports them (`pub use hpdl_model::{Capability, ReadWriteSet}`). This avoids duplicate,
drift-prone parallel types (OD-10.3 / the "move down" decision).

## 2. Definition-side types (hpdl-model)

```text
Capability(String)                 // 1..=64, [A-Za-z0-9_\-.]
ReadWriteSet { reads: BTreeSet<String>, writes: BTreeSet<String>, capabilities: BTreeSet<Capability> }
  - validates every read/write field path (dotted, non-empty segments [A-Za-z0-9_\-])
  - uncovered_writes(): writes whose namespace is not covered by any capability namespace
ActionDeclaration (extended): + capabilities: BTreeSet<Capability>  (builder: with_capabilities)
  - read_write_set() -> Result<ReadWriteSet, Error>  (also validates field paths)
```

**Coverage rule:** a write `state.<domain>.<field>` is covered iff some declared capability's
namespace (its segments up to the trailing verb, e.g. `state.atmosphere.write` →
`state.atmosphere`) equals or prefixes the write's namespace (`state.atmosphere`). A broader
capability (`state.write` → `state`) covers nested writes. Status writes remain forbidden at
construction (`ActionDeclaration::new`).

## 3. Runtime-side contracts (hpdl-runtime-api)

```text
Tick(u64)                          // monotonic; checked next()
ActionId(String)                   // 1..=128, [A-Za-z0-9_\-.:]
StateHash(String)                  // 64-char lowercase hex (engine computes; crate validates shape)
EventOrderKey { tick, action_id }  // total ordering key
EventEnvelope { action_id, tick, module_name, module_version, input_state, output_state,
                read_write, binding }
  - binding = SHA-256(HPDL-EVENT-v1 || length-prefixed fields)  (dedicated event separator)
  - is_consistent(): recompute binding and compare (pure, no float, no panic)
  - is_replay_duplicate_of(other): same (action_id, tick, input_state)
```

## 4. Validation flow (hpdl-sema)

`Validator::validate_actions` now runs the existing `validate_actions` **plus**
`validate_capabilities`:
- assemble each action's `ReadWriteSet` → malformed field path yields `MalformedFieldPath`;
- each uncovered mutating write yields `UncoveredWrite`;
- status-write prohibition unchanged (rejected at construction, E4005/E5000 elsewhere).

## 5. The "required test cases" and where they live

The design's required cases are mostly *execution* behaviors. They are exercised against the
real contracts by the **mock executor** test double (`tests/mock_executor.rs`), never by HPDL
itself:

| Required case | Where tested | Kind |
|---|---|---|
| unauthorized action | mock executor (uncovered write rejected) + `hpdl-sema` `UncoveredWrite` | runtime (mock) + definition |
| replay duplicate | mock executor (`is_replay_duplicate_of`) | runtime (mock) |
| deterministic ordering | mock executor (sort by `EventOrderKey`) | runtime (mock) |
| cross-domain portal effects | mock executor (capability-gated magic-domain write) | runtime (mock) |
| event hash consistency | `EventEnvelope::is_consistent` unit + mock | contract |

## 6. Decisions

- **OD-10.1 → B (ADR 0004):** `hpdl-runtime-api` activated.
- **OD-10.2 → N/A:** capabilities are *not* lowered into canonical PIR this feature (the
  capability set is model/validation surface, not PIR payload), so no PIR bytes change and no
  golden-vector churn. Event/replay records are runtime artifacts (state-schema versioning),
  not PIR.
- **OD-10.3 → prefix/namespace coverage** rule documented above; registry-defined capabilities
  deferred to feature 14.
- **OD-10.4 → event domain separator** `HPDL-EVENT-v1` (distinct from PIR's). `StateHash` shape
  is 64-char hex; the engine owns the state-canonicalization + hashing.
- **OD-10.5 → signing deferred:** events bind hashes now; cryptographic signatures are a
  runtime/registry concern.

## 7. What this design deliberately does NOT do

- No executor; HPDL does not run actions or evolve state (the mock is a test double only).
- No cryptographic signing; no capability registry; no state canonicalization algorithm here.
- No PIR change; no new frontend syntax.
- No `f32`/`f64`; no panicking paths.
