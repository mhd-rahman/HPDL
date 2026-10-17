# 0001 — Workspace Foundation — Open Decisions

**Status:** Pending  
**Date:** 2026-10-14  
**Feature:** `00-workspace-foundation`

## Context

The Workspace Foundation feature (TASK-03, TASK-05, TASK-07) cannot be fully
completed without resolving five configuration decisions. These are recorded
here so they can be tracked, approved, and superseded by a follow-up ADR.

---

## Open Decisions

### OD-01 — Pinned toolchain version

**Question:** Which exact stable Rust release should be pinned in `rust-toolchain.toml`?  
**Default used:** `1.81.0`  
**Status:** ⏳ Pending approval  
**Impact:** All CI matrix legs; MSRV must be ≤ this version.  
**Owner:** —

---

### OD-02 — MSRV

**Question:** What is the Minimum Supported Rust Version recorded in each crate's `package.rust-version`?  
**Default used:** `1.80`  
**Status:** ⏳ Pending approval  
**Impact:** Sets the lower bound for downstream consumers of published crates; affects CI matrix.  
**Owner:** —

---

### OD-03 — CI provider

**Question:** Is GitHub Actions the confirmed CI provider?  
**Default used:** GitHub Actions (`.github/workflows/ci.yml`)  
**Status:** ⏳ Pending approval  
**Impact:** Workflow file location and action references (`dtolnay/rust-toolchain`, `EmbarkStudios/cargo-deny-action`, etc.).  
**Owner:** —

---

### OD-04 — `cargo semver-checks` failure policy

**Question:** Should `cargo semver-checks` fail on warnings as well as errors, or errors only?  
**Default used:** Default (errors only — `cargo semver-checks` exits non-zero on breaking changes)  
**Status:** ⏳ Pending approval  
**Impact:** CI gating strictness for `hpdl-pir` API changes.  
**Owner:** —

---

### OD-05 — License allowlist

**Question:** Which SPDX license identifiers are permitted in the dependency graph?  
**Default used:** `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `Unicode-DFS-2016`  
**Status:** ⏳ Pending approval  
**Impact:** `deny.toml` allowlist; any crate outside this list requires an explicit exception with justification.  
**Owner:** —

---

## Resolution process

When each decision is approved, update the entry above to `✅ Approved — <value>`,
record the effective date, and update the corresponding file
(`rust-toolchain.toml`, `Cargo.toml`, `.github/workflows/ci.yml`, `deny.toml`).
Once all five are resolved, update this ADR status to `Accepted`.
