//! The compile-time module registry (HPDL feature 14 registry concern, ADR 0011).
//!
//! A [`ModuleRegistry`] is an offline, deterministic index of registered module
//! implementations. It answers three questions and no others: does a
//! `name@version` **exist**, is its **kind** the one a reference site requires,
//! and does its content **digest** match. It performs no I/O, fetches no bytes,
//! runs no module code, and makes **no** authenticity / trust-root judgement
//! (that is deferred — ADR 0008 OD-14.4, ADR 0011).
//!
//! The registry implements [`hpdl_sema::ModuleResolver`] so the single semantic
//! validator can consult it without `hpdl-sema` gaining any dependency on this
//! crate: the trait lives in `hpdl-sema`, the implementation here.

use std::collections::{BTreeMap, BTreeSet};

use hpdl_model::{ModuleKind, RegisteredModule};
use hpdl_sema::{ModuleResolver, ResolverError};

/// An offline, deterministic registry of module implementations.
///
/// Built from a set of [`RegisteredModule`] descriptors. Code modules are keyed
/// by `(name, version)`; baseline artifacts are keyed by content digest (they
/// have no version — identity *is* the digest).
///
/// # Example
///
/// ```
/// use hpdl_compiler::ModuleRegistry;
/// use hpdl_model::{Digest, ModuleKind, PackageId, RegisteredModule};
///
/// let reg = ModuleRegistry::from_modules([
///     RegisteredModule::new(
///         PackageId::new("hpdl.vitality.eco", "1.0.0").unwrap(),
///         ModuleKind::Evaluator,
///         Digest::new("a".repeat(64)).unwrap(),
///     ),
/// ]);
/// assert_eq!(reg.len(), 1);
/// ```
#[derive(Clone, Debug, Default)]
pub struct ModuleRegistry {
    /// Code modules keyed by `(name, version)` → `(kind, digest-hex)`.
    modules: BTreeMap<(String, String), (ModuleKind, String)>,
    /// Registered baseline content digests (hex).
    baselines: BTreeSet<String>,
}

impl ModuleRegistry {
    /// Build a registry from a set of registered module descriptors.
    ///
    /// A [`ModuleKind::Baseline`] descriptor registers its digest as a baseline
    /// artifact (its version is ignored — baselines are identified by content);
    /// every other kind registers a `(name, version) → (kind, digest)` entry.
    /// Later duplicates overwrite earlier ones deterministically.
    #[must_use]
    pub fn from_modules(modules: impl IntoIterator<Item = RegisteredModule>) -> Self {
        let mut code: BTreeMap<(String, String), (ModuleKind, String)> = BTreeMap::new();
        let mut baselines: BTreeSet<String> = BTreeSet::new();

        for m in modules {
            let digest = m.digest().as_str().to_string();
            if m.kind() == ModuleKind::Baseline {
                baselines.insert(digest);
            } else {
                code.insert(
                    (m.id().name().to_string(), m.id().version().to_string()),
                    (m.kind(), digest),
                );
            }
        }

        Self {
            modules: code,
            baselines,
        }
    }

    /// The number of registered code modules (excludes baseline artifacts).
    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Whether the registry has no registered code modules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// The number of registered baseline artifacts.
    #[must_use]
    pub fn baseline_count(&self) -> usize {
        self.baselines.len()
    }
}

impl ModuleResolver for ModuleRegistry {
    fn resolve_module(
        &self,
        name: &str,
        version: &str,
        expected: ModuleKind,
        expected_digest: Option<&str>,
    ) -> Result<(), ResolverError> {
        // Existence: an exact (name, version) must be registered.
        let (found_kind, found_digest) = self
            .modules
            .get(&(name.to_string(), version.to_string()))
            .ok_or(ResolverError::NotFound)?;

        // Kind: the registered kind must match what the reference site requires.
        if *found_kind != expected {
            return Err(ResolverError::KindMismatch {
                expected,
                found: *found_kind,
            });
        }

        // Integrity: if the reference pins a digest, it must match the registry's.
        if let Some(expected_digest) = expected_digest {
            if expected_digest != found_digest {
                return Err(ResolverError::IntegrityMismatch {
                    expected: expected_digest.to_string(),
                    found: found_digest.clone(),
                });
            }
        }

        Ok(())
    }

    fn resolve_baseline(&self, digest: &str) -> Result<(), ResolverError> {
        if self.baselines.contains(digest) {
            Ok(())
        } else {
            Err(ResolverError::NotFound)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{Digest, PackageId};

    fn reg() -> ModuleRegistry {
        ModuleRegistry::from_modules([
            RegisteredModule::new(
                PackageId::new("hpdl.vitality.eco", "1.0.0").unwrap(),
                ModuleKind::Evaluator,
                Digest::new("a".repeat(64)).unwrap(),
            ),
            RegisteredModule::new(
                PackageId::new("hpdl.climate.gcm", "2.0.0").unwrap(),
                ModuleKind::ClimateModel,
                Digest::new("b".repeat(64)).unwrap(),
            ),
            RegisteredModule::new(
                PackageId::new("baseline.preindustrial", "1.0.0").unwrap(),
                ModuleKind::Baseline,
                Digest::new("c".repeat(64)).unwrap(),
            ),
        ])
    }

    #[test]
    fn resolves_existing_module_of_correct_kind() {
        assert_eq!(
            reg().resolve_module("hpdl.vitality.eco", "1.0.0", ModuleKind::Evaluator, None),
            Ok(())
        );
    }

    #[test]
    fn missing_module_is_not_found() {
        assert_eq!(
            reg().resolve_module("hpdl.unknown", "1.0.0", ModuleKind::Evaluator, None),
            Err(ResolverError::NotFound)
        );
    }

    #[test]
    fn wrong_version_is_not_found() {
        assert_eq!(
            reg().resolve_module("hpdl.vitality.eco", "9.9.9", ModuleKind::Evaluator, None),
            Err(ResolverError::NotFound)
        );
    }

    #[test]
    fn wrong_kind_is_kind_mismatch() {
        assert_eq!(
            reg().resolve_module("hpdl.climate.gcm", "2.0.0", ModuleKind::Evaluator, None),
            Err(ResolverError::KindMismatch {
                expected: ModuleKind::Evaluator,
                found: ModuleKind::ClimateModel,
            })
        );
    }

    #[test]
    fn digest_match_passes_and_mismatch_fails() {
        // Matching digest passes.
        assert_eq!(
            reg().resolve_module(
                "hpdl.vitality.eco",
                "1.0.0",
                ModuleKind::Evaluator,
                Some(&"a".repeat(64)),
            ),
            Ok(())
        );
        // Mismatched digest is an integrity failure.
        assert!(matches!(
            reg().resolve_module(
                "hpdl.vitality.eco",
                "1.0.0",
                ModuleKind::Evaluator,
                Some(&"f".repeat(64)),
            ),
            Err(ResolverError::IntegrityMismatch { .. })
        ));
    }

    #[test]
    fn baseline_resolution() {
        assert_eq!(reg().resolve_baseline(&"c".repeat(64)), Ok(()));
        assert_eq!(
            reg().resolve_baseline(&"d".repeat(64)),
            Err(ResolverError::NotFound)
        );
        assert_eq!(reg().baseline_count(), 1);
    }
}
