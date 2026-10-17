# Immutable Standard-Library Packages & Lockfile — Design

## 1. Scope

Supersede feature 06's provisional lock with an authoritative, format-versioned `hpdl.lock`
that pins the transitive closure, plus an offline deterministic `PackageResolver` with
integrity verification and cycle rejection. Package identity/manifest declarations live in
`hpdl-model`; the lockfile + resolver live in `hpdl-compiler`. No registry server, no network,
no archive download, no PIR change.

## 2. Module layout

```
hpdl-model/src/package.rs      -- PackageId, PackageManifest (+ Error variants)
hpdl-compiler/src/lockfile.rs  -- Lockfile (authoritative, format-versioned)
hpdl-compiler/src/resolve.rs   -- extend: PackageResolver, closure walk, integrity, cycles
hpdl-compiler/src/options.rs   -- CompileOptions gains an authoritative lock
hpdl-compiler/src/lib.rs       -- new Error variants + re-exports
```

## 3. Model types (`hpdl-model/src/package.rs`)

### 3.1 `PackageId`

```rust
pub struct PackageId { name: String, version: String }
```
- `name`: `ModuleRef`-name grammar (1..=128, non-empty).
- `version`: exact pin only. `PackageId::new` rejects a mutable label with
  `Error::MutablePackageVersion`: empty, `latest`, or a first char in `^~><*=`, or any
  whitespace. (Deliberately strict: an exact version like `1.2.0` or `1.2.0-rc.1` is accepted;
  a range/label is not.)
- Derives `Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize` so it is
  `BTreeSet`-orderable for deterministic closures.

### 3.2 `PackageManifest`

```rust
pub struct PackageManifest {
    id: PackageId,
    content_digest: Digest,
    dependencies: BTreeSet<PackageId>,
}
```
`new(id, content_digest, deps) -> Result<Self, Error>`: dedups deps into a `BTreeSet`, enforces
`deps.len() <= MAX_PACKAGE_DEPS` (bound, else `Error::TooManyDependencies`), and rejects a
self-dependency (`id ∈ deps` → `Error::SelfDependency`). Accessors return ordered views. A
declaration only — no fetch, no verify-bytes, no resolution.

### 3.3 Model `Error` additions

`MutablePackageVersion(String)`, `TooManyDependencies(usize)`, `SelfDependency(String)`,
reusing `InvalidModuleRef`-style validation for names.

## 4. Authoritative lockfile (`hpdl-compiler/src/lockfile.rs`)

### 4.1 Format (text, deterministic)

