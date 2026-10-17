# Immutable Standard-Library Packages & Lockfile — Requirements

## Overview

This feature covers **domain-14 work: the standard library packaging & resolution contract** —
the authoritative, content-addressed, independently versioned package model and the
deterministic `hpdl.lock` format that pins a build's exact transitive closure. The architecture
steering is explicit: "Standard library — immutable, content-addressed, independently versioned
packages, not tied to compiler releases. `hpdl.lock` pins version, content hash and transitive
closure. Never resolve a mutable branch or label."

Feature 06 shipped a deliberately **minimal, provisional** lock format and resolver (OD-06.1
recorded that the authoritative `hpdl.lock` is owned by feature 14). This feature supersedes
that provisional format with an authoritative, format-versioned one that carries the transitive
closure and content digests, adds offline deterministic resolution with **integrity
verification and dependency-cycle rejection**, and introduces the package *manifest* /
*identity* types.

Primary crates: **`hpdl-compiler`** (authoritative `Lockfile`, `PackageResolver`, integrity +
cycle checks) and **`hpdl-model`** (source-independent `PackageId` / `PackageManifest`
declaration types). The simulation engine and clients are unaffected.

> **Critical framing:** This is a *resolution/packaging contract*, not a package registry
> server, archive downloader, or network client. Resolution is **offline** and
> **deterministic**: it reads a lock and verifies declared digests against supplied package
> metadata; it never fetches bytes over a network, never resolves a mutable `latest`/branch,
> and fails closed with typed errors. "Which bytes are at this digest" is supplied to the
> resolver; byte *fetching/decoding* stays out of scope.

## Goals

1. **Package identity & manifest**: a `PackageId` (namespaced name + exact semantic version —
   never a range or label) and a `PackageManifest` (identity + its declared dependency
   `PackageId`s + content digest), in `hpdl-model` as source-independent declarations.
2. **Authoritative lockfile**: a format-versioned `Lockfile` that pins, for every package in
   the transitive closure, its exact version, content digest, and its direct dependencies —
   enough to reconstruct and verify the whole closure deterministically and offline.
