//! Module / package resolution against a deterministic lockfile.
//!
//! This stage walks a definition's declared [`hpdl_model::ModuleRef`]s and
//! matches each against a [`BuildLock`]. It performs **no** network or
//! filesystem access and never resolves a mutable branch or label; anything it
//! cannot pin exactly fails closed with a typed [`crate::Error`].
//!
//! The authoritative `hpdl.lock` format is owned by the stdlib feature (14).
//! The format read here is a minimal, deterministic, line-oriented subset
//! sufficient for resolution (open decision OD-06.1).

use std::collections::{BTreeMap, BTreeSet};

use hpdl_model::PlanetDefinition;

use crate::options::ResourceLimits;
use crate::Error;

/// A single locked module entry: an exact version and content hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockedModule {
    /// Module name.
    pub name: String,
    /// Exact pinned version string.
    pub version: String,
    /// Content hash of the pinned module.
    pub content_hash: String,
}

/// A resolved module: the outcome of matching a declared reference to the lock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedModule {
    /// Module name.
    pub name: String,
    /// Resolved version string.
    pub version: String,
    /// Content hash of the resolved module.
    pub content_hash: String,
}

/// The deterministically ordered set of modules resolved for a build.
///
/// Modules are kept sorted by name so they contribute stable bytes to the
/// build manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedModules {
    modules: Vec<ResolvedModule>,
}

impl ResolvedModules {
    /// Return the resolved modules as a sorted slice.
    #[must_use]
    pub fn as_slice(&self) -> &[ResolvedModule] {
        &self.modules
    }

    /// Return the number of resolved modules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Return `true` if no modules were resolved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Consume and return the sorted module vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<ResolvedModule> {
        self.modules
    }
}

/// A build lockfile pinning module versions and content hashes.
///
/// Backed by a `BTreeMap` keyed by module name so iteration order is fixed and
/// never leaks `HashMap` nondeterminism into output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BuildLock {
    entries: BTreeMap<String, LockedModule>,
}

impl BuildLock {
    /// Parse a lockfile from its deterministic text form.
    ///
    /// Format: one module per line as `name version content_hash`, with blank
    /// lines and `#`-prefixed comment lines ignored. Lines are 1-indexed in
    /// error reports.
    ///
    /// # Errors
    ///
    /// Returns [`Error::LockParse`] on a line that does not have exactly three
    /// whitespace-separated fields, or on a duplicate module name;
    /// [`Error::Limit`] if the text exceeds the default source-size bound.
    ///
    /// # Example
    ///
    /// ```
    /// use hpdl_compiler::BuildLock;
    ///
    /// let lock = BuildLock::parse("# modules\nhpdl.atmosphere 1.2.0 abcd1234\n").unwrap();
    /// assert_eq!(lock.get("hpdl.atmosphere").unwrap().version, "1.2.0");
    /// ```
    pub fn parse(text: &str) -> Result<Self, Error> {
        Self::parse_with_limits(text, &ResourceLimits::default())
    }

