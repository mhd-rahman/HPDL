---
inclusion: fileMatch
fileMatchPattern: ["**/*.rs", "**/Cargo.toml", "rust-toolchain.toml", "deny.toml"]
---

# HPDL — Rust Standards

## Workspace
- Commit `Cargo.lock`. Pin toolchain in `rust-toolchain.toml`. Document MSRV.
- Explicit crate features; record enabled features for reproducible builds.
- Contract crates (`hpdl-pir`, `hpdl-runtime-api`) stay semver-stable with minimal dependencies.

## Reliability
- Strongly typed domain structs; `Result` with typed errors (`thiserror`).
- No `unwrap()` / `expect()` / `panic!` / indexing that can panic on untrusted-input paths.
- Checked arithmetic (`checked_*`) in all numeric and fixed-point code.
- No silent defaults that change planetary semantics.
- No `f32`/`f64` in consensus-critical code.
- Use `BTreeMap`/`BTreeSet` (or explicit sorting) wherever order can reach output or hashes. Never iterate a `HashMap` into canonical output.

## Untrusted input & security
- Treat `.hpdl`, `.ron`, PIR blobs, lockfiles, package archives, snapshots and migration payloads as untrusted.
- Enforce limits on input size, nesting depth, collection length, allocation and decode work. Fail closed with typed errors.
- Verify content hashes before parsing packages. Reject path traversal, symlinks escaping package roots, and dependency cycles.
- Never execute code from definitions.

## Documentation
rustdoc on every public item in published crates, with examples. Format and compatibility decisions go in ADRs.

## Repo hygiene files
`SECURITY.md`, `CHANGELOG.md`, `deny.toml` must exist and stay current.
