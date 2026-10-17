//! # hpdl-compiler
//!
//! The HPDL compiler pipeline. This crate is the **orchestration layer**: it
//! sequences the existing stages — frontend decode (RON via [`hpdl_ron`] or
//! native `.hpdl` via [`hpdl_syntax`]), the shared semantic validator
//! ([`hpdl_sema`]), module resolution against a lockfile, fixed-point
//! representability checks, and canonical PIR emission ([`hpdl_pir`]) — into a
//! single deterministic path and reports structured, typed diagnostics.
//!
//! It owns **no semantic rules of its own** and performs **no simulation**. It
//! produces immutable artifacts: a canonical [`PirEnvelope`] and a reproducible
//! [`BuildManifest`].
//!
//! ## Dependency constraints
//!
//! This crate sits above `hpdl-ron`, `hpdl-sema`, and `hpdl-pir`. `hpdl-pir` and
//! `hpdl-runtime-api` must **never** depend on this crate; compiler-internal
//! types are not part of the stable runtime API.
//!
//! ## Pipeline
//!
//! ```text
//! source (RON or native .hpdl)
//!   → frontend decode     → PlanetDefinition
//!   → semantic validation → ValidatedDefinition
//!   → module resolution   → ResolvedModules
//!   → representability    → (verified)
//!   → canonical emission  → PirEnvelope (+ content hash)
//!   → manifest            → BuildManifest
//! ```
//!
//! Equivalent semantics yield identical PIR bytes regardless of frontend.
//!
//! ## Example
//!
//! ```
//! use hpdl_compiler::{Compiler, CompileOptions};
//!
//! const SOURCE: &str = r#"
//! PlanetDefinition(
//!     language_version: "1.0",
//!     body: CelestialBody(
//!         id: BodyId("earth"),
//!         properties: PhysicalProperties(
//!             mass_kg: Some(5972000000000000000000000),
//!             radius_m: Some(6371000),
//!             surface_gravity_mms2: Some(9810),
//!             orbital_period_s: None,
//!             rotation_period_s: None,
//!         ),
//!         traits: TraitSet([]),
//!         modules: [],
//!         statuses: [],
//!         actions: [],
//!         migrations: [],
//!     )
//! )
//! "#;
//!
//! let compiler = Compiler::new(CompileOptions::default());
//! let result = compiler.compile_ron_str("earth", SOURCE).expect("compile");
//! assert!(result.envelope().verify_hash());
//! assert_eq!(result.content_hash_hex().len(), 64);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
#![deny(clippy::float_arithmetic)] // no f32/f64 in canonical output paths

mod emit;
mod frontend;
mod lockfile;
mod lower;
mod manifest;
mod options;
mod resolve;
mod result;

use hpdl_sema::{Validator, ValidatorConfig};
use thiserror::Error;

pub use lockfile::{LockedPackage, Lockfile, LOCK_FORMAT_VERSION};
pub use manifest::BuildManifest;
pub use options::{CompileOptions, ResourceLimits};
pub use resolve::{
    verify_integrity, BuildLock, LockedModule, PackageResolver, ResolvedModule, ResolvedModules,
    ResolvedPackage,
};
pub use result::CompileResult;

// Re-export the model package declaration types so the CLI / downstream can
// construct package graphs without a direct `hpdl-model` dependency.
pub use hpdl_model::{PackageId, PackageManifest};

// Re-export the contract types callers need so downstream crates (e.g.
// `hpdl-cli`) can consume compiler output without depending on `hpdl-pir`
// or `hpdl-sema` directly for the common path.
pub use hpdl_pir::{CanonicalHash, PirEnvelope, PirVersion};
pub use hpdl_sema::{Diagnostic, DiagnosticCode, Severity, SourceSpan};

/// Errors produced during compilation.
///
/// Every variant is non-destructive and carries enough context to be
/// actionable in CLI presentation. Unsupported or unresolved declarations fail
/// here rather than silently producing plausible-looking output.
#[derive(Debug, Error)]
pub enum Error {
    /// A frontend failed to decode the source into a model.
    #[error("frontend decode failed for `{name}`: {reason}")]
    Frontend {
        /// The logical source name.
        name: String,
        /// The frontend's diagnostic text.
        reason: String,
    },

    /// Semantic validation produced one or more diagnostics.
    #[error("semantic validation failed with {} diagnostic(s)", .0.len())]
    Validation(Vec<Diagnostic>),

    /// A declared module reference is not present in the lockfile.
    #[error("module `{name}` is not present in the lockfile")]
    UnresolvedModule {
        /// The module name that could not be resolved.
        name: String,
    },