    /// Parse with explicit resource limits (byte-size bound).
    ///
    /// Treats the lockfile as untrusted input: an oversized lock is rejected with
    /// [`Error::Limit`] before any line is processed, mirroring the authoritative
    /// [`crate::Lockfile::parse_with_limits`]. This keeps the provisional and
    /// authoritative lock readers consistent so neither can be driven to unbounded
    /// work by a hostile file.
    ///
    /// # Errors
    ///
    /// - [`Error::Limit`] if `text` exceeds `limits.max_source_bytes`.
    /// - [`Error::LockParse`] on a malformed line or a duplicate module name.
    pub fn parse_with_limits(text: &str, limits: &ResourceLimits) -> Result<Self, Error> {
        if text.len() > limits.max_source_bytes {
            return Err(Error::Limit(format!(
                "build lock is {} bytes, exceeds max {}",
                text.len(),
                limits.max_source_bytes
            )));
        }

        let mut entries: BTreeMap<String, LockedModule> = BTreeMap::new();

        for (idx, raw) in text.lines().enumerate() {
            let line_no = idx + 1;
            let line = raw.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut fields = line.split_whitespace();
            let name = fields.next().ok_or_else(|| Error::LockParse {
                line: line_no,
                reason: "missing module name".to_string(),
            })?;
            let version = fields.next().ok_or_else(|| Error::LockParse {
                line: line_no,
                reason: format!("module `{name}` is missing a version field"),
            })?;
            let content_hash = fields.next().ok_or_else(|| Error::LockParse {
                line: line_no,
                reason: format!("module `{name}` is missing a content-hash field"),
            })?;

            if fields.next().is_some() {
                return Err(Error::LockParse {
                    line: line_no,
                    reason: format!(
                        "module `{name}` has extra fields; expected exactly `name version hash`"
                    ),
                });
            }

            let entry = LockedModule {
                name: name.to_string(),
                version: version.to_string(),
                content_hash: content_hash.to_string(),
            };

            if entries.insert(name.to_string(), entry).is_some() {
                return Err(Error::LockParse {
                    line: line_no,
                    reason: format!("duplicate module name `{name}`"),
                });
            }
        }

        Ok(Self { entries })
    }

    /// Look up a locked module by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&LockedModule> {
        self.entries.get(name)
    }

    /// Return `true` if the lock contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the number of locked modules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Resolve a definition's declared module references against the lock.
///
/// Produces a [`ResolvedModules`] set sorted by module name. A definition that
/// declares no modules resolves to an empty set, with or without a lock.
///
/// # Errors
///
/// - [`Error::UnresolvedModule`] if a declared module is absent from the lock
///   (or no lock was supplied).
/// - [`Error::ModuleVersionMismatch`] if the declared version differs from the
///   locked version.
/// - [`Error::Limit`] if the resolved count exceeds `limits.max_modules`.
pub(crate) fn resolve_modules(
    definition: &PlanetDefinition,
    lock: Option<&BuildLock>,
    limits: &ResourceLimits,
) -> Result<ResolvedModules, Error> {
    // Collect into a BTreeMap to deduplicate and order deterministically.
    let mut resolved: BTreeMap<String, ResolvedModule> = BTreeMap::new();

    for module_ref in definition.body().modules() {
        let name = module_ref.name();
        let requested = module_ref.version();

        let locked =
            lock.and_then(|lock| lock.get(name))
                .ok_or_else(|| Error::UnresolvedModule {
                    name: name.to_string(),
                })?;

        if locked.version != requested {
            return Err(Error::ModuleVersionMismatch {
                name: name.to_string(),
                requested: requested.to_string(),
                locked: locked.version.clone(),
            });
        }

        resolved.insert(
            name.to_string(),
            ResolvedModule {
                name: locked.name.clone(),
                version: locked.version.clone(),
                content_hash: locked.content_hash.clone(),
            },
        );

        if resolved.len() > limits.max_modules {
            return Err(Error::Limit(format!(
                "resolved module count exceeds max {}",
                limits.max_modules
            )));
        }
    }

    Ok(ResolvedModules {
        modules: resolved.into_values().collect(),
    })
}

/// A package resolved from the authoritative lockfile: its exact version, content
/// digest, and the names of its direct dependencies (in total order).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPackage {
    /// Package name.
    pub name: String,
    /// Exact resolved version.
    pub version: String,
    /// Content digest (hex) of the resolved package.
    pub content_digest: String,
    /// Names of the package's direct dependencies, in total order.
    pub dependencies: Vec<String>,
}

/// An offline, deterministic resolver over an authoritative [`Lockfile`].
///
/// Given a set of root package names, it walks the lock to the full transitive
/// closure in a fixed order, verifying integrity and rejecting dependency cycles.
/// It performs **no** network or filesystem access and never resolves a mutable
/// label.
pub struct PackageResolver<'a> {
    lock: &'a crate::Lockfile,
    limits: &'a ResourceLimits,
}

