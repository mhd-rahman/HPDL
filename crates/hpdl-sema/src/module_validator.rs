//! Module reference and world law validation.

use crate::{Diagnostic, DiagnosticCode};
use hpdl_model::PlanetDefinition;
use std::collections::BTreeSet;

/// Validator for module references and world laws.
pub struct ModuleValidator;

impl ModuleValidator {
    /// Create a new module validator.
    pub fn new() -> Self {
        Self
    }

    /// Validate all module references.
    pub fn validate_modules(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let body = definition.body();

        // Track module identifiers for duplicate detection
        let mut seen_modules = BTreeSet::new();

        for module_ref in body.modules() {
            // Validate module name format (use name() not identifier())
            let module_name = module_ref.name();

            if module_name.is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::MalformedModuleId,
                    "Module name cannot be empty".to_string(),
                ));
                continue;
            }

            if module_name.len() > 256 {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::MalformedModuleId,
                    format!(
                        "Module name '{}' exceeds maximum length of 256",
                        module_name
                    ),
                ));
            }

            // Check for valid characters (alphanumeric, hyphen, underscore, colon, slash)
            if !module_name.chars().all(|c| {
                c.is_ascii_alphanumeric()
                    || c == '-'
                    || c == '_'
                    || c == ':'
                    || c == '/'
                    || c == '.'
            }) {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::MalformedModuleId,
                        format!("Module name '{}' contains invalid characters", module_name),
                    )
                    .with_note(
                        "Only alphanumeric, '-', '_', ':', '/', and '.' are allowed".to_string(),
                    ),
                );
            }

            // Check for duplicates
            let key = module_name.to_string();
            if !seen_modules.insert(key.clone()) {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::ConflictingModuleVersions,
                        format!("Module '{}' is referenced multiple times", key),
                    )
                    .with_note("Each module should only be referenced once".to_string()),
                );
            }

            // Validate version (version() returns &str, not Option<&str>)
            let version = module_ref.version();
            if version.is_empty() {
                diagnostics.push(Diagnostic::error(
                    DiagnosticCode::MalformedModuleId,
                    format!("Module '{}': version cannot be empty", module_name),
                ));
            }
        }

        diagnostics
    }

    /// Validate world-law declarations (HPDL feature 11, domain 16).
    ///
    /// This is the single semantic pass for world laws, magic, and alternative
    /// physics. HPDL declares and validates contracts; it never executes a law.
    ///
    /// Rules enforced:
    /// - **Duplicate law module reference** within a body → `E6002`
    ///   (`ConflictingModuleVersions`).
    /// - **One authoritative law per exclusive domain** → `E6003`
    ///   (`MultipleWorldLawsForDomain`). Two laws claiming authority over the
    ///   same domain (via `Replaces`, or an additive law colliding with a
    ///   replacement) is a conflict, never a silent override.
    /// - **Magic affects physical state only through a declared bridge** →
    ///   `E6004` (`MissingMagicBridge`). A magic law reaching a physical domain
    ///   with no [`MagicBridge`](hpdl_model::MagicBridge) fails.
    /// - **Non-executable declaration** → `E6005` (`UnsupportedWorldLaw`), a
    ///   warning: a well-formed law whose module has no registered
    ///   implementation is accepted as a declaration but never simulated. The
    ///   module registry is deferred (feature 14), so every declared law is
    ///   flagged at this stage.
    pub fn validate_world_laws(&self, definition: &PlanetDefinition) -> Vec<Diagnostic> {
        use std::collections::{BTreeMap, BTreeSet};

        let mut diagnostics = Vec::new();
        let body = definition.body();
        let laws = body.world_laws();

        if laws.is_empty() {
            return diagnostics;
        }

        // Track module references for duplicate detection (reuse E6002).
        let mut seen_modules: BTreeSet<String> = BTreeSet::new();

        // Map each exclusively-claimed domain to the law modules that claim it,
        // so a domain claimed by more than one authoritative law is a conflict.
        // BTreeMap/BTreeSet keep iteration deterministic.
        let mut authoritative: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for law in laws {
            let module_name = law.module().name().to_string();

            // Duplicate law module reference within the body (E6002).
            if !seen_modules.insert(module_name.clone()) {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::ConflictingModuleVersions,
                        format!(
                            "World law module '{}' is referenced multiple times",
                            module_name
                        ),
                    )
                    .with_note(
                        "Each world-law module should be referenced once per body".to_string(),
                    ),
                );
            }

            // Magic reaching physical state requires a declared bridge (E6004).
            if law.is_magic() && law.touches_physical() && law.bridge().is_none() {
                let physical: Vec<&str> = law
                    .domains()
                    .filter(|d| d.is_physical())
                    .map(|d| d.as_str())
                    .collect();
                let physical_note = if physical.is_empty() {
                    "the law reaches physical state".to_string()
                } else {
                    format!("physical domain(s): {}", physical.join(", "))
                };
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::MissingMagicBridge,
                        format!(
                            "Magic law '{}' affects physical state without a declared bridge",
                            module_name
                        ),
                    )
                    .with_note(physical_note)
                    .with_note(
                        "Magic affects physical state only through a declared MagicBridge"
                            .to_string(),
                    ),
                );
            }

            // Record exclusive (authoritative) domain claims.
            for domain in law.authoritative_domains() {
                authoritative
                    .entry(domain.as_str().to_string())
                    .or_default()
                    .insert(module_name.clone());
            }

            // Every declared law is non-executable at this stage (E6005, warning).
            diagnostics.push(
                Diagnostic::warning(
                    DiagnosticCode::UnsupportedWorldLaw,
                    format!(
                        "World law '{}' has no registered implementation",
                        module_name
                    ),
                )
                .with_note(
                    "Accepted as a declaration, never simulated; the module registry is deferred"
                        .to_string(),
                ),
            );
        }

        // One authoritative law per exclusive domain (E6003).
        for (domain, claimants) in &authoritative {
            if claimants.len() > 1 {
                let modules: Vec<&str> = claimants.iter().map(String::as_str).collect();
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::MultipleWorldLawsForDomain,
                        format!(
                            "Domain '{}' is claimed by multiple authoritative world laws: {}",
                            domain,
                            modules.join(", ")
                        ),
                    )
                    .with_note(
                        "Only one authoritative (replacement) law may own an exclusive domain"
                            .to_string(),
                    ),
                );
            }
        }

        diagnostics
    }
}

