# 0001 — Workspace Foundation — Open Decisions

**Status:** Accepted
**Date:** 2026-10-14 (ratified 2026-10-17)
**Feature:** `00-workspace-foundation`

## Context

The Workspace Foundation feature (TASK-03, TASK-05, TASK-07) could not be fully
completed without resolving five configuration decisions. They were recorded as
"default used, pending approval" and are now **ratified** below. Where a ratified
value differs from the original default, the change and its rationale are noted.

---

## Decisions (ratified)

### OD-01 — Pinned toolchain version

**Question:** Which exact stable Rust release should be pinned in `rust-toolchain.toml`?
**Ratified:** ✅ `1.99.1`
**Effective:** 2026-10-17
**Rationale:** This is an internal project that tracks current stable. `1.99.1`
(released 2026-10-15) is the patched successor to `1.99.0` — it fixed three serious
defects, so it is the safe current pin rather than `1.99.0` itself. The original
starting default was `1.81.0`.
**Impact:** `rust-toolchain.toml` channel; CI `RUST_VERSION`; all CI matrix legs.

---

### OD-02 — MSRV

**Question:** What is the Minimum Supported Rust Version in each crate's `package.rust-version`?
**Ratified:** ✅ `1.85.0`
**Effective:** 2026-10-17
**Rationale:** Raised from the original `1.80` default to keep a meaningful but
generous gap below the pinned toolchain (`1.99.1`). `1.85.0` is recent enough to
avoid old-compiler baggage while still giving internal consumers of the published
contract crates (`hpdl-pir`, `hpdl-runtime-api`) a ~14-release cushion. The `msrv`
CI job builds and tests the whole workspace at this version so the floor is proven,
not just declared.
**Impact:** `rust-version` in all crate manifests; CI `RUST_MSRV`; the `msrv` CI job.

---

### OD-03 — CI provider

**Question:** Is GitHub Actions the confirmed CI provider?
**Ratified:** ✅ GitHub Actions
**Effective:** 2026-10-17
**Rationale:** The repo is hosted on GitHub; Actions is the native, zero-friction
choice. Workflows live under `.github/workflows/` (`ci.yml`, `security-audit.yml`,
`coverage.yml`, `fuzz.yml`) and use `dtolnay/rust-toolchain`,
`EmbarkStudios/cargo-deny-action`, and `taiki-e/install-action`.
**Impact:** Workflow file locations and action references.

---

### OD-04 — `cargo semver-checks` failure policy

**Question:** Should `cargo semver-checks` fail on warnings as well as errors, or errors only?
**Ratified:** ✅ Errors only (default behaviour — non-zero exit on a detected breaking change)
**Effective:** 2026-10-17
**Rationale:** The contract crates are pre-1.0 and internal; gating on genuine
breaking-change detection is the right strictness. Warnings are advisory and should
not block a merge. Runs on `hpdl-pir` and `hpdl-runtime-api`.
**Impact:** CI `semver` job gating.

---

### OD-05 — License allowlist

**Question:** Which SPDX license identifiers are permitted in the dependency graph?
**Ratified:** ✅ `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `Unicode-3.0`
**Effective:** 2026-10-17
**Rationale:** Matches the actual `deny.toml` allowlist. The original ADR draft
listed `Unicode-DFS-2016`, but the dependency graph requires `Unicode-3.0`
(unicode-ident 1.0.27+ relicensed to `(MIT OR Apache-2.0) AND Unicode-3.0`); the
ADR text is corrected to match `deny.toml`. Any license outside this set requires an
explicit, justified `[[licenses.exceptions]]` entry.
**Impact:** `deny.toml` `licenses.allow`; the `cargo deny` CI gate.

---

## Resolution record

All five decisions are ratified as of 2026-10-17 and the corresponding files
(`rust-toolchain.toml`, every crate `Cargo.toml`, `.github/workflows/*.yml`,
`deny.toml`) reflect the ratified values. This ADR is now **Accepted**. A future
change to any of these values requires a new ADR superseding the relevant decision.
