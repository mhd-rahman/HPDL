//! Compilation configuration.
//!
//! [`CompileOptions`] and [`ResourceLimits`] carry only explicit, deterministic
//! settings. Nothing here reads ambient configuration (environment, clock,
//! filesystem), so a given set of options always drives the same behavior.

use crate::lockfile::Lockfile;
use crate::resolve::BuildLock;
use hpdl_sema::ValidatorConfig;

/// Resource limits enforced before and during compilation.
///
/// Limits bound the work done on untrusted input so a malicious source or
/// lockfile cannot cause unbounded allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Maximum accepted source size in bytes.
    ///
    /// Defaults to 10 MiB, matching `hpdl_pir`'s maximum PIR payload size so
    /// the frontend and backend agree on the upper bound.
    pub max_source_bytes: usize,

    /// Maximum number of modules in the resolved closure.
    ///
    /// Defaults to 1024. Resolution fails with [`crate::Error::Limit`] if the
    /// resolved count exceeds this.
    pub max_modules: usize,
}

impl ResourceLimits {
    /// Default maximum source size (10 MiB).
    pub const DEFAULT_MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;
    /// Default maximum resolved-module count.
    pub const DEFAULT_MAX_MODULES: usize = 1024;
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: Self::DEFAULT_MAX_SOURCE_BYTES,
            max_modules: Self::DEFAULT_MAX_MODULES,
        }
    }
}

/// Configuration for a compilation run.
///
/// Construct with [`CompileOptions::default`] for strict, deterministic
/// defaults, then override fields as needed. The `Default` carries no
/// lockfile, so any definition that declares module references will fail
/// resolution until a [`BuildLock`] is supplied.
#[derive(Clone, Debug, Default)]
pub struct CompileOptions {
    /// Configuration for the shared semantic validator.
    pub validator: ValidatorConfig,

    /// Optional provisional (feature-06) lockfile pinning module versions and
    /// content hashes.
    pub lock: Option<BuildLock>,

    /// Optional authoritative (feature-14) package lockfile pinning the full
    /// transitive closure with content digests. When present, the package closure
    /// is resolved and verified in addition to provisional module resolution.
    pub package_lock: Option<Lockfile>,

    /// Resource limits enforced during compilation.
    pub limits: ResourceLimits,
}

impl CompileOptions {
    /// Construct options with the given provisional lockfile, keeping other defaults.
    #[must_use]
    pub fn with_lock(mut self, lock: BuildLock) -> Self {
        self.lock = Some(lock);
        self
    }

    /// Construct options with the given authoritative package lockfile, keeping
    /// other defaults.
    #[must_use]
    pub fn with_package_lock(mut self, lock: Lockfile) -> Self {
        self.package_lock = Some(lock);
        self
    }

    /// Construct options with the given resource limits, keeping other defaults.
    #[must_use]
    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn default_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_source_bytes, 10 * 1024 * 1024);
        assert_eq!(limits.max_modules, 1024);
    }

    #[test]
    fn default_options_have_no_lock() {
        let options = CompileOptions::default();
        assert!(options.lock.is_none());
        assert!(options.package_lock.is_none());
        assert_eq!(options.limits, ResourceLimits::default());
        assert_eq!(options.validator, ValidatorConfig::default());
    }

    #[test]
    fn with_limits_overrides() {
        let limits = ResourceLimits {
            max_source_bytes: 1024,
            max_modules: 4,
        };
        let options = CompileOptions::default().with_limits(limits);
        assert_eq!(options.limits.max_source_bytes, 1024);
        assert_eq!(options.limits.max_modules, 4);
    }
}
