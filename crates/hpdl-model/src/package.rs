//! Standard-library package identity and manifest declarations (HPDL feature 14).
//!
//! These are *source-independent declarations*: a [`PackageId`] names an immutable,
//! content-addressed, independently versioned package by an **exact** version (never
//! a mutable label), and a [`PackageManifest`] records a package's content digest and
//! its declared direct dependencies. Resolution of the transitive closure against a
//! lockfile — with integrity and cycle checks — lives in `hpdl-compiler`.
//!
//! This module performs no I/O, fetches no bytes, and resolves nothing.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Digest, Error};

const MAX_PACKAGE_NAME_LEN: usize = 128;
const MAX_PACKAGE_VERSION_LEN: usize = 64;

/// Maximum number of direct dependencies a single [`PackageManifest`] may declare.
pub const MAX_PACKAGE_DEPS: usize = 1024;

/// An immutable package identity: a namespaced name and an **exact** pinned version.
///
/// A mutable label (`latest`, a range like `^1.2`, `~1.2`, `>=1`, `*`, or an empty
/// version) is rejected — the standard library never resolves a mutable reference
/// (architecture steering). Only an exact version (e.g. `1.2.0`, `1.2.0-rc.1`) is
/// accepted.
///
/// # Example
///
/// ```
/// use hpdl_model::PackageId;
///
/// let p = PackageId::new("hpdl.atmosphere", "1.2.0").unwrap();
/// assert_eq!(p.name(), "hpdl.atmosphere");
/// assert_eq!(p.version(), "1.2.0");
///
/// assert!(PackageId::new("hpdl.atmosphere", "latest").is_err());
/// assert!(PackageId::new("hpdl.atmosphere", "^1.2").is_err());
/// assert!(PackageId::new("hpdl.atmosphere", "").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PackageId {
    name: String,
    version: String,
}

impl PackageId {
    /// Construct a validated `PackageId`.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidModuleRef`] if the name is empty or longer than 128 chars,
    ///   or the version is longer than 64 chars.
    /// - [`Error::MutablePackageVersion`] if the version is empty or a mutable label
    ///   (`latest`, a leading range operator `^~><=*`, or embedded whitespace).
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        let version = version.into();
        if name.is_empty() || name.len() > MAX_PACKAGE_NAME_LEN {
            return Err(Error::InvalidModuleRef);
        }
        if version.len() > MAX_PACKAGE_VERSION_LEN {
            return Err(Error::InvalidModuleRef);
        }
        if !Self::is_exact_version(&version) {
            return Err(Error::MutablePackageVersion(version));
        }
        Ok(Self { name, version })
    }

    /// Whether `version` is an exact pin rather than a mutable label.
    fn is_exact_version(version: &str) -> bool {
        if version.is_empty() || version.eq_ignore_ascii_case("latest") {
            return false;
        }
        // No whitespace anywhere.
        if version.chars().any(|c| c.is_whitespace()) {
            return false;
        }
        // No leading range / wildcard operator.
        match version.chars().next() {
            Some('^') | Some('~') | Some('>') | Some('<') | Some('=') | Some('*') => false,
            _ => !version.contains('*'),
        }
    }

    /// The package name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The exact pinned version.
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// A package's content digest and declared direct dependencies.
///
/// A *declaration* only: it records what a package *is* and what it *depends on*;
/// it fetches no bytes and resolves nothing. The dependency set is deduplicated and
/// kept in total ([`BTreeSet`]) order so a manifest contributes deterministic bytes.
///
/// # Example
///
/// ```
/// use hpdl_model::{Digest, PackageId, PackageManifest};
///
/// let m = PackageManifest::new(
///     PackageId::new("hpdl.climate", "2.0.0").unwrap(),
///     Digest::new("a".repeat(64)).unwrap(),
///     [PackageId::new("hpdl.atmosphere", "1.2.0").unwrap()],
/// ).unwrap();
/// assert_eq!(m.dependencies().count(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    id: PackageId,
    content_digest: Digest,
    dependencies: BTreeSet<PackageId>,
}

impl PackageManifest {
    /// Construct a validated `PackageManifest`.
    ///
    /// # Errors
    ///
    /// - [`Error::SelfDependency`] if the package lists itself as a dependency.
    /// - [`Error::TooManyDependencies`] if more than [`MAX_PACKAGE_DEPS`] distinct
    ///   dependencies are declared.
    pub fn new(
        id: PackageId,
        content_digest: Digest,
        dependencies: impl IntoIterator<Item = PackageId>,
    ) -> Result<Self, Error> {
        let mut deps: BTreeSet<PackageId> = BTreeSet::new();
        for dep in dependencies {
            if dep.name() == id.name() {
                return Err(Error::SelfDependency(id.name().to_string()));
            }
            deps.insert(dep);
            if deps.len() > MAX_PACKAGE_DEPS {
                return Err(Error::TooManyDependencies(deps.len()));
            }
        }
        Ok(Self {
            id,
            content_digest,
            dependencies: deps,
        })
    }

    /// The package identity.
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    /// The package content digest.
    pub fn content_digest(&self) -> &Digest {
        &self.content_digest
    }

    /// The declared direct dependencies, in total order.
    pub fn dependencies(&self) -> impl Iterator<Item = &PackageId> {
        self.dependencies.iter()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn exact_versions_accepted() {
        assert!(PackageId::new("hpdl.a", "1.2.0").is_ok());
        assert!(PackageId::new("hpdl.a", "1.2.0-rc.1").is_ok());
        assert!(PackageId::new("hpdl.a", "0.0.1").is_ok());
    }

    #[test]
    fn mutable_labels_rejected() {
        for bad in [
            "", "latest", "LATEST", "^1.2", "~1.2", ">=1.0", "<2", "*", "1.*", "1 .0",
        ] {
            assert!(
                matches!(
                    PackageId::new("hpdl.a", bad),
                    Err(Error::MutablePackageVersion(_)) | Err(Error::InvalidModuleRef)
                ),
                "expected rejection for {bad:?}"
            );
        }
    }

    #[test]
    fn empty_name_rejected() {
        assert_eq!(PackageId::new("", "1.0.0"), Err(Error::InvalidModuleRef));
    }

    #[test]
    fn manifest_dedups_and_orders_deps() {
        let m = PackageManifest::new(
            PackageId::new("hpdl.root", "1.0.0").unwrap(),
            Digest::new("a".repeat(64)).unwrap(),
            [
                PackageId::new("hpdl.b", "2.0.0").unwrap(),
                PackageId::new("hpdl.a", "1.0.0").unwrap(),
                PackageId::new("hpdl.b", "2.0.0").unwrap(), // duplicate
            ],
        )
        .unwrap();
        let names: Vec<&str> = m.dependencies().map(PackageId::name).collect();
        assert_eq!(names, vec!["hpdl.a", "hpdl.b"]); // deduped + sorted
    }

    #[test]
    fn self_dependency_rejected() {
        let err = PackageManifest::new(
            PackageId::new("hpdl.root", "1.0.0").unwrap(),
            Digest::new("a".repeat(64)).unwrap(),
            [PackageId::new("hpdl.root", "9.9.9").unwrap()],
        )
        .unwrap_err();
        assert!(matches!(err, Error::SelfDependency(_)));
    }
}
