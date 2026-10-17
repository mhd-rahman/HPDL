# Compiler Pipeline — Design

> **Status:** Engineering design for `hpdl-compiler` v0.1. Grounded in the current
> public APIs of `hpdl-model`, `hpdl-sema`, `hpdl-pir`, and `hpdl-ron`. Not an
> assertion of simulation or runtime behavior.

## 1. Purpose

Orchestrate the deterministic path from authored source to canonical PIR:

```
source (RON today, HPDL later)
  → [frontend decode]        hpdl-ron  → PlanetDefinition
  → [semantic validation]    hpdl-sema → ValidatedDefinition
  → [module resolution]      lockfile  → ResolvedModules
  → [lowering / repr check]  checked   → (model, verified representable)
  → [canonical emission]     hpdl-pir  → PirEnvelope (+ content hash)
  → [manifest]               BuildManifest
```

Equivalent semantics must yield identical PIR bytes regardless of frontend. The
compiler adds no semantic rules and performs no simulation.

## 2. Module layout

```
crates/hpdl-compiler/src/
  lib.rs          # crate docs, re-exports, Error, high-level Compiler
  options.rs      # CompileOptions, ResourceLimits
  result.rs       # CompileResult
  frontend.rs     # RON decode seam (frontend-neutral core)
  resolve.rs      # BuildLock, LockedModule, ResolvedModules, resolution
  lower.rs        # representability checks (fixed-point lowering guard)
  emit.rs         # canonical PIR emission + hash verification
  manifest.rs     # BuildManifest + deterministic text serialization
crates/hpdl-compiler/tests/
  pipeline.rs     # end-to-end + negative tests
  equivalence.rs  # compile_validated vs compile_ron_str parity
  golden_build.rs # frozen hash + manifest fixture
crates/hpdl-compiler/tests/golden/
  minimal_build.txt   # frozen manifest + hash for a known planet
```

## 3. Public API (precise signatures)

```rust
// lib.rs
pub use options::{CompileOptions, ResourceLimits};
pub use result::CompileResult;
pub use resolve::{BuildLock, LockedModule, ResolvedModule, ResolvedModules};
pub use manifest::BuildManifest;

// Re-export the contract types the caller needs so hpdl-cli need not depend on
// hpdl-pir/hpdl-model directly for the common path.
pub use hpdl_pir::{CanonicalHash, PirEnvelope, PirVersion};

pub struct Compiler { options: CompileOptions }

impl Compiler {
    pub fn new(options: CompileOptions) -> Self;

    /// Decode + validate + resolve + lower + emit from RON source text.
    pub fn compile_ron_str(&self, name: &str, source: &str)
        -> Result<CompileResult, Error>;

    /// Shared tail of the pipeline from an already-validated definition.
    /// Used by every frontend and by equivalence tests.
    pub fn compile_validated(&self, name: &str, validated: ValidatedDefinition)
        -> Result<CompileResult, Error>;
}
```

```rust
// options.rs
#[derive(Clone, Debug)]
pub struct CompileOptions {
    pub validator: hpdl_sema::ValidatorConfig,
    pub lock: Option<BuildLock>,
    pub limits: ResourceLimits,
}
impl Default for CompileOptions { /* strict, deterministic */ }

#[derive(Clone, Copy, Debug)]
pub struct ResourceLimits {
    pub max_source_bytes: usize, // default 10 MiB (matches PIR MAX_PIR_SIZE)
    pub max_modules: usize,      // default 1024
}
```

```rust
// result.rs
#[derive(Clone, Debug)]
pub struct CompileResult {
    envelope: PirEnvelope,
    manifest: BuildManifest,
}
impl CompileResult {
    pub fn envelope(&self) -> &PirEnvelope;
    pub fn manifest(&self) -> &BuildManifest;
    pub fn content_hash_hex(&self) -> String; // envelope.content_hash.to_hex()
    pub fn into_parts(self) -> (PirEnvelope, BuildManifest);
}
```

```rust
// resolve.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockedModule { pub name: String, pub version: String, pub content_hash: String }

#[derive(Clone, Debug, Default)]
pub struct BuildLock { entries: BTreeMap<String, LockedModule> }
impl BuildLock {
    pub fn parse(text: &str) -> Result<Self, Error>;   // deterministic line format
    pub fn get(&self, name: &str) -> Option<&LockedModule>;
    pub fn is_empty(&self) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedModule { pub name: String, pub version: String, pub content_hash: String }

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedModules { modules: Vec<ResolvedModule> } // sorted by name
```

