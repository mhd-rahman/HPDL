use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::Error;

const MAX_NAME_LEN: usize = 128;
const MAX_VERSION_LEN: usize = 64;

/// A reference to a versioned external module.
///
/// Module references are purely declarative — they name a module and a
/// semver-compatible version string. Resolution against the lockfile and
/// registry happens in a later pipeline stage.
///
/// # Example
///
/// ```
/// use hpdl_model::ModuleRef;
///
/// let r = ModuleRef::new("hpdl.atmosphere", "1.2.0").unwrap();
/// assert_eq!(r.name(), "hpdl.atmosphere");
/// assert_eq!(r.version(), "1.2.0");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct ModuleRef {
    name: String,
    version: String,
}

impl ModuleRef {
    /// Construct a `ModuleRef`, validating both fields.
    ///
    /// Returns [`Error::InvalidModuleRef`] if either field is empty or
    /// exceeds its length limit (name: 128, version: 64).
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        let version = version.into();
        if name.is_empty() || name.len() > MAX_NAME_LEN {
            return Err(Error::InvalidModuleRef);
        }
        if version.is_empty() || version.len() > MAX_VERSION_LEN {
            return Err(Error::InvalidModuleRef);
        }
        Ok(Self { name, version })
    }

    /// Return the module name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the version string.
    pub fn version(&self) -> &str {
        &self.version
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn valid_ref() {
        let r = ModuleRef::new("hpdl.atmosphere", "1.0.0").unwrap();
        assert_eq!(r.name(), "hpdl.atmosphere");
        assert_eq!(r.version(), "1.0.0");
    }

    #[test]
    fn empty_name() {
        assert_eq!(ModuleRef::new("", "1.0.0"), Err(Error::InvalidModuleRef));
    }

    #[test]
    fn empty_version() {
        assert_eq!(ModuleRef::new("mod", ""), Err(Error::InvalidModuleRef));
    }

    #[test]
    fn name_too_long() {
        let long = "a".repeat(129);
        assert_eq!(
            ModuleRef::new(long.as_str(), "1.0.0"),
            Err(Error::InvalidModuleRef)
        );
    }

    #[test]
    fn version_too_long() {
        let long = "1".repeat(65);
        assert_eq!(
            ModuleRef::new("mod", long.as_str()),
            Err(Error::InvalidModuleRef)
        );
    }
}
