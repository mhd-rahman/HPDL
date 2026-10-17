# Requirements — Workspace Foundation

> **Feature:** `00-workspace-foundation`  
> **Design doc:** `docs/design/00-workspace-foundation.md`  
> **Status:** Draft

---

## Overview

Establish the Cargo workspace that hosts all HPDL crates. This feature covers the project skeleton, inter-crate dependency graph, toolchain pinning, release policies, CI matrix, and repository hygiene files. Nothing is simulated or executed here; this is the structural foundation every other feature builds on.

---

## Functional Requirements

### REQ-01 — Cargo workspace layout

The repository root **must** contain a `Cargo.toml` workspace manifest that declares all active crates as members.

**Acceptance criteria:**
- `cargo build --workspace` completes without errors on a fresh checkout.
- The manifest lists exactly: `hpdl-model`, `hpdl-sema`, `hpdl-ron`, `hpdl-pir`, `hpdl-compiler`, `hpdl-cli`.
- Deferred crates (`hpdl-runtime-api`, `hpdl-syntax`, `hpdl-lsp`) are **not** included until their feature is explicitly started.
- Workspace-level `[profile.*]` and `[patch]` sections are defined once at the root, not duplicated per crate.

---

### REQ-02 — Toolchain and MSRV pinning

The project **must** pin the Rust toolchain and document the Minimum Supported Rust Version (MSRV).

**Acceptance criteria:**
- A `rust-toolchain.toml` exists at the repo root with an explicit `channel` (stable release, not `"stable"`).
- The MSRV is recorded in every crate's `Cargo.toml` under `package.rust-version`.
- `cargo +<pinned-toolchain> check --workspace` succeeds.
- Toolchain changes require a deliberate update to `rust-toolchain.toml` and a `CHANGELOG.md` entry.

---

### REQ-03 — Crate dependency graph rules

Inter-crate dependencies **must** obey a strict directed acyclic order to prevent coupling compiler internals to runtime consumers.

**Acceptance criteria:**
- Allowed dependency direction (top → bottom): `hpdl-cli` → `hpdl-compiler` → `hpdl-sema` → `hpdl-model`; `hpdl-compiler` → `hpdl-pir`; `hpdl-ron` → `hpdl-sema`.
- `hpdl-pir` and `hpdl-runtime-api` have **zero** dependencies on `hpdl-compiler`, `hpdl-sema`, or `hpdl-ron`.
- `hpdl-model` has **zero** workspace dependencies.
- `cargo deny check` reports no duplicate or disallowed dependency paths.
- No cyclic dependencies exist (`cargo build --workspace` would fail, but must also be explicitly tested with `cargo deny`).

---

### REQ-04 — Contract crate stability (`hpdl-pir`)

`hpdl-pir` is a published contract crate and **must** maintain SemVer stability.

**Acceptance criteria:**
- `hpdl-pir/Cargo.toml` has a `version` field following SemVer.
- `cargo semver-checks -p hpdl-pir` passes on every CI run after the initial `1.0.0` publish.
- The crate's public surface exports only stable types (no `pub use` of compiler-internal structs).
- Every public item carries a `rustdoc` doc comment with at least one usage example.

---

### REQ-05 — Dependency vetting (`deny.toml`)

All third-party crates **must** be audited for license compatibility and known vulnerabilities.

**Acceptance criteria:**
- `deny.toml` exists at the repo root and is non-empty (configured, not default).
- `cargo deny check` passes in CI with no `deny`-level license or advisory violations.
- Any exception requires an explicit `[advisories.ignore]` or `[licenses.exceptions]` entry with a justification comment.
- `Cargo.lock` is committed so dependency resolution is reproducible.

---

### REQ-06 — CI matrix

Automated checks **must** run on Linux, macOS, and Windows for every pull request and main-branch push.

**Acceptance criteria:**
- CI configuration (GitHub Actions or equivalent) defines three OS targets: `ubuntu-latest`, `macos-latest`, `windows-latest`.
- Each matrix leg runs: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`.
- The `hpdl-pir` leg additionally runs `cargo semver-checks -p hpdl-pir`.
- A failing leg blocks merge; no leg may be marked `continue-on-error: true` without explicit approval.
- CI uses the toolchain version specified in `rust-toolchain.toml`.

---

### REQ-07 — Repository hygiene files

The repository **must** include standard project hygiene files.

**Acceptance criteria:**
- `SECURITY.md` exists and documents the vulnerability disclosure process and contact.
- `CHANGELOG.md` exists and follows a versioned, dated format (e.g. Keep a Changelog).
- Both files are updated as part of any release PR.
- A `.gitignore` at the repo root excludes `target/`, editor-specific files, and OS artifacts.

---

### REQ-08 — Typed errors and no panics on untrusted input

Every fallible public API in workspace crates **must** use typed `Result` errors; no operation on untrusted data may panic.

**Acceptance criteria:**
- All public fallible functions return `Result<T, E>` where `E` is a concrete type derived with `thiserror`.
- No `unwrap()`, `expect()`, `panic!()`, or slice indexing that can reach untrusted data in production code paths (enforced by Clippy lint `clippy::unwrap_used` and `clippy::indexing_slicing` in CI).
- Arithmetic on untrusted numeric fields uses `checked_*` methods.
- `f32`/`f64` are banned from any code path that produces canonical or consensus output.

---

### REQ-09 — Crate feature flags

Optional capabilities **must** be gated behind explicit Cargo feature flags, never compiled in unconditionally.

**Acceptance criteria:**
- Each crate's `Cargo.toml` documents every declared feature flag with a comment.
- `cargo build --workspace --no-default-features` succeeds.
- Feature combinations used in CI are recorded so builds are reproducible.
- No feature silently enables another crate's private internals.

---

### REQ-10 — Rustdoc coverage on published crates

Every public item in `hpdl-pir` (and any other published crate) **must** be documented.

**Acceptance criteria:**
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p hpdl-pir --no-deps` exits 0.
- Doc comments include at least a one-sentence description and a minimal code example where the API is non-trivial.
- Format and compatibility decisions are recorded as ADRs under `docs/rfcs/`.

---

## Non-Functional Requirements

| ID | Requirement |
|---|---|
| NFR-01 | All CI jobs complete within 15 minutes on a standard hosted runner. |
| NFR-02 | `Cargo.lock` is committed; no floating dependency versions in workspace `Cargo.toml`. |
| NFR-03 | No network access is required for `cargo test --workspace` (tests are self-contained). |
| NFR-04 | Workspace compiles on stable Rust at the pinned toolchain version; no nightly features. |
| NFR-05 | `cargo doc --workspace --no-deps` produces no warnings. |

---

## Out of Scope

- Implementing any domain logic, parsing, validation, or PIR encoding (those are features 01–16).
- Scaffolding deferred crates (`hpdl-runtime-api`, `hpdl-syntax`, `hpdl-lsp`).
- Publishing crates to crates.io (publication is a release-process concern, not part of this feature).
- LSP or editor integration.

---

## Open Decisions (require explicit approval before implementation)

| # | Decision | Owner |
|---|---|---|
| OD-01 | Exact pinned toolchain version (stable channel tag). | — |
| OD-02 | MSRV value (e.g. 1.75 or 1.80). | — |
| OD-03 | CI provider (GitHub Actions assumed; confirm). | — |
| OD-04 | `cargo semver-checks` run policy: fail-on-warn or fail-on-error only. | — |
| OD-05 | License allowlist for `deny.toml` (MIT/Apache-2.0 assumed; dual-license strategy TBD). | — |