```rust
// manifest.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildManifest {
    pub source_name: String,
    pub compiler_semantics_version: String, // CARGO_PKG_VERSION
    pub pir_version: PirVersion,
    pub content_hash: String,                // hex
    pub modules: Vec<ResolvedModule>,        // sorted
}
impl BuildManifest {
    pub fn to_text(&self) -> String; // stable, sorted, line-oriented
}
```

```rust
// lib.rs
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("frontend decode failed for `{source}`: {reason}")]
    Frontend { source: String, reason: String },

    #[error("semantic validation failed with {} diagnostic(s)", .0.len())]
    Validation(Vec<hpdl_sema::Diagnostic>),

    #[error("module `{name}` is not present in the lockfile")]
    UnresolvedModule { name: String },

    #[error("module `{name}` version mismatch: definition requests {requested}, lock pins {locked}")]
    ModuleVersionMismatch { name: String, requested: String, locked: String },

    #[error("lockfile parse error on line {line}: {reason}")]
    LockParse { line: usize, reason: String },

    #[error("lowering error for field `{field}`: {reason}")]
    Lowering { field: String, reason: String },

    #[error("canonical emission failed: {0}")]
    Emission(String),

    #[error("resource limit exceeded: {0}")]
    Limit(String),
}
```

> Note: `Error::Validation` carries `hpdl_sema::Diagnostic`. `Diagnostic`,
> `DiagnosticCode`, `Severity`, `SourceSpan` are re-exported from `hpdl-sema`;
> this preserves stable diagnostic codes for CLI presentation (feature 15).

## 4. Stage design

### 4.1 Frontend decode (`frontend.rs`)
- `decode_ron(name, source)` builds a `SourceDocument::from_string(name, source)`
  and runs `RonLoader::new().parse(&source)`, mapping any `hpdl_ron::Error` into
  `Error::Frontend`.
- Returns a bare `PlanetDefinition` (via `ParsedDefinition::into_definition`) so the
  core pipeline is frontend-neutral. When `hpdl-syntax` lands it adds a sibling
  `decode_hpdl` that returns the same type; nothing downstream changes.

### 4.2 Validation
- `Validator::new(options.validator.clone()).validate(def)` → on `Err(diags)` map to
  `Error::Validation(diags)`. Success yields `ValidatedDefinition`.
- `compile_validated` enters here-after, guaranteeing one and only one validation.

### 4.3 Module resolution (`resolve.rs`)
- Collect the body's `ModuleRef`s (deterministic order from the model's storage).
- For each `ModuleRef`:
  - look up `lock.get(name)`:
    - absent lock entry (or no lock at all) ⇒ `Error::UnresolvedModule`.
    - version != `module_ref.version()` ⇒ `Error::ModuleVersionMismatch`.
    - else push a `ResolvedModule { name, version, content_hash }`.
- Sort resolved modules by `name` (`BTreeMap` collection → `Vec`) for stable output.
- Enforce `limits.max_modules` on the resolved count (`Error::Limit`).
- Empty module set resolves to empty `ResolvedModules` regardless of lock presence.

**Lock format** (minimal, deterministic; OD-06.1):
```
# comment lines start with '#'
<name> <version> <content_hash>
```
- One module per non-empty, non-comment line; exactly three whitespace-separated
  fields. Any other shape ⇒ `Error::LockParse { line, reason }`.
- Duplicate `name` ⇒ `Error::LockParse`. Stored in a `BTreeMap` so iteration order is
  fixed.

### 4.4 Lowering / representability guard (`lower.rs`)
- The model already stores canonical scaled integers, so lowering here is a
  **representability guard**, not a re-scaling:
  - `mass_kg: u128` and `radius_m: u128` must fit the PIR storage; verified with
    checked conversion against the documented width. (Current PIR encodes the model
    directly via bincode, so the guard checks the model's own invariants and
    reserves the seam for Amendment A2 per-field quanta.)
  - `surface_gravity_mms2: u64`, `orbital_period_s: u64`, `rotation_period_s: i64`
    are range-checked with `checked_*` where any arithmetic is introduced.
