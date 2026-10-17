# Conformance & Security — Tasks

> Consolidation / gap-closing feature. Most of the conformance/security surface already
> exists (CI matrix, digest parity, cargo-deny/audit, semver, MSRV, SECURITY.md, PIR
> fuzz, malformed-input tests) — those are **referenced, not rebuilt**. These tasks close
> the genuine gaps. OD-16.1…OD-16.5 proposed in requirements; OD-06.2 remains the blocker
> for cross-quantum conformance and is surfaced via ADR 0010, not resolved in code.
>
> ⚠️ Status: specification only. Tasks are NOT implemented yet — do not check them off
> until the work is actually done and verified on the pinned toolchain.

- [ ] 1. REQ-01 — Frozen exact PIR digest vectors
  - Add `crates/hpdl-pir/tests/golden/pir_vectors.json` (name, language_version, body_id,
    pir_version, exact content_hash_hex) for a fixed fixture set. Extend `golden_hash.rs`
    to assert encoded digest + version equal the frozen values. Document no-silent-
    regeneration. Capture digests once from a verified pinned-toolchain run before freezing.
  - **Files:** `crates/hpdl-pir/tests/golden/pir_vectors.json`, `crates/hpdl-pir/tests/golden_hash.rs`.

- [ ] 2. REQ-02 — Contract-level replay vectors
  - `crates/hpdl-runtime-api/tests/replay_vectors.rs` + `tests/golden/replay_vectors.json`:
    frozen binding hashes + order keys for `EventEnvelope`, `ProcessReceipt`,
    `MigrationReceipt` from fixed inputs; assert exact binding + `is_consistent()` + sorted
    order. Label contract-level (engine external).
  - **Files:** `crates/hpdl-runtime-api/tests/replay_vectors.rs`, `.../tests/golden/replay_vectors.json`.

- [ ] 3. REQ-03 — RON frontend fuzz target
  - `fuzz/fuzz_targets/decode_ron.rs` over the `hpdl-ron` loader (totality only); add
    `[[bin]]` + `hpdl-ron` dep to `fuzz/Cargo.toml`; add `fuzz/seeds/valid.ron`; run both
    targets in `fuzz.yml`. Note `hpdl-syntax` fuzzing deferred (OD-16.3) in `fuzz/README.md`.
  - **Files:** `fuzz/fuzz_targets/decode_ron.rs`, `fuzz/Cargo.toml`, `fuzz/seeds/valid.ron`,
    `.github/workflows/fuzz.yml`, `fuzz/README.md`.

- [ ] 4. REQ-04 — Property-based suite (proptest, stable)
  - Add `proptest` dev-dep to `hpdl-pir`; `crates/hpdl-pir/tests/property.rs` with
    decode-never-panics, encode→decode round-trip, deterministic-encode; fixed
    `ProptestConfig`. Runs in `cargo test` on all 3 OSes (no nightly).
  - **Files:** `crates/hpdl-pir/Cargo.toml`, `crates/hpdl-pir/tests/property.rs`.

- [ ] 5. REQ-05 — Broaden cross-platform digest parity
  - Extend `emit_digests.rs` with environment/status/orbital/world-law fixtures (fixed
    order) and append the REQ-02 contract binding hashes so replay hashes are compared
    across OSes. Confirm `digest-parity` CI stays green on all 3 OSes.
  - **Files:** `crates/hpdl-compiler/examples/emit_digests.rs`.

- [ ] 6. REQ-06 — Reconcile security surface + MSRV consistency
  - Expand `SECURITY.md` in-scope list to all 8 active crates + untrusted-input surfaces.
    Verify every crate `rust-version = "1.85.0"`; optionally add a CI grep step.
  - **Files:** `SECURITY.md`, (optional) `.github/workflows/ci.yml`, crate `Cargo.toml`s if drifted.

- [ ] 7. REQ-07 — ADR 0010: cross-quantum conformance deferred (OD-06.2)
  - Record that cross-quantum / `ties_to_even` conformance is blocked on the fixed-point
    lowering contract (OD-06.2); no placeholder tests. Record OD-16.1…OD-16.5.
  - **Files:** `docs/rfcs/0010-conformance-security.md`, `docs/rfcs/README.md`, `CHANGELOG.md`.

- [ ] 8. Verify
  - `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
    `cargo test --workspace` (incl. new golden/replay/property tests); `cargo deny check`;
    `cargo semver-checks -p hpdl-pir -p hpdl-runtime-api`; fuzz targets build on nightly
    (gnu target). Confirm CI digest-parity green across ubuntu/macos/windows.

## Out of scope (explicit)
- Rebuilding existing CI jobs, `deny.toml`, `SECURITY.md` structure, or malformed-input
  tests (referenced only).
- The authoritative replay engine (external HORIZON); only contract-level vectors here.
- Resolving OD-06.2 or writing cross-quantum arithmetic tests (deferred, surfaced).
- Fuzzing the native `.hpdl` parser (OD-16.3 deferred).
- Any PIR / canonical-bytes change.
