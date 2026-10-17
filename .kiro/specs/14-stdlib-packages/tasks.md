# Immutable Standard-Library Packages & Lockfile — Tasks

> Scope: authoritative content-addressed package model + format-versioned `hpdl.lock` + offline
> deterministic resolver with integrity + cycle checks. No registry server, network, archive
> download, package signing / trust roots (OD-14.4 deferred), mutable labels, or PIR change.
> OD-14.1…OD-14.3 resolved in ADR 0008.

- [x] 1. `PackageId` + `PackageManifest` in `hpdl-model`
  - `package.rs`: exact-version-only `PackageId` (mutable labels rejected), bounded/dedup
    `PackageManifest` with `Digest` content hash; self-dependency rejected. New `Error`
    variants. Export from `lib.rs`. Unit tests + doctests.
  - **Files:** `crates/hpdl-model/src/package.rs`, `error.rs`, `lib.rs`.

- [x] 2. Authoritative `Lockfile` in `hpdl-compiler`
  - `lockfile.rs`: format-versioned parse (`lock_format_version = 1`), transitive-closure
    entries (`package = name version digest [deps...]`), `BTreeMap`-backed, `to_text()`
    re-serializer, bounds, 1-indexed errors. Unit tests + doctests.
  - **Files:** `crates/hpdl-compiler/src/lockfile.rs`, `lib.rs`.

- [x] 3. `PackageResolver` + closure walk in `resolve.rs`
  - Offline deterministic BFS/DFS over the lock; integrity check before accept; cycle detection
    (incl. self); closure bound; `ResolvedPackage` sorted output; `resolve_packages` bridge
    from `ModuleRef` roots. Unit tests.
  - **Files:** `crates/hpdl-compiler/src/resolve.rs`.

- [x] 4. Compiler Error variants + options integration
  - `UnresolvedPackage`, `PackageVersionMismatch`, `PackageIntegrity`, `DependencyCycle`,
    `LockFormat`; `CompileOptions.package_lock` + `with_package_lock`; wire into the pipeline
    between validation and lowering; fold closure into `BuildManifest`; PIR bytes unchanged.
  - **Files:** `crates/hpdl-compiler/src/lib.rs`, `options.rs`.

- [x] 5. Integration + golden tests
  - `tests/packages.rs`: tampered, conflict, offline, mixed versions, same-locked-source-same-
    PIR; frozen `tests/golden/stdlib.lock`.
  - **Files:** `crates/hpdl-compiler/tests/packages.rs`, `tests/golden/stdlib.lock`.

- [x] 6. ADR 0008 + CHANGELOG
  - Record OD-14.1…OD-14.3; surface OD-14.4 (trust roots). CHANGELOG section.
  - **Files:** `docs/rfcs/0008-stdlib-packages-lockfile.md`, `CHANGELOG.md`, `docs/rfcs/README.md`.

- [x] 7. Verify: fmt, clippy -D warnings, test --workspace.

## Out of scope (explicit)
- Registry server / network / archive download / package byte decoding.
- Package signing / trust roots / authenticity (OD-14.4 deferred).
- Mutable-label resolution; PIR change; CLI presentation (feature 15); executability decision.
