use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::{CelestialBody, Error};

/// The root type representing a complete HPDL planet declaration file.
///
/// A `PlanetDefinition` pairs a `language_version` tag (e.g. `"1.0"`) with
/// a single [`CelestialBody`] declaration. It is an immutable artifact —
/// it never holds mutable runtime state.
///
/// # Example
///
/// ```
/// use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};
///
/// let body = CelestialBody::new(
///     BodyId::new("kepler-186f").unwrap(),
///     PhysicalProperties::default(),
///     TraitSet::empty(),
///     vec![], vec![], vec![], vec![],
/// ).unwrap();
///
/// let def = PlanetDefinition::new("1.0", body).unwrap();
/// assert_eq!(def.language_version(), "1.0");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct PlanetDefinition {
    language_version: String,
    body: CelestialBody,
}

impl PlanetDefinition {
    /// Construct a `PlanetDefinition`.
    ///
    /// Returns [`Error::InvalidLanguageVersion`] if `language_version` is
    /// empty or longer than 32 characters.
    pub fn new(language_version: impl Into<String>, body: CelestialBody) -> Result<Self, Error> {
        let language_version = language_version.into();
        if language_version.is_empty() || language_version.len() > 32 {
            return Err(Error::InvalidLanguageVersion(language_version));
        }
        Ok(Self {
            language_version,
            body,
        })
    }

    /// Return the declared HPDL language version.
    pub fn language_version(&self) -> &str {
        &self.language_version
    }

    /// Return a reference to the celestial body declaration.
    pub fn body(&self) -> &CelestialBody {
        &self.body
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::{BodyId, PhysicalProperties, TraitSet};

    fn minimal_body() -> CelestialBody {
        CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn valid_definition() {
        let def = PlanetDefinition::new("1.0", minimal_body()).unwrap();
        assert_eq!(def.language_version(), "1.0");
    }

    #[test]
    fn empty_version_invalid() {
        assert!(matches!(
            PlanetDefinition::new("", minimal_body()),
            Err(Error::InvalidLanguageVersion(_))
        ));
    }

    #[test]
    fn version_too_long_invalid() {
        let long = "1".repeat(33);
        assert!(matches!(
            PlanetDefinition::new(long, minimal_body()),
            Err(Error::InvalidLanguageVersion(_))
        ));
    }

    #[test]
    fn max_length_version_ok() {
        let exactly_32 = "1".repeat(32);
        assert!(PlanetDefinition::new(exactly_32, minimal_body()).is_ok());
    }
}