- On any overflow / unrepresentable magnitude ⇒ `Error::Lowering { field, reason }`.
- Pure function of the validated model; no ambient state. No float arithmetic
  (crate-wide deny).

### 4.5 Canonical emission (`emit.rs`)
- `encode_planet_definition(def)` → `PirEnvelope`, mapping `hpdl_pir::Error` to
  `Error::Emission`.
- Call `envelope.verify_hash()`; a `false` result is an internal invariant breach →
  `Error::Emission("content hash verification failed")`.
- No re-implementation of encoding or hashing; `hpdl-pir` remains the single source
  of canonical bytes.

### 4.6 Manifest (`manifest.rs`)
- Built from: `source_name`, `env!("CARGO_PKG_VERSION")`, `envelope.version`,
  `envelope.content_hash.to_hex()`, and the sorted `ResolvedModules`.
- `to_text()` emits sorted, line-oriented key/value pairs then one line per module,
  e.g.:
  ```
  source_name = minimal
  compiler_semantics_version = 0.1.0
  pir_version = 1.0
  content_hash = <64 hex chars>
  module = hpdl.atmosphere 1.2.0 <hash>
  ```
- Deterministic: no HashMap iteration; module lines already sorted.

## 5. Determinism & numerics

- Every ordering that reaches the manifest or hash inputs uses `BTreeMap`/sorted
  `Vec`.
- No clock, locale, env, network, RNG, or filesystem reads on the compile path.
- No `f32`/`f64`; `#![deny(clippy::float_arithmetic)]`. Any numeric check uses
  `checked_*`.
- The only hashing is delegated to `hpdl-pir`'s domain-separated SHA-256 over
  canonical bytes. The compiler never hashes `Debug` output or in-memory layout.

## 6. Security / untrusted input

- `compile_ron_str` checks `source.len() > limits.max_source_bytes` **before**
  constructing a `SourceDocument` → `Error::Limit`.
- `BuildLock::parse` bounds per-line fields and rejects malformed/duplicate entries;
  it allocates one `BTreeMap` entry per valid line only.
- Resolution bounds the closure with `limits.max_modules`.
- No `unwrap`/`expect`/`panic`/panicking index on any path; all fallible steps
  return typed `Error`.

## 7. Testing strategy

**`tests/pipeline.rs`** (positive + negative):
- minimal RON compiles → `verify_hash()` true, manifest well-formed.
- oversized source → `Error::Limit`.
- malformed RON → `Error::Frontend`.
- definition that fails sema → `Error::Validation`.
- lock parse errors (wrong arity, duplicate name) → `Error::LockParse`.
- module declared but absent from lock → `Error::UnresolvedModule`.
- module version mismatch → `Error::ModuleVersionMismatch`.
- `max_modules` exceeded → `Error::Limit`.

**`tests/equivalence.rs`**:
- Build a `ValidatedDefinition` directly and via RON for the same semantics; assert
  identical `envelope.payload` and `content_hash`.

**`tests/golden_build.rs`** + `tests/golden/minimal_build.txt`:
- Freeze the canonical content hash and the manifest `to_text()` for a fixed planet.
- Changing these requires a deliberate, reviewed fixture update (never silently
  regenerated) and, if PIR bytes change, a PIR format-version decision in feature 02
  — out of scope to change here.

All tests avoid wall-clock/locale/order dependence so they hold on Linux, macOS, and
Windows.

## 8. Dependency / compatibility impact

- Adds `hpdl-compiler → hpdl-ron` (new Cargo edge; matches architecture steering).
- Keeps `hpdl-compiler → hpdl-sema`, `→ hpdl-pir`. `hpdl-model` reached transitively;
  model types used in signatures are re-exported via the contract crates where
  possible.
- No reverse edges introduced into `hpdl-pir` / `hpdl-runtime-api`.
- No PIR format change; no golden-vector change in `hpdl-pir`.
- `hpdl-compiler` 0.1.x: additive public surface, not a published contract crate.

## 9. Definition of done

- All stages implemented with unit + integration coverage and a golden build fixture.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo deny check` all pass.
- Rustdoc on every public item; CHANGELOG updated.
- No unsupported feature silently claims simulation or success; all failure modes are
  typed and documented.