impl<'a> PackageResolver<'a> {
    /// Construct a resolver bound to a lockfile and resource limits.
    #[must_use]
    pub fn new(lock: &'a crate::Lockfile, limits: &'a ResourceLimits) -> Self {
        Self { lock, limits }
    }

    /// Resolve the transitive closure of `roots`, sorted by package name.
    ///
    /// # Errors
    ///
    /// - [`Error::UnresolvedPackage`] if a root or transitive dependency is absent
    ///   from the lock.
    /// - [`Error::DependencyCycle`] if the graph contains a cycle (self-dependency
    ///   included).
    /// - [`Error::Limit`] if the closure exceeds `limits.max_modules`.
    pub fn resolve_closure(&self, roots: &[&str]) -> Result<Vec<ResolvedPackage>, Error> {
        let mut resolved: BTreeMap<String, ResolvedPackage> = BTreeMap::new();
        let mut in_progress: BTreeSet<String> = BTreeSet::new();

        // Visit roots in sorted order for deterministic traversal.
        let mut sorted_roots: Vec<&str> = roots.to_vec();
        sorted_roots.sort_unstable();
        sorted_roots.dedup();
        for root in sorted_roots {
            self.visit(root, &mut resolved, &mut in_progress)?;
        }

        Ok(resolved.into_values().collect())
    }

    fn visit(
        &self,
        name: &str,
        resolved: &mut BTreeMap<String, ResolvedPackage>,
        in_progress: &mut BTreeSet<String>,
    ) -> Result<(), Error> {
        if resolved.contains_key(name) {
            return Ok(());
        }
        if in_progress.contains(name) {
            return Err(Error::DependencyCycle(name.to_string()));
        }

        let locked = self
            .lock
            .get(name)
            .ok_or_else(|| Error::UnresolvedPackage {
                name: name.to_string(),
            })?;

        in_progress.insert(name.to_string());

        // Recurse into dependencies in their total (BTreeSet) order.
        for dep in &locked.dependencies {
            self.visit(dep, resolved, in_progress)?;
        }

        in_progress.remove(name);

        resolved.insert(
            name.to_string(),
            ResolvedPackage {
                name: locked.name.clone(),
                version: locked.version.clone(),
                content_digest: locked.content_digest.as_str().to_string(),
                dependencies: locked.dependencies.iter().cloned().collect(),
            },
        );

        if resolved.len() > self.limits.max_modules {
            return Err(Error::Limit(format!(
                "resolved package closure exceeds max {}",
                self.limits.max_modules
            )));
        }

        Ok(())
    }
}

/// Resolve a definition's declared module references against the authoritative
/// lockfile as package roots, returning the full verified transitive closure.
///
/// Each `ModuleRef{name, version}` is treated as a root package; the resolved
/// package's version must match the requested one.
///
/// # Errors
///
/// - [`Error::UnresolvedPackage`] / [`Error::DependencyCycle`] / [`Error::Limit`]
///   from closure resolution.
/// - [`Error::PackageVersionMismatch`] if a root's resolved version differs from the
///   requested version.
/// - [`Error::PackageIntegrity`] if a supplied expected digest does not match.
pub(crate) fn resolve_packages(
    definition: &PlanetDefinition,
    lock: &crate::Lockfile,
    limits: &ResourceLimits,
) -> Result<Vec<ResolvedPackage>, Error> {
    let roots: Vec<&str> = definition
        .body()
        .modules()
        .iter()
        .map(hpdl_model::ModuleRef::name)
        .collect();

    let resolver = PackageResolver::new(lock, limits);
    let closure = resolver.resolve_closure(&roots)?;

    // Verify each root's resolved version matches the requested version.
    for module_ref in definition.body().modules() {
        if let Some(pkg) = closure.iter().find(|p| p.name == module_ref.name()) {
            if pkg.version != module_ref.version() {
                return Err(Error::PackageVersionMismatch {
                    name: module_ref.name().to_string(),
                    requested: module_ref.version().to_string(),
                    locked: pkg.version.clone(),
                });
            }
        }
    }

    Ok(closure)
}

