//! The module-resolver seam the single validator consults (ADR 0011).
//!
//! `hpdl-sema` is the one semantic validator and depends only on `hpdl-model`.
//! It must not reach out to a registry, a lockfile, the network, or the
//! compiler. Instead, a caller (the compiler) may supply a [`ModuleResolver`]
//! implementation; the validator consults it to decide whether a declared
//! module reference is backed by a registered, correctly-typed, content-verified
//! implementation.
//!
//! If **no** resolver is supplied, the validator behaves exactly as before the
//! registry existed: every module reference is flagged non-executable. A resolver
//! can only ever *clear* a reference by resolving it; it never introduces new
//! acceptance paths that bypass the existing structural checks.
//!
//! The resolver verifies three things and nothing more (ADR 0011): **existence**,
//! **kind**, and **integrity** (digest match). It makes no authenticity / trust
//! decision — that is deferred (ADR 0008 OD-14.4).

use hpdl_model::ModuleKind;

/// Why a module reference could not be resolved against the registry.
///
/// Every variant is a genuine resolution failure (not "no registry exists"): the
/// validator maps it to the appropriate `Unsupported*` diagnostic so the message
/// explains *why* a declaration is not backed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolverError {
    /// No module with this exact `name@version` is registered.
    NotFound,
    /// A module is registered under this `name@version`, but it implements a
    /// different [`ModuleKind`] than the reference site requires.
    KindMismatch {
        /// The kind the reference site required.
        expected: ModuleKind,
        /// The kind actually registered.
        found: ModuleKind,
    },
    /// The reference pins a content digest that does not match the registered
    /// module's digest (an integrity failure — a tampered or substituted module).
    IntegrityMismatch {
        /// The digest the reference/declaration expected (hex).
        expected: String,
        /// The digest actually registered (hex).
        found: String,
    },
}

/// A read-only resolver the validator consults to back module references with a
/// registered implementation.
///
/// Implementations must be **pure and deterministic**: the same query yields the
/// same result, with no I/O, clock, or ambient state. The compiler's
/// `ModuleRegistry` is the production implementation.
pub trait ModuleResolver {
    /// Resolve a versioned module reference, requiring a specific [`ModuleKind`].
    ///
    /// `expected_digest`, when `Some`, is a content digest the reference pins; a
    /// resolver that finds the module must additionally verify the digest matches
    /// and return [`ResolverError::IntegrityMismatch`] otherwise.
    ///
    /// Returns `Ok(())` when a module with the exact `name@version` exists, has
    /// the `expected` kind, and (if a digest is pinned) matches it.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolverError`] describing the first failure.
    fn resolve_module(
        &self,
        name: &str,
        version: &str,
        expected: ModuleKind,
        expected_digest: Option<&str>,
    ) -> Result<(), ResolverError>;

    /// Resolve a content-addressed baseline artifact by its digest.
    ///
    /// Returns `Ok(())` when a baseline with this exact content digest is
    /// registered. Baselines have no version; identity is the digest itself.
    ///
    /// # Errors
    ///
    /// Returns [`ResolverError::NotFound`] when no baseline with the digest is
    /// registered.
    fn resolve_baseline(&self, digest: &str) -> Result<(), ResolverError>;
}

/// Convenience: resolve through an `Option<&dyn ModuleResolver>`.
///
/// Returns `None` when no resolver is supplied (the "no registry" default — the
/// caller then applies the legacy non-executable flag). Returns `Some(result)`
/// when a resolver is present.
#[must_use]
pub fn resolve_module_opt(
    resolver: Option<&dyn ModuleResolver>,
    name: &str,
    version: &str,
    expected: ModuleKind,
    expected_digest: Option<&str>,
) -> Option<Result<(), ResolverError>> {
    resolver.map(|r| r.resolve_module(name, version, expected, expected_digest))
}

/// Convenience: resolve a baseline through an `Option<&dyn ModuleResolver>`.
#[must_use]
pub fn resolve_baseline_opt(
    resolver: Option<&dyn ModuleResolver>,
    digest: &str,
) -> Option<Result<(), ResolverError>> {
    resolver.map(|r| r.resolve_baseline(digest))
}