3. **Deterministic offline resolver**: a `PackageResolver` that, given a root set of
   `PackageId`s (from a definition's module references) and a `Lockfile`, resolves the full
   transitive closure in a fixed order, with no network/filesystem access and no mutable-label
   resolution.
4. **Integrity verification**: every resolved package's declared digest is checked against the
   lock entry before it is accepted; a mismatch (tampered package) is a typed error. Content
   hashes are verified *before* a package is used.
5. **Cycle & bound safety**: dependency cycles are rejected with a typed error; the closure
   size, per-package dependency count, and lock size are bounded; malformed input fails closed
   and never panics or does unbounded work.
6. **Backward-compatible migration from the provisional format**: the provisional feature-06
   `BuildLock` continues to parse (so existing builds don't break), but the authoritative
   `Lockfile` is the new source of truth; the format carries an explicit format-version so a
   future change is a deliberate, reviewed decision.

## Scope boundaries

- **No registry server / network / archive download.** The resolver takes a `Lockfile` and
  package metadata as *input*; it never contacts a registry, downloads an archive, or decodes
  package bytes. Fetching and byte decoding are a separate concern (a client/registry repo).
- **No mutable references.** A `PackageId` version is an exact pinned version; `latest`, a
  branch, or a tag is rejected. The lock never records a mutable label.
- **No trust-root / signature verification of packages.** Package *integrity* (digest match)
  is verified; package *authenticity* (who signed it, is the registry trusted) is a trust-root
  concern surfaced as an open decision (OD-14.4) and **not** decided in code here.
- **No PIR change.** The lock and manifest are build-time inputs; they influence the build
  *manifest* (resolved-module list) but do not change canonical PIR bytes for a given
  definition. Equivalent semantics still produce identical PIR.
- **No executable module semantics.** A package names and versions a module; whether that
  module has a registered *runtime implementation* is the runtime registry concern (features
  11/12 flagged modules non-executable via `E6005` etc.); this feature resolves package
  identity + integrity, not executability.

## Functional Requirements

### FR1: Package identity (`PackageId`)

**FR1.1**: Introduce `PackageId` in `hpdl-model`: a namespaced package name (same grammar as a
`ModuleRef` name) plus an **exact** version string. Construction rejects empty/overlong fields.

**FR1.2**: `PackageId` rejects any version that is a mutable label rather than an exact pin —
`latest`, a leading `^`/`~`/`>`/`<`/`*`, a branch-like token, or an empty version. Only an
exact version is accepted; a range/label is `Error::MutablePackageVersion`.

### FR2: Package manifest (`PackageManifest`)

**FR2.1**: Introduce `PackageManifest` in `hpdl-model`: a `PackageId`, its content `Digest`,
and its declared direct dependencies (a set of `PackageId`s). Validated at construction; the
dependency set is bounded and deduplicated deterministically (`BTreeSet`-ordered).

**FR2.2**: `PackageManifest` is a *declaration*; it does not fetch, verify bytes, or resolve.
It is the unit the lockfile records and the resolver reasons over.

### FR3: Authoritative lockfile (`Lockfile`)

**FR3.1**: Introduce `Lockfile` in `hpdl-compiler`: a format-versioned, deterministic text
format pinning every package in the transitive closure — `(name, version, content_digest,
[dependency names])`. It carries an explicit `lock_format_version` header so any future format
change is a deliberate, reviewed decision (not a silent divergence).

**FR3.2**: `Lockfile::parse` reads the format deterministically: fixed field order, ignored
blank/comment lines, 1-indexed error reporting, duplicate-package rejection, bounded size. A
digest field must be a valid 64-hex `Digest`. Malformed input is `Error::LockParse` /
`Error::LockFormat`.

**FR3.3**: The provisional feature-06 `BuildLock` format continues to parse unchanged (back-
compat); the authoritative `Lockfile` is the new source of truth and can be used wherever a
build lock is accepted.

### FR4: Deterministic offline resolver (`PackageResolver`)

**FR4.1**: Introduce `PackageResolver` in `hpdl-compiler`: given a root set of `PackageId`s and
a `Lockfile`, it resolves the full transitive closure in a fixed (`BTreeSet`/sorted) order,
offline, with no network or filesystem access and no mutable-label resolution.

**FR4.2**: A root or transitive dependency absent from the lock is `Error::UnresolvedPackage`.
A requested exact version that disagrees with the lock is `Error::PackageVersionMismatch`.

**FR4.3**: Resolution is deterministic and reproducible: the same roots + same lock ⇒ the same
ordered closure on every platform.

### FR5: Integrity & cycle safety

**FR5.1**: Before a resolved package is accepted, its declared content digest is checked
against the lock entry; a mismatch is `Error::PackageIntegrity` (tampered package). Content
hashes are verified before the package is used.

**FR5.2**: A dependency cycle in the closure is rejected with `Error::DependencyCycle` (naming
the offending package); self-dependency counts as a cycle. Resolution never loops unbounded.

**FR5.3**: Bounds are enforced: closure size (reuse/extend `max_modules`), per-package
dependency count, and lock byte size. Exceeding a bound is `Error::Limit`. Malformed input
never panics, indexes out of bounds, or does unbounded work.

### FR6: Integration with the compiler pipeline

**FR6.1**: `CompileOptions` can carry an authoritative `Lockfile` (in addition to / superseding
the provisional `BuildLock`); the resolver runs between validation and lowering, as resolution
does today. A definition declaring no module references resolves to an empty closure.

**FR6.2**: The resolved closure feeds the existing `BuildManifest` (sorted resolved-module
list) with no change to its stable text format beyond what the richer lock provides; PIR bytes
for a given definition are unchanged.

## Non-Functional Requirements

### NFR1 — Determinism: resolution order, lock parsing, and the resolved closure are fully
deterministic — `BTreeSet`/`BTreeMap` or explicit sorting throughout, no `HashMap` iteration
into output, no wall-clock/locale/filesystem order. Same inputs ⇒ identical closure on Linux,
macOS, Windows.

### NFR2 — Security / untrusted input: treat the lockfile and package metadata as untrusted;
enforce size/count bounds; verify content digests before use; reject mutable labels, cycles,
and duplicates; fail closed with typed errors; no panicking paths.

### NFR3 — Independence: packages are versioned independently of compiler releases; the lock
format carries its own version; `PackageId`/`PackageManifest` are source-independent model
types with no parser/PIR coupling.

### NFR4 — Documentation: rustdoc + examples on every new public item; CHANGELOG updated; an
ADR records the resolved decisions (format version, digest-type reuse, resolution scope) and
surfaces the trust-root open decision.

### NFR5 — Tests: unit + integration + golden coverage for: a tampered package (digest
mismatch), a dependency conflict (version mismatch), an offline build, "same locked source ⇒
same PIR", mixed package versions, a dependency cycle, a mutable-label rejection, and bound
violations. A frozen `hpdl.lock` fixture under `tests/golden/`.

## Constraints

- `hpdl-compiler → { hpdl-ron, hpdl-syntax } → hpdl-model`, `hpdl-compiler → hpdl-sema`,
  `hpdl-compiler → hpdl-pir`. No reverse edges; `hpdl-pir`/`hpdl-runtime-api` never depend on
  the compiler. `PackageId`/`PackageManifest` live in `hpdl-model` (no parser/PIR coupling);
  `Lockfile`/`PackageResolver` live in `hpdl-compiler`.
- Reuse `hpdl_model::Digest` for content digests (OD-14.1) and the `ModuleRef`-name grammar for
  package names — do not introduce parallels.
- No registry server, network, archive download/decode, package signing/trust-roots, mutable
  labels, PIR change, or executability decision.

## Dependency & versioning implications (surfaced up front)

- **Lock format version.** The authoritative `Lockfile` carries `lock_format_version` (starting
  at `1`). Changing the format later is a deliberate, reviewed decision with an ADR and golden-
  vector update — never a silent divergence (as OD-06.1 required).
- **Provisional `BuildLock`.** Kept parseable for back-compat; the authoritative `Lockfile` is
  the new source of truth. No existing build breaks.
- **No PIR change.** The lock/manifest are build inputs; canonical PIR bytes for a given
  definition are unchanged. Golden PIR vectors and `PirVersion::CURRENT` untouched.
- **SemVer.** `hpdl-model` gains additive public types (`PackageId`, `PackageManifest`);
  `hpdl-compiler` is `publish = false` (internal), so its API churn is unconstrained by
  semver-checks, but the lock *format* version is the real compatibility surface.

## Open decisions / approvals required — resolved below (ADR 0008); trust-root remains
UNRESOLVED by design

- **OD-14.1 — Content-digest type — RESOLVED: reuse `hpdl_model::Digest`** (already a validated
  64-hex SHA-256 newtype) rather than a new parallel `ContentDigest`. The design doc named
  `ContentDigest`; reusing `Digest` avoids a duplicate type and the 64-hex integrity check the
  design requires comes for free.
- **OD-14.2 — Lock format ownership — RESOLVED: authoritative `Lockfile` supersedes the
  provisional `BuildLock`**, which stays parseable. The authoritative format carries a format-
  version header and the transitive closure; the provisional one is retained only for
  back-compat.
- **OD-14.3 — Resolution scope — RESOLVED: resolve body-level module references in this
  feature.** World-law / bridge / evaluator / climate-model references stay flagged
  non-executable (features 11/12) and are not resolved against the lock here; widening the
  resolved surface is a future, separately-reviewed step.
- **OD-14.4 — Trust roots / package authenticity — UNRESOLVED (surfaced, not decided).**
  Package *integrity* (digest match) is verified; package *authenticity* (signatures, trusted
  registries, trust roots) is explicitly deferred and recorded in ADR 0008 as requiring
  approval before any code lands.

## Success Criteria

1. ✅ `PackageId`, `PackageManifest` (in `hpdl-model`) and `Lockfile`, `PackageResolver` (in
   `hpdl-compiler`) exist with validated constructors, typed errors, and rustdoc.
2. ✅ Mutable labels, duplicate packages, version mismatches, digest mismatches, dependency
   cycles, and bound violations are explicit typed errors; nothing silently succeeds or
   resolves a mutable label.
3. ✅ Resolution is offline and deterministic; the same roots + lock ⇒ the same ordered closure
   cross-platform; the authoritative lock carries a format version.
4. ✅ The provisional `BuildLock` still parses; the resolved closure feeds `BuildManifest`; PIR
   bytes for a given definition are unchanged.
5. ✅ A frozen `hpdl.lock` golden fixture + the design's required test cases (tampered,
   conflict, offline, same-locked-source-same-PIR, mixed versions) pass.
6. ✅ CI passes; CHANGELOG + ADR 0008 updated; OD-14.4 (trust roots) surfaced, not resolved.

## Out of Scope

- A package registry server, network transport, archive download, or package byte
  decoding/verification-of-bytes (client/registry repo).
- Package signing, trust roots, authenticity (OD-14.4 — deferred).
- Mutable `latest`/branch/tag resolution (always rejected).
- Any PIR format change; native `.hpdl` / RON syntax changes; CLI presentation (feature 15).
- Deciding module *executability* (runtime registry; features 11/12).

## References

- Steering: `architecture.md` (Standard library: immutable, content-addressed, independently
  versioned; `hpdl.lock` pins version/hash/closure; never a mutable label; dependency
  direction), `rust-standards.md` (verify content hashes before parsing packages; reject path
  traversal / cycles; bound untrusted input), `determinism.md` (deterministic resolution,
  no hashmap iteration), `product.md` (definitions and runtime state separately versioned).
- Builds on: feature 06 (provisional `BuildLock`/`resolve_modules`, OD-06.1 deferring the
  authoritative lock to this feature; `BuildManifest`).
- Design: `docs/design/14-stdlib-packages.md`.
