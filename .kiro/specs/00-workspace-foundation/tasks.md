# Tasks — Workspace Foundation

> **Feature:** `00-workspace-foundation`  
> **Requirements:** `.kiro/specs/00-workspace-foundation/requirements.md`  
> **Design doc:** `docs/design/00-workspace-foundation.md`  
> **Status:** Complete — local gate verified (build, clippy, fmt, test, doc). `cargo deny`,
> `cargo semver-checks`, and multi-OS CI remain to be run in a CI environment with those
> tools installed and the repo under git.

---

## Implementation Plan

Tasks are ordered by dependency. Complete each task fully (including its verification steps) before starting the next. Mark tasks `[x]` as they are completed.

---

### TASK-01 — Create the Cargo workspace root manifest

**Requirement(s):** REQ-01  
**Effort:** Small

**What to do:**
- Create `Cargo.toml` at the repository root as a workspace manifest.
- Declare workspace members: `hpdl-model`, `hpdl-sema`, `hpdl-ron`, `hpdl-pir`, `hpdl-compiler`, `hpdl-cli`.
- Define shared `[profile.dev]` and `[profile.release]` sections once at workspace level.
- Set `resolver = "2"`.

**Verification:**
- [x] `cargo build --workspace` exits 0 on Windows (verified). Linux/macOS pending CI.
- [x] No member crate duplicates profile or patch configuration from the root.
- [x] Deferred crates are absent from the manifest.

---

### TASK-02 — Scaffold each active crate with minimal skeleton

**Requirement(s):** REQ-01, REQ-08, REQ-09  
**Effort:** Medium  
**Depends on:** TASK-01

**What to do:**
For each of `hpdl-model`, `hpdl-sema`, `hpdl-ron`, `hpdl-pir`, `hpdl-compiler`, `hpdl-cli`:
- Create `<crate>/Cargo.toml` with `name`, `version = "0.1.0"`, `edition = "2021"`, and a `rust-version` placeholder (to be filled in TASK-03).
- Create `<crate>/src/lib.rs` (or `main.rs` for `hpdl-cli`) with a `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` attribute and a single `//! Crate-level doc comment`.
- Add feature flag stubs per crate as appropriate; document each flag with an inline comment.
- Add `thiserror` as a dependency to every crate that exposes fallible APIs.

**Verification:**
- [x] `cargo build --workspace` succeeds (verified).
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes with zero warnings (verified).
- [x] Each crate has at least one feature-flag entry in its `Cargo.toml`.

---

### TASK-03 — Pin toolchain and set MSRV

**Requirement(s):** REQ-02  
**Effort:** Small  
**Depends on:** TASK-02

**What to do:**
- Create `rust-toolchain.toml` at the repo root with an explicit stable channel pin (requires OD-01 approval).
- Add `rust-version = "<MSRV>"` to each crate's `Cargo.toml` (requires OD-02 approval).
- Commit `Cargo.lock`.

**Verification:**
- [x] `rust-toolchain.toml` exists and pins `1.99.1` (OD-01 ratified in ADR 0001).
- [x] All crates have `rust-version = "1.85.0"` (OD-02 ratified in ADR 0001).
- [x] `Cargo.lock` present at repo root (committed once the repo is under git — see note).
- [x] `CHANGELOG.md` notes the chosen toolchain and MSRV.

> **OD-01 and OD-02:** Ratified in ADR 0001 (2026-10-17) — toolchain `1.99.1`, MSRV `1.85.0`.

---

### TASK-04 — Enforce inter-crate dependency graph rules

**Requirement(s):** REQ-03  
**Effort:** Small  
**Depends on:** TASK-02

**What to do:**
- Wire up `[dependencies]` in each crate's `Cargo.toml` to match the allowed DAG.

**Verification:**
- [x] `hpdl-model` — no workspace deps.
- [x] `hpdl-sema` — depends on `hpdl-model` only.
- [x] `hpdl-pir` — depends on `hpdl-model` only.
- [x] `hpdl-ron` — depends on `hpdl-sema` only.
- [x] `hpdl-compiler` — depends on `hpdl-sema`, `hpdl-pir`, and `hpdl-ron` (edge added in feature 06).
- [x] `hpdl-cli` — depends on `hpdl-compiler` only.
- [ ] `cargo deny check` passes. *(cargo-deny not installed in this environment — run in CI)*