```
lock_format_version = 1
package = <name> <version> <digest> [<dep-name> ...]
```
- First non-comment line MUST be `lock_format_version = N`; an unknown N is `Error::LockFormat`.
- Each `package =` line: name, exact version, 64-hex digest, then zero+ dependency names
  (names only; versions resolved via the dep's own `package =` line in the closure).
- Blank/`#` lines ignored; 1-indexed errors; duplicate package name → `Error::LockParse`;
  malformed digest → `Error::LockParse`; byte-size bound (reuse `max_source_bytes`).

### 4.2 `Lockfile` type

```rust
pub struct Lockfile {
    format_version: u32,
    entries: BTreeMap<String, LockedPackage>,
}
pub struct LockedPackage { name, version, content_digest: Digest, dependencies: BTreeSet<String> }
```
`parse(text) -> Result<Self, Error>`; `get(name)`; `format_version()`; `len()`/`is_empty()`.
`BTreeMap`/`BTreeSet` keep iteration deterministic. A `to_text()` re-serializer emits the
canonical form (sorted) for golden fixtures and `hpdl lock` output.

## 5. Resolver (`hpdl-compiler/src/resolve.rs`, extended)

### 5.1 `PackageResolver`

```rust
pub struct PackageResolver<'a> { lock: &'a Lockfile, limits: &'a ResourceLimits }
impl PackageResolver<'_> {
    pub fn resolve_closure(&self, roots: &[&str]) -> Result<Vec<ResolvedPackage>, Error>;
}
pub struct ResolvedPackage { name, version, content_digest: Digest, dependencies: Vec<String> }
```

### 5.2 Algorithm (offline, deterministic, bounded)

1. Seed a `BTreeSet<String>` worklist from sorted `roots`.
2. BFS/DFS over the lock, maintaining `visited: BTreeSet` and an `in_progress: BTreeSet` stack
   for cycle detection. For each name:
   - absent in lock → `Error::UnresolvedPackage(name)`.
   - re-entered while `in_progress` → `Error::DependencyCycle(name)` (self-dep included).
   - digest not valid / mismatched vs an expected digest (when the definition or a parent
     manifest carries one) → `Error::PackageIntegrity { name, expected, found }`.
   - closure size `> limits.max_modules` → `Error::Limit`.
3. Push each dependency name; recurse. Collect into a `BTreeMap<String, ResolvedPackage>` so
   output is sorted by name. Same roots + lock ⇒ identical ordered `Vec`.

### 5.3 Integrity

`verify_integrity(expected: &Digest, found: &Digest)` is a constant-time-ish `==` on the two
64-hex strings (both already validated `Digest`s). Verified *before* a package is accepted
into the closure. A tampered package (lock digest ≠ expected digest supplied by the
definition/parent) fails closed.

### 5.4 Bridging module refs → packages

`resolve_modules` (feature 06) still resolves `definition.body().modules()` against a build
lock. New: a `resolve_packages(definition, &Lockfile, limits)` path maps each
`ModuleRef{name,version}` to a root `PackageId`, calls `resolve_closure`, and verifies each
resolved package's version matches the requested one (`Error::PackageVersionMismatch`). OD-14.3:
only body-level module refs are roots in this feature.

## 6. Compiler integration (`options.rs`, `lib.rs`)

- `CompileOptions` gains `pub package_lock: Option<Lockfile>` and `with_package_lock(..)`.
  When present, the pipeline resolves the package closure (between validation and lowering) in
  addition to the provisional `BuildLock` module resolution; when absent, behaviour is
  unchanged (back-compat).
- New `Error` variants in `hpdl-compiler`: `UnresolvedPackage(String)`,
  `PackageVersionMismatch { name, requested, locked }`, `PackageIntegrity { name, expected,
  found }`, `DependencyCycle(String)`, `LockFormat { reason }`. Each carries actionable context
  for CLI presentation (feature 15).
- The resolved closure is folded into the existing `BuildManifest` resolved-module list (sorted
  by name). PIR bytes for a given definition are **unchanged** — the lock influences the
  manifest, not canonical PIR.

## 7. Determinism, security, forbidden list

- `BTreeMap`/`BTreeSet`/explicit sorting everywhere; no `HashMap` into output; no wall-clock,
  locale, or filesystem order; same roots + lock ⇒ identical closure cross-platform.
- Treat the lock + package metadata as untrusted: byte-size bound, closure-size bound,
  per-package dependency bound; verify digests before use; reject mutable labels, duplicates,
  and cycles; no `unwrap`/`expect`/panic/indexing on input paths; checked arithmetic on counts.
- **Not** added: registry server, network, archive download/decode, package signing / trust
  roots (OD-14.4 deferred), mutable-label resolution, PIR change, executability decision.

## 8. Tests

- Model unit tests (`package.rs`): mutable-label rejection (`latest`, `^1`, `~1`, `>=1`, `*`,
  empty), exact-version acceptance, self-dependency rejection, dependency bound, deterministic
  ordering.
- Lockfile unit tests (`lockfile.rs`): parse round-trip, format-version header required /
  unknown rejected, duplicate package rejected, malformed digest rejected, size bound.
- Resolver unit tests (`resolve.rs`): closure resolution + ordering, unresolved package,
  version mismatch, digest mismatch (tampered), dependency cycle (incl. self), closure bound,
  offline (no I/O), empty roots → empty closure.
- Integration (`tests/packages.rs`): tampered package, dependency conflict, offline build,
  mixed package versions, "same locked source ⇒ same PIR" (compile twice, compare content
  hashes).
- Golden: a frozen `crates/hpdl-compiler/tests/golden/stdlib.lock` fixture + its expected
  canonical `to_text()`.

## 9. Compatibility

- `hpdl-model` additive (`PackageId`, `PackageManifest`); `hpdl-compiler` is `publish = false`.
- Lock format version starts at `1`; future changes are deliberate + ADR + golden update.
- No PIR change; provisional `BuildLock` still parses.
- ADR 0008 records OD-14.1…OD-14.3 and surfaces OD-14.4 (trust roots) as unresolved.