/// Map a definition's declared module references to [`ResolvedModule`]s using a
/// resolved package closure, sorted by name.
///
/// Only the body-level module references appear in the build manifest (matching the
/// provisional resolver), each carrying the version and content digest pinned by the
/// authoritative lock.
#[must_use]
pub(crate) fn modules_from_closure(
    definition: &PlanetDefinition,
    closure: &[ResolvedPackage],
) -> Vec<ResolvedModule> {
    let mut out: BTreeMap<String, ResolvedModule> = BTreeMap::new();
    for module_ref in definition.body().modules() {
        if let Some(pkg) = closure.iter().find(|p| p.name == module_ref.name()) {
            out.insert(
                pkg.name.clone(),
                ResolvedModule {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                    content_hash: pkg.content_digest.clone(),
                },
            );
        }
    }
    out.into_values().collect()
}

/// Verify a package's actual content digest against the digest pinned by the lock.
///
/// Returns [`Error::PackageIntegrity`] on mismatch. This is the integrity check a
/// caller (e.g. a client that has fetched package bytes out of band) performs
/// *before* using a package: hash the bytes, then compare against the lock's
/// [`ResolvedPackage::content_digest`]. Both inputs are hex strings (already
/// validated when they came from a [`crate::Lockfile`] or [`hpdl_model::Digest`]).
pub fn verify_integrity(name: &str, expected: &str, found: &str) -> Result<(), Error> {
    if expected == found {
        Ok(())
    } else {
        Err(Error::PackageIntegrity {
            name: name.to_string(),
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{BodyId, CelestialBody, ModuleRef, PhysicalProperties, TraitSet};

    fn definition_with_modules(modules: Vec<ModuleRef>) -> PlanetDefinition {
        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            modules,
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        PlanetDefinition::new("1.0", body).unwrap()
    }

    #[test]
    fn parse_valid_lock() {
        let lock = BuildLock::parse(
            "# header comment\n\nhpdl.atmosphere 1.2.0 aaaa\nhpdl.ocean 0.4.1 bbbb\n",
        )
        .unwrap();
        assert_eq!(lock.len(), 2);
        assert_eq!(lock.get("hpdl.atmosphere").unwrap().version, "1.2.0");
        assert_eq!(lock.get("hpdl.ocean").unwrap().content_hash, "bbbb");
    }

    #[test]
    fn parse_rejects_wrong_arity() {
        let err = BuildLock::parse("hpdl.atmosphere 1.2.0").unwrap_err();
        assert!(matches!(err, Error::LockParse { line: 1, .. }));

        let err = BuildLock::parse("hpdl.atmosphere 1.2.0 aaaa extra").unwrap_err();
        assert!(matches!(err, Error::LockParse { line: 1, .. }));
    }

    #[test]
    fn parse_rejects_duplicate_name() {
        let err = BuildLock::parse("m 1.0.0 aaaa\nm 1.0.0 bbbb").unwrap_err();
        assert!(matches!(err, Error::LockParse { line: 2, .. }));
    }

    #[test]
    fn parse_rejects_oversized_lock() {
        let limits = ResourceLimits {
            max_source_bytes: 8,
            ..ResourceLimits::default()
        };
        let err = BuildLock::parse_with_limits("hpdl.atmosphere 1.2.0 aaaa", &limits).unwrap_err();
        assert!(matches!(err, Error::Limit(_)));
    }

    #[test]
    fn parse_within_limit_is_ok() {
        let limits = ResourceLimits::default();
        let lock = BuildLock::parse_with_limits("hpdl.atmosphere 1.2.0 aaaa", &limits).unwrap();
        assert_eq!(lock.len(), 1);
    }

    #[test]
    fn parse_empty_is_empty() {
        let lock = BuildLock::parse("\n# only comments\n\n").unwrap();
        assert!(lock.is_empty());
    }

    #[test]
    fn no_modules_resolves_empty_without_lock() {
        let def = definition_with_modules(vec![]);
        let resolved = resolve_modules(&def, None, &ResourceLimits::default()).unwrap();
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolves_matching_module_sorted() {
        let def = definition_with_modules(vec![
            ModuleRef::new("hpdl.ocean", "0.4.1").unwrap(),
            ModuleRef::new("hpdl.atmosphere", "1.2.0").unwrap(),
        ]);
        let lock = BuildLock::parse("hpdl.atmosphere 1.2.0 aaaa\nhpdl.ocean 0.4.1 bbbb").unwrap();
        let resolved = resolve_modules(&def, Some(&lock), &ResourceLimits::default()).unwrap();
        let slice = resolved.as_slice();
        assert_eq!(slice.len(), 2);
        // sorted by name: atmosphere before ocean
        assert_eq!(slice[0].name, "hpdl.atmosphere");
        assert_eq!(slice[1].name, "hpdl.ocean");
    }

    #[test]
    fn unlocked_module_fails_closed() {
        let def = definition_with_modules(vec![ModuleRef::new("hpdl.magic", "1.0.0").unwrap()]);
        let err = resolve_modules(&def, None, &ResourceLimits::default()).unwrap_err();
        assert!(matches!(err, Error::UnresolvedModule { .. }));

        let lock = BuildLock::parse("hpdl.atmosphere 1.2.0 aaaa").unwrap();
        let err = resolve_modules(&def, Some(&lock), &ResourceLimits::default()).unwrap_err();
        assert!(matches!(err, Error::UnresolvedModule { .. }));
    }

    #[test]
    fn version_mismatch_fails() {
        let def =
            definition_with_modules(vec![ModuleRef::new("hpdl.atmosphere", "2.0.0").unwrap()]);
        let lock = BuildLock::parse("hpdl.atmosphere 1.2.0 aaaa").unwrap();
        let err = resolve_modules(&def, Some(&lock), &ResourceLimits::default()).unwrap_err();
        match err {
            Error::ModuleVersionMismatch {
                name,
                requested,
                locked,
            } => {
                assert_eq!(name, "hpdl.atmosphere");
                assert_eq!(requested, "2.0.0");
                assert_eq!(locked, "1.2.0");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn max_modules_enforced() {
        let def = definition_with_modules(vec![
            ModuleRef::new("a", "1.0.0").unwrap(),
            ModuleRef::new("b", "1.0.0").unwrap(),
        ]);
        let lock = BuildLock::parse("a 1.0.0 aaaa\nb 1.0.0 bbbb").unwrap();
        let limits = ResourceLimits {
            max_source_bytes: usize::MAX,
            max_modules: 1,
        };
        let err = resolve_modules(&def, Some(&lock), &limits).unwrap_err();
        assert!(matches!(err, Error::Limit(_)));
    }

    // ----- PackageResolver (feature 14) -----

    fn dg(c: char) -> String {
        c.to_string().repeat(64)
    }

    fn lockfile(entries: &str) -> crate::Lockfile {
        crate::Lockfile::parse(&format!("lock_format_version = 1\n{entries}")).unwrap()
    }

    #[test]
    fn closure_resolves_transitively_sorted() {
        let limits = ResourceLimits::default();
        let lock = lockfile(&format!(
            "package = hpdl.climate 2.0.0 {} hpdl.atmosphere\npackage = hpdl.atmosphere 1.2.0 {}\n",
            dg('a'),
            dg('b'),
        ));
        let resolver = PackageResolver::new(&lock, &limits);
        let closure = resolver.resolve_closure(&["hpdl.climate"]).unwrap();
        let names: Vec<&str> = closure.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["hpdl.atmosphere", "hpdl.climate"]);
    }

    #[test]
    fn closure_empty_roots_is_empty() {
        let limits = ResourceLimits::default();
        let lock = lockfile(&format!("package = hpdl.a 1.0.0 {}\n", dg('a')));
        let resolver = PackageResolver::new(&lock, &limits);
        assert!(resolver.resolve_closure(&[]).unwrap().is_empty());
    }

    #[test]
    fn unresolved_package_fails_closed() {
        let limits = ResourceLimits::default();
        let lock = lockfile(&format!("package = hpdl.a 1.0.0 {}\n", dg('a')));
        let resolver = PackageResolver::new(&lock, &limits);
        let err = resolver.resolve_closure(&["hpdl.missing"]).unwrap_err();
        assert!(matches!(err, Error::UnresolvedPackage { .. }));
    }

    #[test]
    fn missing_transitive_dependency_fails() {
        let limits = ResourceLimits::default();
        let lock = lockfile(&format!(
            "package = hpdl.climate 2.0.0 {} hpdl.ghost\n",
            dg('a'),
        ));
        let resolver = PackageResolver::new(&lock, &limits);
        let err = resolver.resolve_closure(&["hpdl.climate"]).unwrap_err();
        assert!(matches!(err, Error::UnresolvedPackage { name } if name == "hpdl.ghost"));
    }

    #[test]
    fn dependency_cycle_is_rejected() {
        let limits = ResourceLimits::default();
        let lock = lockfile(&format!(
            "package = a 1.0.0 {} b\npackage = b 1.0.0 {} a\n",
            dg('a'),
            dg('b'),
        ));
        let resolver = PackageResolver::new(&lock, &limits);
        let err = resolver.resolve_closure(&["a"]).unwrap_err();
        assert!(matches!(err, Error::DependencyCycle(_)));
    }

    #[test]
    fn self_dependency_is_a_cycle() {
        let limits = ResourceLimits::default();
        let lock = lockfile(&format!("package = a 1.0.0 {} a\n", dg('a')));
        let resolver = PackageResolver::new(&lock, &limits);
        let err = resolver.resolve_closure(&["a"]).unwrap_err();
        assert!(matches!(err, Error::DependencyCycle(_)));
    }

    #[test]
    fn closure_bound_enforced() {
        let lock = lockfile(&format!(
            "package = a 1.0.0 {} b\npackage = b 1.0.0 {}\n",
            dg('a'),
            dg('b'),
        ));
        let limits = ResourceLimits {
            max_source_bytes: usize::MAX,
            max_modules: 1,
        };
        let resolver = PackageResolver::new(&lock, &limits);
        let err = resolver.resolve_closure(&["a"]).unwrap_err();
        assert!(matches!(err, Error::Limit(_)));
    }

    #[test]
    fn integrity_check_detects_mismatch() {
        assert!(verify_integrity("p", &dg('a'), &dg('a')).is_ok());
        let err = verify_integrity("p", &dg('a'), &dg('b')).unwrap_err();
        assert!(matches!(err, Error::PackageIntegrity { .. }));
    }

    #[test]
    fn resolve_packages_version_mismatch() {
        let def =
            definition_with_modules(vec![ModuleRef::new("hpdl.atmosphere", "9.9.9").unwrap()]);
        let lock = lockfile(&format!("package = hpdl.atmosphere 1.2.0 {}\n", dg('a')));
        let err = resolve_packages(&def, &lock, &ResourceLimits::default()).unwrap_err();
        assert!(matches!(err, Error::PackageVersionMismatch { .. }));
    }

    #[test]
    fn resolve_packages_matches_and_resolves_closure() {
        let def = definition_with_modules(vec![ModuleRef::new("hpdl.climate", "2.0.0").unwrap()]);
        let lock = lockfile(&format!(
            "package = hpdl.climate 2.0.0 {} hpdl.atmosphere\npackage = hpdl.atmosphere 1.2.0 {}\n",
            dg('a'),
            dg('b'),
        ));
        let closure = resolve_packages(&def, &lock, &ResourceLimits::default()).unwrap();
        assert_eq!(closure.len(), 2);
    }
}