---

### TASK-05 — Create and configure `deny.toml`

**Requirement(s):** REQ-05  
**Effort:** Small  
**Depends on:** TASK-02

**Verification:**
- [x] `deny.toml` exists at repo root with licenses, advisories, and bans sections.
- [x] MIT and Apache-2.0 are in the allowlist (OD-05 default — update after approval).
- [ ] `cargo deny check` exits 0. *(cargo-deny not installed in this environment — run in CI)*

> **OD-05:** License allowlist defaults applied. Pending explicit approval.

---

### TASK-06 — Add repository hygiene files

**Requirement(s):** REQ-07  
**Effort:** Small  
**Depends on:** TASK-01

**Verification:**
- [x] `SECURITY.md` exists — documents disclosure process.
- [x] `CHANGELOG.md` exists — follows Keep a Changelog format.
- [x] `.gitignore` exists — excludes `target/`, editor and OS artifacts.

---

### TASK-07 — Set up CI matrix

**Requirement(s):** REQ-06  
**Effort:** Medium  
**Depends on:** TASK-03, TASK-05, TASK-06

**Verification:**
- [x] `.github/workflows/ci.yml` created with `ubuntu-latest`, `macos-latest`, `windows-latest` matrix.
- [x] Each leg runs: fmt, clippy, test, deny.
- [x] Separate `semver` job runs `cargo semver-checks -p hpdl-pir`.
- [x] Separate `docs` job runs rustdoc with `-D missing_docs`.
- [x] No `continue-on-error: true` in any leg.
- [ ] CI triggers verified on a real PR. *(requires GitHub repo)*

> **OD-03:** GitHub Actions assumed. Pending explicit confirmation.

---

### TASK-08 — Enforce no-panic and typed-error lints in CI

**Requirement(s):** REQ-08  
**Effort:** Small  
**Depends on:** TASK-07

**Verification:**
- [x] `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` in all crates.
- [x] `#![deny(clippy::float_arithmetic)]` in `hpdl-model`, `hpdl-pir`, `hpdl-compiler`.
- [x] `cargo clippy` passes after lint additions (verified).

---

### TASK-09 — Rustdoc and ADR foundation for `hpdl-pir`

**Requirement(s):** REQ-04, REQ-10  
**Effort:** Small  
**Depends on:** TASK-02

**Verification:**
- [x] All public items in `hpdl-pir/src/lib.rs` carry rustdoc comments with examples.
- [x] `cargo doc -p hpdl-pir --no-deps` exits 0 (verified).
- [x] `docs/rfcs/README.md` exists and describes the ADR format.
- [x] `docs/rfcs/0001-workspace-open-decisions.md` lists all five open decisions as Pending.

---

### TASK-10 — Final acceptance pass

**Requirement(s):** All  
**Effort:** Small  
**Depends on:** TASK-01 through TASK-09

**Verification:**
- [x] `cargo fmt --all --check` passes (verified).
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes (verified).
- [x] `cargo test --workspace` passes per-crate (verified; see note on the `hpdl-cli` binary).
- [ ] `cargo deny check` passes. *(cargo-deny not installed in this environment — run in CI)*
- [ ] `cargo semver-checks -p hpdl-pir` passes. *(cargo-semver-checks not installed; no git baseline yet — run in CI)*
- [ ] CI green on Linux, macOS, Windows. *(requires GitHub repo / CI run)*
- [x] No deferred crate in the workspace.
- [x] All five OD decisions recorded in `docs/rfcs/0001-workspace-open-decisions.md`.
- [x] `workflow.md` crate status table updated to `skeleton (feature 00)`.

---

## Task Dependency Summary

```
TASK-01 (workspace manifest)
  └─ TASK-02 (crate skeletons)
       ├─ TASK-03 (toolchain/MSRV)          ← OD-01, OD-02 defaults applied
       ├─ TASK-04 (dependency graph)
       ├─ TASK-05 (deny.toml)               ← OD-05 defaults applied
       └─ TASK-09 (rustdoc + ADRs)
TASK-01
  └─ TASK-06 (hygiene files)
TASK-03 + TASK-05 + TASK-06
  └─ TASK-07 (CI matrix)                   ← OD-03 defaults applied
       └─ TASK-08 (no-panic lints in CI)
TASK-01…TASK-09
  └─ TASK-10 (acceptance pass)
```
