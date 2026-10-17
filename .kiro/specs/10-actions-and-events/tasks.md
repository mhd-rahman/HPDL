# Actions and Events — Tasks

> Both halves implemented. OD-10.1 resolved as **option B** (ADR 0004): `hpdl-runtime-api`
> activated. OD-10.2 is **N/A** (capabilities not lowered into PIR this feature). Capability
> vocabulary lives in `hpdl-model` and is re-exported from `hpdl-runtime-api` (the "move down"
> decision). HPDL ships contracts, not an executor — runtime behaviors are exercised by a
> mock executor test double.

## Task 1: Capability + ReadWriteSet in hpdl-model
**Status:** completed — `crates/hpdl-model/src/capability.rs`: `Capability` (validated token)
and `ReadWriteSet` (ordered reads/writes/capabilities + `uncovered_writes` coverage rule) with
field-path validation; new `Error::{InvalidCapability, InvalidFieldPath}`; re-exported from the
crate root. Unit tests for validation, ordering, and coverage.

**Files:** `crates/hpdl-model/src/capability.rs`, `error.rs`, `lib.rs`.

---

## Task 2: Extend ActionDeclaration with capabilities
**Status:** completed — optional `capabilities: BTreeSet<Capability>` via `with_capabilities`
builder; `capabilities()` accessor; `read_write_set()` assembles a `ReadWriteSet` (and validates
field paths). `ActionDeclaration::new(name, reads, writes)` unchanged; status-write prohibition
preserved.

**Files:** `crates/hpdl-model/src/action_declaration.rs`.

---

## Task 3: Capability diagnostics + validation (hpdl-sema)
**Status:** completed — `ActionValidator::validate_capabilities` wired into the single
`Validator` (via `validate_actions`): malformed field path → `MalformedFieldPath` (E9301),
uncovered mutating write → `UncoveredWrite` (E9302). New stable codes added and tested; the
status-write prohibition is unchanged. Pre-existing fixtures/tests updated to declare covering
capabilities (sema integration test; `earth_like.ron`).

**Files:** `crates/hpdl-sema/src/action_validator.rs`, `diagnostic.rs`, `validator.rs`;
`crates/hpdl-sema/tests/integration_tests.rs`; `crates/hpdl-ron/tests/fixtures/earth_like.ron`.

---

## Task 4: Activate hpdl-runtime-api (OD-10.1 B)
**Status:** completed — new published contract crate scaffolded (ADR 0004); workspace
`Cargo.toml` member uncommented; workflow steering status `deferred` → `active (feature 10)`;
`semver-checks` line updated. Depends only on `hpdl-pir` + `hpdl-model` (+ `serde`, `sha2`,
`thiserror`); strict lints.

**Files:** `crates/hpdl-runtime-api/Cargo.toml`, `src/lib.rs`; `Cargo.toml`;
`.kiro/steering/workflow.md`; `docs/rfcs/0004-activate-hpdl-runtime-api.md`.

---

## Task 5: Runtime contract types
**Status:** completed — `Tick`, `ActionId`, `StateHash` (`ids.rs`); `EventEnvelope` +
`EventOrderKey` with a dedicated `HPDL-EVENT-v1` domain separator, pure `is_consistent()`,
`is_replay_duplicate_of()`, and total ordering (`event.rs`); `Capability`/`ReadWriteSet`
re-exported from `hpdl-model`. Unit tests for ids, binding consistency, duplicate identity,
ordering.

**Files:** `crates/hpdl-runtime-api/src/{ids.rs,event.rs,error.rs,lib.rs}`.

---

## Task 6: Mock-executor tests (required behaviors)
**Status:** completed — `tests/mock_executor.rs` is a clearly-labelled **test double** standing
in for the HORIZON engine; exercises unauthorized-action rejection, replay-duplicate detection,
deterministic ordering, cross-domain portal gating, and event-hash consistency against the real
contracts. The crate ships no executor.

**Files:** `crates/hpdl-runtime-api/tests/mock_executor.rs`.

---

## Task 7: Docs + CHANGELOG
**Status:** completed — rustdoc on all new public items; `DIAGNOSTIC_CODES.md` E9300 block;
`CONVENTIONS.md` note on the `Capability("...")` RON newtype form; CHANGELOG entries for the
runtime-api activation and the definition-side capability surface.

**Files:** `crates/hpdl-sema/DIAGNOSTIC_CODES.md`, `crates/hpdl-ron/CONVENTIONS.md`,
`CHANGELOG.md`, source rustdoc.

---

## Deferred / out of scope (explicit, not silently skipped)

- **Execution engine.** HPDL does not run actions or evolve state; only the mock test double
  does. The real executor is the external HORIZON engine.
- **Cryptographic signing** of events (OD-10.5) — events bind hashes; signatures deferred to
  runtime/registry.
- **Capability registry / trust roots** (OD-10.3) — deferred to feature 14; declared
  capabilities use the documented prefix coverage rule.
- **State canonicalization + hashing algorithm** — the engine owns it; `StateHash` validates
  shape only.
- **Native `.hpdl` / RON syntax polish** for capabilities beyond the serde shape (features
  05 / 04).

## Definition of Done

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test` for model / sema / ron / runtime-api (per-crate; full `--workspace`
      recommended in CI/WSL)
- [ ] `cargo deny check` — run in CI (tool not installed in dev env)
- [ ] `cargo semver-checks -p hpdl-pir -p hpdl-runtime-api` — run in CI (new crate has no
      baseline yet)
- [x] No `unwrap`/`expect`/panic on input paths; checked arithmetic; no floats.
- [x] No action executed by HPDL; statuses/scores never authored; out-of-range never clamped.
- [x] OD-10.1 (ADR 0004) and OD-10.2 (N/A) recorded.
