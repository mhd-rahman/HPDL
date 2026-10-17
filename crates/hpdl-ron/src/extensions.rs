//! Extension field support for RON format.
//!
//! Extension fields allow RON files to include custom metadata without
//! affecting validation. They must use a namespaced prefix to distinguish
//! them from standard HPDL fields.

/// Extension namespace configuration.
///
/// Extension fields must be prefixed with a namespace to avoid conflicts
/// with standard HPDL fields. The default namespace is `_ext::`.
///
/// # Example
///
/// ```ron
/// PlanetDefinition(
///     language_version: "1.0",
///     body: CelestialBody(
///         id: BodyId("earth"),
///         physical: PhysicalProperties(
///             mass_kg: 5972000000000000000000000,
///             radius_m: 6371000,
///         ),
///         traits: TraitSet(tags: []),
///         modules: [],
///         statuses: [],
///         actions: [],
///         migrations: [],
///         // Extension field with custom metadata
///         _ext::author: "Jane Doe",
///         _ext::created: "2024-01-15",
///     )
/// )
/// ```
#[derive(Debug, Clone)]
pub struct ExtensionConfig {
    /// The namespace prefix for extension fields (default: "_ext::").
    namespace_prefix: String,

    /// Whether to allow unknown fields with the extension prefix.
    allow_extensions: bool,
}

impl ExtensionConfig {
    /// Default extension namespace prefix.
    pub const DEFAULT_PREFIX: &'static str = "_ext::";

    /// Create a new extension configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an extension configuration with a custom namespace prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The namespace prefix (e.g., "_ext::", "@ext/", "x-")
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            namespace_prefix: prefix.into(),
            allow_extensions: true,
        }
    }

    /// Enable or disable extension fields.
    #[must_use]
    pub fn allow_extensions(mut self, allow: bool) -> Self {
        self.allow_extensions = allow;
        self
    }

    /// Get the namespace prefix.
    pub fn namespace_prefix(&self) -> &str {
        &self.namespace_prefix
    }

    /// Check if extension fields are allowed.
    pub fn extensions_allowed(&self) -> bool {
        self.allow_extensions
    }

    /// Check if a field name is an extension field.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The field name to check
    ///
    /// # Returns
    ///
    /// `true` if the field starts with the extension namespace prefix.
    pub fn is_extension_field(&self, field_name: &str) -> bool {
        field_name.starts_with(&self.namespace_prefix)
    }

    /// Validate an extension field name.
    ///
    /// Extension field names must:
    /// - Start with the namespace prefix
    /// - Contain only alphanumeric characters, underscores, and hyphens after the prefix
    /// - Not be empty after the prefix
    ///
    /// # Arguments
    ///
    /// * `field_name` - The field name to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err(String)` with error message if invalid.
    pub fn validate_extension_field(&self, field_name: &str) -> Result<(), String> {
        if !self.is_extension_field(field_name) {
            return Err(format!(
                "Extension field must start with '{}', got: '{}'",
                self.namespace_prefix, field_name
            ));
        }

        let name_part = &field_name[self.namespace_prefix.len()..];

        if name_part.is_empty() {
            return Err(format!(
                "Extension field name cannot be empty after prefix '{}'",
                self.namespace_prefix
            ));
        }

        // Validate characters: alphanumeric, underscore, hyphen
        if !name_part
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(format!(
                "Extension field name '{}' contains invalid characters. Only alphanumeric, '_', and '-' allowed.",
                field_name
            ));
        }

        Ok(())
    }
}

impl Default for ExtensionConfig {
    fn default() -> Self {
        Self {
            namespace_prefix: Self::DEFAULT_PREFIX.to_string(),
            allow_extensions: true,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ExtensionConfig::new();
        assert_eq!(config.namespace_prefix(), "_ext::");
        assert!(config.extensions_allowed());
    }

    #[test]
    fn test_custom_prefix() {
        let config = ExtensionConfig::with_prefix("@ext/");
        assert_eq!(config.namespace_prefix(), "@ext/");
        assert!(config.extensions_allowed());
    }

    #[test]
    fn test_disable_extensions() {
        let config = ExtensionConfig::new().allow_extensions(false);
        assert!(!config.extensions_allowed());
    }

    #[test]
    fn test_is_extension_field() {
        let config = ExtensionConfig::new();
        assert!(config.is_extension_field("_ext::author"));
        assert!(config.is_extension_field("_ext::created_at"));
        assert!(!config.is_extension_field("author"));
        assert!(!config.is_extension_field("physical"));
    }

    #[test]
    fn test_validate_valid_extension_field() {
        let config = ExtensionConfig::new();
        assert!(config.validate_extension_field("_ext::author").is_ok());
        assert!(config.validate_extension_field("_ext::created_at").is_ok());
        assert!(config.validate_extension_field("_ext::version-2").is_ok());
    }

    #[test]
    fn test_validate_missing_prefix() {
        let config = ExtensionConfig::new();
        let result = config.validate_extension_field("author");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must start with"));
    }

    #[test]
    fn test_validate_empty_name() {
        let config = ExtensionConfig::new();
        let result = config.validate_extension_field("_ext::");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_invalid_characters() {
        let config = ExtensionConfig::new();
        let result = config.validate_extension_field("_ext::invalid.name");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid characters"));
    }

    #[test]
    fn test_custom_prefix_validation() {
        let config = ExtensionConfig::with_prefix("x-");
        assert!(config.is_extension_field("x-custom"));
        assert!(config.validate_extension_field("x-custom").is_ok());
        assert!(!config.is_extension_field("_ext::custom"));
    }
}
