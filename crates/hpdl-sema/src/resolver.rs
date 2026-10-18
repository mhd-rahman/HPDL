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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// A minimal in-memory resolver used to exercise the `Option<&dyn …>`
    /// convenience helpers and the `ResolverError` variants directly, without
    /// routing through the full `Validator`.
    struct StubResolver {
        /// `name@version -> kind` for the single module it knows.
        module: Option<(String, String, ModuleKind)>,
        /// The one baseline digest it knows, if any.
        baseline: Option<String>,
    }

    impl ModuleResolver for StubResolver {
        fn resolve_module(
            &self,
            name: &str,
            version: &str,
            expected: ModuleKind,
            expected_digest: Option<&str>,
        ) -> Result<(), ResolverError> {
            let Some((n, v, kind)) = &self.module else {
                return Err(ResolverError::NotFound);
            };
            if n != name || v != version {
                return Err(ResolverError::NotFound);
            }
            if *kind != expected {
                return Err(ResolverError::KindMismatch {
                    expected,
                    found: *kind,
                });
            }
            if let Some(want) = expected_digest {
                // The stub pins a fixed digest to make integrity failures observable.
                let have = "a".repeat(64);
                if want != have {
                    return Err(ResolverError::IntegrityMismatch {
                        expected: want.to_string(),
                        found: have,
                    });
                }
            }
            Ok(())
        }

        fn resolve_baseline(&self, digest: &str) -> Result<(), ResolverError> {
            match &self.baseline {
                Some(d) if d == digest => Ok(()),
                _ => Err(ResolverError::NotFound),
            }
        }
    }

    fn resolver_knowing_evaluator() -> StubResolver {
        StubResolver {
            module: Some((
                "hpdl.vitality.eco".to_string(),
                "1.0.0".to_string(),
                ModuleKind::Evaluator,
            )),
            baseline: Some("b".repeat(64)),
        }
    }

    #[test]
    fn resolve_module_opt_is_none_without_resolver() {
        let got = resolve_module_opt(None, "x", "1.0.0", ModuleKind::Evaluator, None);
        assert!(got.is_none(), "no resolver must yield None (legacy default)");
    }

    #[test]
    fn resolve_module_opt_resolves_existing_module() {
        let r = resolver_knowing_evaluator();
        let got = resolve_module_opt(
            Some(&r),
            "hpdl.vitality.eco",
            "1.0.0",
            ModuleKind::Evaluator,
            None,
        );
        assert_eq!(got, Some(Ok(())));
    }

    #[test]
    fn resolve_module_opt_reports_not_found() {
        let r = resolver_knowing_evaluator();
        let got = resolve_module_opt(Some(&r), "missing", "1.0.0", ModuleKind::Evaluator, None);
        assert_eq!(got, Some(Err(ResolverError::NotFound)));
    }

    #[test]
    fn resolve_module_opt_reports_kind_mismatch() {
        let r = resolver_knowing_evaluator();
        let got = resolve_module_opt(
            Some(&r),
            "hpdl.vitality.eco",
            "1.0.0",
            ModuleKind::ClimateModel,
            None,
        );
        assert_eq!(
            got,
            Some(Err(ResolverError::KindMismatch {
                expected: ModuleKind::ClimateModel,
                found: ModuleKind::Evaluator,
            }))
        );
    }

    #[test]
    fn resolve_module_opt_reports_integrity_mismatch() {
        let r = resolver_knowing_evaluator();
        let got = resolve_module_opt(
            Some(&r),
            "hpdl.vitality.eco",
            "1.0.0",
            ModuleKind::Evaluator,
            Some(&"f".repeat(64)),
        );
        assert_eq!(
            got,
            Some(Err(ResolverError::IntegrityMismatch {
                expected: "f".repeat(64),
                found: "a".repeat(64),
            }))
        );
    }

    #[test]
    fn resolve_module_opt_integrity_match_is_ok() {
        let r = resolver_knowing_evaluator();
        let got = resolve_module_opt(
            Some(&r),
            "hpdl.vitality.eco",
            "1.0.0",
            ModuleKind::Evaluator,
            Some(&"a".repeat(64)),
        );
        assert_eq!(got, Some(Ok(())));
    }

    #[test]
    fn resolve_baseline_opt_is_none_without_resolver() {
        assert!(resolve_baseline_opt(None, &"b".repeat(64)).is_none());
    }

    #[test]
    fn resolve_baseline_opt_resolves_known_digest() {
        let r = resolver_knowing_evaluator();
        assert_eq!(resolve_baseline_opt(Some(&r), &"b".repeat(64)), Some(Ok(())));
    }

    #[test]
    fn resolve_baseline_opt_reports_not_found() {
        let r = resolver_knowing_evaluator();
        assert_eq!(
            resolve_baseline_opt(Some(&r), &"e".repeat(64)),
            Some(Err(ResolverError::NotFound))
        );
    }

    #[test]
    fn resolver_error_is_debug_clone_eq() {
        // The error type is part of the sema public surface; exercise its derives.
        let err = ResolverError::KindMismatch {
            expected: ModuleKind::Evaluator,
            found: ModuleKind::ClimateModel,
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
        assert!(format!("{err:?}").contains("KindMismatch"));
        assert_ne!(err, ResolverError::NotFound);
    }
}
