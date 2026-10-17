use serde::{Deserialize, Serialize};

use crate::{status_declaration::validate_name, Error};

/// A declaration that a named forward-only schema migration exists.
///
/// Migrations are atomic, forward-only transitions between schema versions.
/// Downgrade (`to_version <= from_version`) is rejected at the model level.
///
/// # Example
///
/// ```
/// use hpdl_model::MigrationDeclaration;
///
/// let m = MigrationDeclaration::new("AddOceanField", 1, 2).unwrap();
/// assert_eq!(m.name(), "AddOceanField");
/// assert_eq!(m.from_version(), 1);
/// assert_eq!(m.to_version(), 2);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationDeclaration {
    name: String,
    from_version: u32,
    to_version: u32,
}

impl MigrationDeclaration {
    /// Construct a `MigrationDeclaration`.
    ///
    /// Returns [`Error::InvalidName`] if the name is invalid.
    /// Returns [`Error::InvalidMigrationVersions`] if `to_version <= from_version`.
    pub fn new(name: impl Into<String>, from_version: u32, to_version: u32) -> Result<Self, Error> {
        let name = name.into();
        validate_name(&name)?;
        if to_version <= from_version {
            return Err(Error::InvalidMigrationVersions {
                from: from_version,
                to: to_version,
            });
        }
        Ok(Self {
            name,
            from_version,
            to_version,
        })
    }

    /// Return the migration name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the source schema version.
    pub fn from_version(&self) -> u32 {
        self.from_version
    }

    /// Return the target schema version.
    pub fn to_version(&self) -> u32 {
        self.to_version
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn valid_migration() {
        let m = MigrationDeclaration::new("AddOcean", 1, 2).unwrap();
        assert_eq!(m.from_version(), 1);
        assert_eq!(m.to_version(), 2);
    }

    #[test]
    fn equal_versions_invalid() {
        assert!(matches!(
            MigrationDeclaration::new("M", 3, 3),
            Err(Error::InvalidMigrationVersions { from: 3, to: 3 })
        ));
    }

    #[test]
    fn downgrade_invalid() {
        assert!(matches!(
            MigrationDeclaration::new("M", 5, 2),
            Err(Error::InvalidMigrationVersions { from: 5, to: 2 })
        ));
    }

    #[test]
    fn empty_name_invalid() {
        assert!(matches!(
            MigrationDeclaration::new("", 0, 1),
            Err(Error::InvalidName(_))
        ));
    }

    #[test]
    fn large_version_jump() {
        assert!(MigrationDeclaration::new("BigJump", 1, 100).is_ok());
    }
}
