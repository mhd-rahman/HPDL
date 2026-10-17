//! Unit and dimension validation registry.

use crate::{Diagnostic, DiagnosticCode};
use std::collections::BTreeMap;

/// Specification for a custom dimension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionSpec {
    /// Dimension name (e.g., "Aether").
    pub name: String,
    /// Namespace (e.g., "arcane").
    pub namespace: String,
    /// Version string.
    pub version: String,
    /// Base unit name.
    pub base_unit: String,
}

/// Registry for validating units and dimensions.
pub struct UnitRegistry {
    custom_dimensions: BTreeMap<String, DimensionSpec>,
}

impl UnitRegistry {
    /// Create a new empty unit registry.
    pub fn new() -> Self {
        Self {
            custom_dimensions: BTreeMap::new(),
        }
    }

    /// Register a custom dimension.
    ///
    /// Returns an error if a dimension with the same fully-qualified name is already registered.
    pub fn register_custom_dimension(&mut self, spec: DimensionSpec) -> Result<(), crate::Error> {
        let fq_name = format!("{}::{}", spec.namespace, spec.name);

        if self.custom_dimensions.contains_key(&fq_name) {
            return Err(crate::Error::Generic(format!(
                "Dimension '{}' is already registered",
                fq_name
            )));
        }

        self.custom_dimensions.insert(fq_name, spec);
        Ok(())
    }

    /// Validate that a numeric value is valid (not NaN or infinity).
    pub fn validate_numeric(&self, value: f64) -> Option<Diagnostic> {
        self.check_no_nan_inf(value)
    }

    /// Check if a custom dimension is registered.
    pub fn check_custom_dimension(&self, namespace: &str, name: &str) -> Option<Diagnostic> {
        let fq_name = format!("{}::{}", namespace, name);

        if !self.custom_dimensions.contains_key(&fq_name) {
            Some(
                Diagnostic::error(
                    DiagnosticCode::UnregisteredCustomDimension,
                    format!("Custom dimension '{}' is not registered", fq_name),
                )
                .with_note(
                    "Register this dimension before use, or use a standard SI dimension"
                        .to_string(),
                ),
            )
        } else {
            None
        }
    }

    /// Get a registered custom dimension spec.
    pub fn get_dimension(&self, namespace: &str, name: &str) -> Option<&DimensionSpec> {
        let fq_name = format!("{}::{}", namespace, name);
        self.custom_dimensions.get(&fq_name)
    }

    /// List all registered custom dimensions.
    pub fn list_dimensions(&self) -> impl Iterator<Item = &DimensionSpec> {
        self.custom_dimensions.values()
    }

    // Internal helpers

    fn check_no_nan_inf(&self, value: f64) -> Option<Diagnostic> {
        if value.is_nan() {
            Some(Diagnostic::error(
                DiagnosticCode::InvalidNumericValue,
                "Value is NaN (not a number)".to_string(),
            ))
        } else if value.is_infinite() {
            Some(Diagnostic::error(
                DiagnosticCode::InvalidNumericValue,
                format!("Value is infinite: {}", value),
            ))
        } else {
            None
        }
    }
}

