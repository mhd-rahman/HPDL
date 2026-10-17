# 0008 — Immutable Standard-Library Packages & Authoritative Lockfile

**Status:** Accepted
**Date:** 2026-10-17
**Feature:** `14-stdlib-packages` (OD-14.1 … OD-14.4)

## Context

The architecture steering requires an immutable, content-addressed, independently versioned
standard library whose `hpdl.lock` pins version, content hash, and transitive closure, and which
never resolves a mutable branch or label. Feature 06 shipped a deliberately minimal, provisional
lock format and resolver, explicitly deferring the authoritative `hpdl.lock` to this feature
(OD-06.1). No registry, package, or lock *format-versioned* types existed before this feature.

## Decision

Add the authoritative packaging + resolution contract: `PackageId` / `PackageManifest` in
`hpdl-model` (source-independent declarations), and a format-versioned `Lockfile` +
offline-deterministic `PackageResolver` (with integrity verification and dependency-cycle
rejection) in `hpdl-compiler`. The provisional `BuildLock` stays parseable; the authoritative
`Lockfile` is the new source of truth and, when supplied, drives module resolution.

- **OD-14.1 — Reuse `hpdl_model::Digest` for content digests.** `Digest` is already a validated
  64-hex SHA-256 newtype; reusing it avoids a parallel `ContentDigest` and gives the integrity
  check for free. (The design doc named `ContentDigest`; `Digest` subsumes it.)
- **OD-14.2 — Authoritative `Lockfile` supersedes the provisional `BuildLock`.** The
  authoritative format carries a `lock_format_version = 1` header and the transitive closure
  (`package = name version digest [deps...]`). The provisional line format is retained only for
  back-compat; existing builds do not break.
- **OD-14.3 — Resolve body-level module references in this feature.** World-law / bridge /
  evaluator / climate-model references remain flagged non-executable (features 11/12) and are
  not resolved against the lock here; widening the resolved surface is a future, separately
  reviewed step.
- **OD-14.4 — Trust roots / package authenticity: UNRESOLVED (surfaced, not decided).** Package
  *integrity* (digest match) is verified via `verify_integrity`; package *authenticity*
  (signatures, trusted registries, trust roots) is explicitly deferred and requires approval
  before any code lands.

## Rationale

This gives HPDL a real, format-versioned lockfile and a deterministic offline resolver that
verifies integrity and rejects cycles and mutable labels — exactly what the architecture
steering demands — without adding a registry server, network transport, archive download, or
PIR change. Resolution is pure: same roots + same lock ⇒ identical ordered closure on every
platform. A tampered package, a version conflict, an unresolved package, a cycle, or a mutable
label is an explicit typed error.

## Alternatives considered

| Option | Why rejected |
|--------|-------------|
| New `ContentDigest` type | Duplicates `hpdl_model::Digest`; no added value. |
| Keep the provisional line format as authoritative | No format version, no transitive closure — cannot verify a build's full closure or evolve the format safely. |
| Resolve world-law/evaluator refs now | Those are flagged non-executable pending the runtime registry; widening scope here would conflate packaging with executability. |
| Fetch/verify package bytes in the resolver | Network/archive handling belongs to a client/registry repo; the compiler resolves offline against supplied metadata. |
| Add a `toml`/`ron` dependency for the lock | A hand-rolled deterministic line format matches the provisional format's style and keeps `hpdl-compiler` dependency-light and the bytes trivially canonical. |

## Compatibility impact

- **`hpdl-model`** gains additive public types (`PackageId`, `PackageManifest`,
  `MAX_PACKAGE_DEPS`) and `Error` variants (`MutablePackageVersion`, `TooManyDependencies`,
  `SelfDependency`). Pre-1.0.
- **`hpdl-compiler`** (`publish = false`, internal) gains `Lockfile`, `LockedPackage`,
  `PackageResolver`, `ResolvedPackage`, `verify_integrity`, `CompileOptions.package_lock`, and
  new `Error` variants. The lock *format version* (`1`) is the real compatibility surface.
- **No PIR change.** The lock/manifest are build inputs; canonical PIR bytes for a given
  definition are unchanged (an integration test asserts a definition hashes identically with
  and without a package lock). `PirVersion::CURRENT` and golden PIR vectors untouched.

## Test consequences

- Model unit tests: mutable-label rejection, exact-version acceptance, dependency dedup/order,
  self-dependency rejection.
- Lockfile unit tests: parse round-trip, format-version header required / unknown rejected,
  duplicate / malformed-digest rejection, size bound.
- Resolver unit tests: transitive closure + ordering, unresolved package, missing transitive
  dep, dependency cycle (incl. self), closure bound, integrity mismatch, version mismatch.
- Integration (`tests/packages.rs`): offline build, dependency conflict, unresolved package,
  missing transitive dep, same-locked-source-same-PIR, lock-does-not-change-PIR, mixed versions,
  and a frozen `tests/golden/stdlib.lock` fixture that round-trips.

## Unresolved — surfaced, NOT decided in code

- **OD-14.4 trust roots / package authenticity.** Who signs a package, which registries are
  trusted, and how trust roots are established and rotated is deferred to a future ADR. This
  feature verifies integrity (digest match) only; it does not verify authenticity.
