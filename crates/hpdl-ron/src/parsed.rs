//! Parsed definition wrapper with source context.

use crate::{Error, SourceDocument};
use hpdl_model::PlanetDefinition;
use hpdl_sema::{ValidatedDefinition, Validator, ValidatorConfig};

/// A parsed planet definition with source document context.
///
/// This wrapper holds the deserialized `PlanetDefinition` along with
/// its source document for diagnostic reporting.
#[derive(Debug, Clone)]
pub struct ParsedDefinition {
    definition: PlanetDefinition,
    source: SourceDocument,
}

impl ParsedDefinition {
    /// Create a new parsed definition.
    ///
    /// # Arguments
    ///
    /// * `definition` - The deserialized planet definition
    /// * `source` - The source document it came from
    pub fn new(definition: PlanetDefinition, source: SourceDocument) -> Self {
        Self { definition, source }
    }

    /// Get a reference to the planet definition.
    pub fn definition(&self) -> &PlanetDefinition {
        &self.definition
    }

    /// Get a reference to the source document.
    pub fn source(&self) -> &SourceDocument {
        &self.source
    }

    /// Consume this parsed definition and extract the inner `PlanetDefinition`.
    pub fn into_definition(self) -> PlanetDefinition {
        self.definition
    }

    /// Validate this parsed definition through the shared semantic validator.
    ///
    /// # Errors
    ///
    /// Returns an error if semantic validation fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hpdl_ron::{RonLoader, SourceDocument};
    ///
    /// # fn example() -> Result<(), hpdl_ron::Error> {
    /// let source = SourceDocument::from_file("planet.ron")?;
    /// let loader = RonLoader::new();
    /// let parsed = loader.parse(&source)?;
    ///
    /// // Validate through shared pipeline
    /// let validated = parsed.validate()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(self) -> Result<ValidatedDefinition, Error> {
        let config = ValidatorConfig::default();
        let validator = Validator::new(config);

        match validator.validate(self.definition) {
            Ok(validated) => Ok(validated),
            Err(diagnostics) => {
                // Convert diagnostics to an error
                // For now, just return the first error message
                let error_msg = diagnostics
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                Err(Error::security(format!(
                    "Validation failed:\n{}",
                    error_msg
                )))
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::RonLoader;

    fn minimal_ron() -> &'static str {
        r#"
        PlanetDefinition(
            language_version: "1.0",
            body: CelestialBody(
                id: BodyId("test-planet"),
                properties: PhysicalProperties(
                    mass_kg: Some(1000000000000000000000000),
                    radius_m: Some(1000000),
                    surface_gravity_mms2: None,
                    orbital_period_s: None,
                    rotation_period_s: None,
                ),
                traits: TraitSet([]),
                modules: [],
                statuses: [],
                actions: [],
                migrations: [],
            )
        )
        "#
    }

    #[test]
    fn test_new_parsed_definition() {
        let loader = RonLoader::new();
        let source = SourceDocument::from_string("test.ron", minimal_ron().to_string());
        let parsed = loader.parse(&source).unwrap();

        assert_eq!(parsed.definition().language_version(), "1.0");
        assert_eq!(parsed.source().path().to_str().unwrap(), "test.ron");
    }

    #[test]
    fn test_into_definition() {
        let loader = RonLoader::new();
        let source = SourceDocument::from_string("test.ron", minimal_ron().to_string());
        let parsed = loader.parse(&source).unwrap();

        let definition = parsed.into_definition();
        assert_eq!(definition.language_version(), "1.0");
    }

    #[test]
    fn test_validate() {
        let loader = RonLoader::new();
        let source = SourceDocument::from_string("test.ron", minimal_ron().to_string());
        let parsed = loader.parse(&source).unwrap();

        let result = parsed.validate();
        assert!(result.is_ok());
    }
}