impl Default for UnitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = UnitRegistry::new();
        assert_eq!(registry.custom_dimensions.len(), 0);
    }

    #[test]
    fn test_register_custom_dimension() {
        let mut registry = UnitRegistry::new();

        let spec = DimensionSpec {
            name: "Aether".to_string(),
            namespace: "arcane".to_string(),
            version: "1.0".to_string(),
            base_unit: "aether_unit".to_string(),
        };

        assert!(registry.register_custom_dimension(spec).is_ok());
        assert_eq!(registry.custom_dimensions.len(), 1);
    }

    #[test]
    fn test_duplicate_dimension_rejected() {
        let mut registry = UnitRegistry::new();

        let spec1 = DimensionSpec {
            name: "Aether".to_string(),
            namespace: "arcane".to_string(),
            version: "1.0".to_string(),
            base_unit: "aether_unit".to_string(),
        };

        let spec2 = DimensionSpec {
            name: "Aether".to_string(),
            namespace: "arcane".to_string(),
            version: "2.0".to_string(),
            base_unit: "aether_unit_v2".to_string(),
        };

        assert!(registry.register_custom_dimension(spec1).is_ok());
        assert!(registry.register_custom_dimension(spec2).is_err());
    }

    #[test]
    fn test_check_custom_dimension_unregistered() {
        let registry = UnitRegistry::new();

        let diag = registry.check_custom_dimension("arcane", "Aether");
        assert!(diag.is_some());

        if let Some(d) = diag {
            assert_eq!(d.code, DiagnosticCode::UnregisteredCustomDimension);
            assert!(d.message.contains("arcane::Aether"));
        }
    }

    #[test]
    fn test_check_custom_dimension_registered() {
        let mut registry = UnitRegistry::new();

        let spec = DimensionSpec {
            name: "Aether".to_string(),
            namespace: "arcane".to_string(),
            version: "1.0".to_string(),
            base_unit: "aether_unit".to_string(),
        };

        registry.register_custom_dimension(spec).unwrap();

        let diag = registry.check_custom_dimension("arcane", "Aether");
        assert!(diag.is_none());
    }

    #[test]
    fn test_get_dimension() {
        let mut registry = UnitRegistry::new();

        let spec = DimensionSpec {
            name: "Aether".to_string(),
            namespace: "arcane".to_string(),
            version: "1.0".to_string(),
            base_unit: "aether_unit".to_string(),
        };

        registry.register_custom_dimension(spec.clone()).unwrap();

        let retrieved = registry.get_dimension("arcane", "Aether");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &spec);
    }

    #[test]
    fn test_validate_numeric_normal() {
        let registry = UnitRegistry::new();
        assert!(registry.validate_numeric(42.0).is_none());
        assert!(registry.validate_numeric(-273.15).is_none());
        assert!(registry.validate_numeric(0.0).is_none());
        assert!(registry.validate_numeric(1.23e45).is_none());
    }

    #[test]
    fn test_validate_numeric_nan() {
        let registry = UnitRegistry::new();
        let diag = registry.validate_numeric(f64::NAN);
        assert!(diag.is_some());

        if let Some(d) = diag {
            assert_eq!(d.code, DiagnosticCode::InvalidNumericValue);
            assert!(d.message.contains("NaN"));
        }
    }

    #[test]
    fn test_validate_numeric_infinity() {
        let registry = UnitRegistry::new();

        let diag_pos = registry.validate_numeric(f64::INFINITY);
        assert!(diag_pos.is_some());

        let diag_neg = registry.validate_numeric(f64::NEG_INFINITY);
        assert!(diag_neg.is_some());

        if let Some(d) = diag_pos {
            assert_eq!(d.code, DiagnosticCode::InvalidNumericValue);
            assert!(d.message.contains("infinite"));
        }
    }

    #[test]
    fn test_list_dimensions() {
        let mut registry = UnitRegistry::new();

        let spec1 = DimensionSpec {
            name: "Aether".to_string(),
            namespace: "arcane".to_string(),
            version: "1.0".to_string(),
            base_unit: "aether_unit".to_string(),
        };

        let spec2 = DimensionSpec {
            name: "Mana".to_string(),
            namespace: "magic".to_string(),
            version: "1.0".to_string(),
            base_unit: "mana_point".to_string(),
        };

        registry.register_custom_dimension(spec1).unwrap();
        registry.register_custom_dimension(spec2).unwrap();

        let dimensions: Vec<_> = registry.list_dimensions().collect();
        assert_eq!(dimensions.len(), 2);
    }

    #[test]
    fn test_different_namespaces_allowed() {
        let mut registry = UnitRegistry::new();

        let spec1 = DimensionSpec {
            name: "Energy".to_string(),
            namespace: "arcane".to_string(),
            version: "1.0".to_string(),
            base_unit: "arcane_energy".to_string(),
        };

        let spec2 = DimensionSpec {
            name: "Energy".to_string(),
            namespace: "divine".to_string(),
            version: "1.0".to_string(),
            base_unit: "divine_energy".to_string(),
        };

        assert!(registry.register_custom_dimension(spec1).is_ok());
        assert!(registry.register_custom_dimension(spec2).is_ok());
        assert_eq!(registry.custom_dimensions.len(), 2);
    }
}