impl Default for ModuleValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{BodyId, CelestialBody, ModuleRef, PhysicalProperties, TraitSet};

    fn minimal_definition() -> PlanetDefinition {
        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        PlanetDefinition::new("1.0", body).unwrap()
    }

    #[test]
    fn test_module_validator_creation() {
        let _validator = ModuleValidator::new();
    }

    #[test]
    fn test_validate_modules_minimal() {
        let validator = ModuleValidator::new();
        let definition = minimal_definition();
        let diagnostics = validator.validate_modules(&definition);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_module_reference() {
        let validator = ModuleValidator::new();

        let module = ModuleRef::new("hpdl/stdlib/atmospheric", "1.0").unwrap();

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![module],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = validator.validate_modules(&definition);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_duplicate_module_rejected() {
        let validator = ModuleValidator::new();

        let module1 = ModuleRef::new("hpdl/stdlib/atmospheric", "1.0").unwrap();
        let module2 = ModuleRef::new("hpdl/stdlib/atmospheric", "1.0").unwrap();

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![module1, module2],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = validator.validate_modules(&definition);

        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| { matches!(d.code, DiagnosticCode::ConflictingModuleVersions) }));
    }

    // ----- world laws (feature 11) -----

    use hpdl_model::{LawDomain, LawMode, MagicBridge, WorldLaw};
    use std::collections::BTreeSet;

    fn body_with_laws(laws: Vec<WorldLaw>) -> PlanetDefinition {
        let mut body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        body = body.with_world_laws(laws);
        PlanetDefinition::new("1.0", body).unwrap()
    }

    #[test]
    fn test_no_world_laws_is_clean() {
        let validator = ModuleValidator::new();
        let def = minimal_definition();
        assert!(validator.validate_world_laws(&def).is_empty());
    }

    #[test]
    fn test_valid_additive_law_only_flagged_unsupported() {
        let validator = ModuleValidator::new();
        let law = WorldLaw::new(
            ModuleRef::new("hpdl.law.extra-gravity", "1.0").unwrap(),
            LawMode::Additive,
            [LawDomain::new("physics.gravity").unwrap()],
            None,
        )
        .unwrap();
        let def = body_with_laws(vec![law]);
        let diags = validator.validate_world_laws(&def);
        // Only the non-executable warning, no errors.
        assert!(diags.iter().all(|d| d.severity != crate::Severity::Error));
        assert!(diags
            .iter()
            .any(|d| d.code == DiagnosticCode::UnsupportedWorldLaw));
    }

    #[test]
    fn test_two_laws_one_domain_rejected() {
        let validator = ModuleValidator::new();
        let gravity: BTreeSet<LawDomain> = [LawDomain::new("physics.gravity").unwrap()].into();
        let law1 = WorldLaw::new(
            ModuleRef::new("hpdl.law.newtonian", "1.0").unwrap(),
            LawMode::Replaces(gravity.clone()),
            [LawDomain::new("physics.gravity").unwrap()],
            None,
        )
        .unwrap();
        let law2 = WorldLaw::new(
            ModuleRef::new("hpdl.law.relativistic", "1.0").unwrap(),
            LawMode::Replaces(gravity),
            [LawDomain::new("physics.gravity").unwrap()],
            None,
        )
        .unwrap();
        let def = body_with_laws(vec![law1, law2]);
        let diags = validator.validate_world_laws(&def);
        assert!(diags
            .iter()
            .any(|d| d.code == DiagnosticCode::MultipleWorldLawsForDomain));
    }

    #[test]
    fn test_magic_without_bridge_rejected() {
        let validator = ModuleValidator::new();
        let law = WorldLaw::new(
            ModuleRef::new("hpdl.law.aethermancy", "1.0").unwrap(),
            LawMode::Additive,
            [
                LawDomain::new("arcane.aether").unwrap(),
                LawDomain::new("physics.energy").unwrap(),
            ],
            None, // no bridge: magic reaching physics
        )
        .unwrap();
        let def = body_with_laws(vec![law]);
        let diags = validator.validate_world_laws(&def);
        assert!(diags
            .iter()
            .any(|d| d.code == DiagnosticCode::MissingMagicBridge));
    }

    #[test]
    fn test_magic_with_bridge_accepted() {
        let validator = ModuleValidator::new();
        let bridge = MagicBridge::new(
            [LawDomain::new("arcane.aether").unwrap()],
            LawDomain::new("physics.energy").unwrap(),
            ModuleRef::new("hpdl.bridge.aether-energy", "1.0").unwrap(),
        )
        .unwrap();
        let law = WorldLaw::new(
            ModuleRef::new("hpdl.law.aethermancy", "1.0").unwrap(),
            LawMode::Additive,
            [
                LawDomain::new("arcane.aether").unwrap(),
                LawDomain::new("physics.energy").unwrap(),
            ],
            Some(bridge),
        )
        .unwrap();
        let def = body_with_laws(vec![law]);
        let diags = validator.validate_world_laws(&def);
        assert!(diags
            .iter()
            .all(|d| d.code != DiagnosticCode::MissingMagicBridge));
    }

    #[test]
    fn test_purely_arcane_law_needs_no_bridge() {
        let validator = ModuleValidator::new();
        let law = WorldLaw::new(
            ModuleRef::new("hpdl.law.pure-mana", "1.0").unwrap(),
            LawMode::Additive,
            [LawDomain::new("arcane.mana").unwrap()],
            None,
        )
        .unwrap();
        let def = body_with_laws(vec![law]);
        let diags = validator.validate_world_laws(&def);
        assert!(diags
            .iter()
            .all(|d| d.code != DiagnosticCode::MissingMagicBridge));
    }

    #[test]
    fn test_duplicate_law_module_rejected() {
        let validator = ModuleValidator::new();
        let law1 = WorldLaw::new(
            ModuleRef::new("hpdl.law.dup", "1.0").unwrap(),
            LawMode::Additive,
            [LawDomain::new("physics.gravity").unwrap()],
            None,
        )
        .unwrap();
        let law2 = WorldLaw::new(
            ModuleRef::new("hpdl.law.dup", "2.0").unwrap(),
            LawMode::Additive,
            [LawDomain::new("physics.thermodynamics").unwrap()],
            None,
        )
        .unwrap();
        let def = body_with_laws(vec![law1, law2]);
        let diags = validator.validate_world_laws(&def);
        assert!(diags
            .iter()
            .any(|d| d.code == DiagnosticCode::ConflictingModuleVersions));
    }

    #[test]
    fn test_disjoint_replacement_domains_compose() {
        let validator = ModuleValidator::new();
        let law1 = WorldLaw::new(
            ModuleRef::new("hpdl.law.gravity", "1.0").unwrap(),
            LawMode::Replaces([LawDomain::new("physics.gravity").unwrap()].into()),
            [LawDomain::new("physics.gravity").unwrap()],
            None,
        )
        .unwrap();
        let law2 = WorldLaw::new(
            ModuleRef::new("hpdl.law.thermo", "1.0").unwrap(),
            LawMode::Replaces([LawDomain::new("physics.thermodynamics").unwrap()].into()),
            [LawDomain::new("physics.thermodynamics").unwrap()],
            None,
        )
        .unwrap();
        let def = body_with_laws(vec![law1, law2]);
        let diags = validator.validate_world_laws(&def);
        assert!(diags
            .iter()
            .all(|d| d.code != DiagnosticCode::MultipleWorldLawsForDomain));
    }

    #[test]
    fn test_multiple_different_modules_allowed() {
        let validator = ModuleValidator::new();

        let module1 = ModuleRef::new("hpdl/stdlib/atmospheric", "1.0").unwrap();
        let module2 = ModuleRef::new("hpdl/stdlib/geological", "1.0").unwrap();

        let body = CelestialBody::new(
            BodyId::new("test").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![module1, module2],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();

        let definition = PlanetDefinition::new("1.0", body).unwrap();
        let diagnostics = validator.validate_modules(&definition);
        assert!(diagnostics.is_empty());
    }
}