    /// A declared module reference disagrees with the locked version.
    #[error(
        "module `{name}` version mismatch: definition requests {requested}, lock pins {locked}"
    )]
    ModuleVersionMismatch {
        /// The module name.
        name: String,
        /// The version requested by the definition.
        requested: String,
        /// The version pinned by the lockfile.
        locked: String,
    },

    /// The lockfile could not be parsed.
    #[error("lockfile parse error on line {line}: {reason}")]
    LockParse {
        /// 1-indexed line number of the offending entry.
        line: usize,
        /// Why the line was rejected.
        reason: String,
    },

    /// The authoritative lockfile header was missing, malformed, or named an
    /// unsupported format version.
    #[error("lockfile format error: {reason}")]
    LockFormat {
        /// Why the lock format was rejected.
        reason: String,
    },

    /// A declared package (root or transitive dependency) is absent from the
    /// authoritative lockfile.
    #[error("package `{name}` is not present in the lockfile")]
    UnresolvedPackage {
        /// The package name that could not be resolved.
        name: String,
    },

    /// A declared package reference disagrees with the locked version.
    #[error(
        "package `{name}` version mismatch: definition requests {requested}, lock pins {locked}"
    )]
    PackageVersionMismatch {
        /// The package name.
        name: String,
        /// The version requested by the definition.
        requested: String,
        /// The version pinned by the lockfile.
        locked: String,
    },

    /// A resolved package's content digest did not match the expected digest — a
    /// tampered or substituted package. Content hashes are verified before use.
    #[error("package `{name}` integrity check failed: expected {expected}, found {found}")]
    PackageIntegrity {
        /// The package name.
        name: String,
        /// The expected content digest (hex).
        expected: String,
        /// The digest actually found (hex).
        found: String,
    },

    /// The package dependency graph contains a cycle (including a self-dependency),
    /// so the transitive closure cannot be resolved.
    #[error("dependency cycle detected at package `{0}`")]
    DependencyCycle(String),

    /// A numeric field is not representable in its canonical storage width.
    #[error("lowering error for field `{field}`: {reason}")]
    Lowering {
        /// The offending field name.
        field: String,
        /// Why lowering failed.
        reason: String,
    },

    /// Canonical PIR emission failed.
    #[error("canonical emission failed: {0}")]
    Emission(String),

    /// A resource limit was exceeded before or during compilation.
    #[error("resource limit exceeded: {0}")]
    Limit(String),
}

/// The HPDL compiler.
///
/// Holds [`CompileOptions`] and exposes entry points that run the full
/// pipeline. All entry points return [`Result`] and never panic on input.
#[derive(Clone, Debug)]
pub struct Compiler {
    options: CompileOptions,
}

impl Compiler {
    /// Construct a compiler with the given options.
    #[must_use]
    pub fn new(options: CompileOptions) -> Self {
        Self { options }
    }

    /// Return a reference to the compiler options.
    #[must_use]
    pub fn options(&self) -> &CompileOptions {
        &self.options
    }

    /// Compile RON source text to a [`CompileResult`].
    ///
    /// Runs the full pipeline: source-size limit check → RON decode → semantic
    /// validation → module resolution → representability check → canonical PIR
    /// emission → manifest.
    ///
    /// # Errors
    ///
    /// Returns a typed [`Error`] at the first stage that fails. A definition
    /// that fails validation is never lowered or emitted.
    pub fn compile_ron_str(&self, name: &str, source: &str) -> Result<CompileResult, Error> {
        if source.len() > self.options.limits.max_source_bytes {
            return Err(Error::Limit(format!(
                "source `{name}` is {} bytes, exceeds max {}",
                source.len(),
                self.options.limits.max_source_bytes
            )));
        }

        let definition = frontend::decode_ron(name, source)?;
        let validated = self.validate(definition)?;
        self.compile_validated(name, validated)
    }

    /// Compile native `.hpdl` source text to a [`CompileResult`].
    ///
    /// Runs the identical pipeline as [`Self::compile_ron_str`], differing only
    /// in the frontend decode step. Equivalent semantics authored in `.hpdl` or
    /// `.ron` produce byte-identical PIR (ADR 0010).
    ///
    /// # Errors
    ///
    /// Returns a typed [`Error`] at the first stage that fails. A definition
    /// that fails validation is never lowered or emitted.
    pub fn compile_hpdl_str(&self, name: &str, source: &str) -> Result<CompileResult, Error> {
        if source.len() > self.options.limits.max_source_bytes {
            return Err(Error::Limit(format!(
                "source `{name}` is {} bytes, exceeds max {}",
                source.len(),
                self.options.limits.max_source_bytes
            )));
        }

        let definition = frontend::decode_hpdl(name, source)?;
        let validated = self.validate(definition)?;
        self.compile_validated(name, validated)
    }

    /// Compile an already-validated definition to a [`CompileResult`].
    ///
    /// This is the shared tail of the pipeline used by every frontend and by
    /// cross-frontend equivalence tests. Because the only way to obtain a
    /// [`hpdl_sema::ValidatedDefinition`] is through the validator, this path
    /// cannot bypass validation.
    ///
    /// # Errors
    ///
    /// Returns a typed [`Error`] if resolution, lowering, or emission fail.
    pub fn compile_validated(
        &self,
        name: &str,
        validated: hpdl_sema::ValidatedDefinition,
    ) -> Result<CompileResult, Error> {
        let definition = validated.into_inner();

        // Module / package resolution. When an authoritative package lock
        // (feature 14) is supplied it is the source of truth: resolve and verify
        // the full transitive closure, and derive the resolved-module list from the
        // roots. Otherwise fall back to the provisional feature-06 lock. Resolution
        // does not change PIR bytes — it only confirms the build's modules resolve
        // offline and deterministically.
        let modules = if let Some(package_lock) = self.options.package_lock.as_ref() {
            let closure =
                resolve::resolve_packages(&definition, package_lock, &self.options.limits)?;
            resolve::modules_from_closure(&definition, &closure)
        } else {
            resolve::resolve_modules(
                &definition,
                self.options.lock.as_ref(),
                &self.options.limits,
            )?
            .into_vec()
        };

        lower::check_representable(&definition)?;

        let envelope = emit::emit(&definition)?;

        let manifest = BuildManifest::new(name, &envelope, modules);

        Ok(CompileResult::new(envelope, manifest))
    }

    /// Run the shared semantic validator exactly once.
    fn validate(
        &self,
        definition: hpdl_model::PlanetDefinition,
    ) -> Result<hpdl_sema::ValidatedDefinition, Error> {
        let validator = Validator::new(self.validator_config());
        validator.validate(definition).map_err(Error::Validation)
    }

    fn validator_config(&self) -> ValidatorConfig {
        self.options.validator.clone()
    }
}
