//! RON loader for deserializing HPDL planet definitions.

use crate::{Error, ParsedDefinition, SecurityConfig, SourceDocument};
use hpdl_model::PlanetDefinition;
use std::path::Path;

/// RON loader for deserializing HPDL planet definitions from RON format.
///
/// The loader enforces security limits to protect against malicious input
/// and provides span tracking for diagnostic reporting.
///
/// # Example
///
/// ```no_run
/// use hpdl_ron::{RonLoader, SourceDocument};
///
/// # fn example() -> Result<(), hpdl_ron::Error> {
/// let source = SourceDocument::from_file("planets/earth.ron")?;
/// let loader = RonLoader::new();
/// let parsed = loader.parse(&source)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct RonLoader {
    security_config: SecurityConfig,
}

impl RonLoader {
    /// Create a new RON loader with default security configuration.
    pub fn new() -> Self {
        Self {
            security_config: SecurityConfig::new(),
        }
    }

    /// Create a new RON loader with custom security configuration.
    pub fn with_config(security_config: SecurityConfig) -> Self {
        Self { security_config }
    }

    /// Parse a RON source document into a `PlanetDefinition`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The RON syntax is invalid
    /// - The structure doesn't match `PlanetDefinition`
    /// - Security limits are exceeded
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse(&self, source: &SourceDocument) -> Result<ParsedDefinition, Error> {
        // Validate content size
        let content_size = source.content().len();
        self.security_config
            .validate_file_size(content_size as u64)?;

        // Parse RON content
        let definition: PlanetDefinition = ron::from_str(source.content())?;

        // Wrap in ParsedDefinition
        Ok(ParsedDefinition::new(definition, source.clone()))
    }

    /// Convenience method to load and parse a RON file in one step.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hpdl_ron::RonLoader;
    ///
    /// # fn example() -> Result<(), hpdl_ron::Error> {
    /// let loader = RonLoader::new();
    /// let parsed = loader.load_file("planets/earth.ron")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_file(&self, path: impl AsRef<Path>) -> Result<ParsedDefinition, Error> {
        let source = SourceDocument::from_file(path)?;
        self.parse(&source)
    }

    /// Convenience method to parse RON content from a string.
    ///
    /// # Arguments
    ///
    /// * `path` - A path or identifier for diagnostic purposes
    /// * `content` - The RON source text
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails or security limits are exceeded.
    ///
    /// # Example
    ///
    /// ```
    /// use hpdl_ron::RonLoader;
    ///
    /// # fn example() -> Result<(), hpdl_ron::Error> {
    /// let ron_content = r#"
    /// PlanetDefinition(
    ///     language_version: "1.0",
    ///     body: CelestialBody(
    ///         id: BodyId("earth"),
    ///         properties: PhysicalProperties(
    ///             mass_kg: Some(5972000000000000000000000),
    ///             radius_m: Some(6371000),
    ///             surface_gravity_mms2: None,
    ///             orbital_period_s: None,
    ///             rotation_period_s: None,
    ///         ),
    ///         traits: TraitSet([]),
    ///         modules: [],
    ///         statuses: [],
    ///         actions: [],
    ///         migrations: [],
    ///     )
    /// )
    /// "#;
    ///
    /// let loader = RonLoader::new();
    /// let parsed = loader.load_string("test.ron", ron_content.to_string())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_string(
        &self,
        path: impl Into<std::path::PathBuf>,
        content: String,
    ) -> Result<ParsedDefinition, Error> {
        let source = SourceDocument::from_string(path, content);
        self.parse(&source)
    }

    /// Get the security configuration used by this loader.
    pub fn security_config(&self) -> &SecurityConfig {
        &self.security_config
    }
}

impl Default for RonLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

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
    fn test_new_loader() {
        let loader = RonLoader::new();
        assert_eq!(
            loader.security_config().max_file_size(),
            SecurityConfig::DEFAULT_MAX_FILE_SIZE
        );
    }

    #[test]
    fn test_with_custom_config() {
        let config = SecurityConfig::builder().max_file_size(5_000_000).build();
        let loader = RonLoader::with_config(config);
        assert_eq!(loader.security_config().max_file_size(), 5_000_000);
    }

    #[test]
    fn test_parse_minimal_definition() {
        let loader = RonLoader::new();
        let source = SourceDocument::from_string("test.ron", minimal_ron().to_string());
        let result = loader.parse(&source);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.definition().language_version(), "1.0");
        assert_eq!(parsed.definition().body().id().as_str(), "test-planet");
    }

    #[test]
    fn test_load_string() {
        let loader = RonLoader::new();
        let result = loader.load_string("test.ron", minimal_ron().to_string());

        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_ron_syntax() {
        let loader = RonLoader::new();
        let invalid_ron = "this is not valid RON {{{";
        let source = SourceDocument::from_string("test.ron", invalid_ron.to_string());

        let result = loader.parse(&source);
        assert!(matches!(result, Err(Error::ParseWithSpan { .. })));
    }

    #[test]
    fn test_file_size_limit_exceeded() {
        let config = SecurityConfig::builder()
            .max_file_size(100) // Very small limit
            .build();
        let loader = RonLoader::with_config(config);

        let large_content = minimal_ron().to_string();
        let source = SourceDocument::from_string("test.ron", large_content);

        let result = loader.parse(&source);
        assert!(matches!(result, Err(Error::FileSizeExceeded { .. })));
    }

    #[test]
    fn test_default_trait() {
        let loader1 = RonLoader::default();
        let loader2 = RonLoader::new();

        assert_eq!(
            loader1.security_config().max_file_size(),
            loader2.security_config().max_file_size()
        );
    }
}
