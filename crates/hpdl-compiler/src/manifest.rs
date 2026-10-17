//! Reproducible build manifest.
//!
//! A [`BuildManifest`] records exactly what a compile produced: the compiler
//! semantics version, the PIR format version, the canonical content hash, and
//! the resolved module closure. Its text form is stable, sorted, and
//! line-oriented so two compiles of the same locked input produce byte-identical
//! manifests and diffs stay small.

use hpdl_pir::{PirEnvelope, PirVersion};

use crate::resolve::ResolvedModule;

/// A deterministic record of a single compilation's outputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildManifest {
    /// Logical name of the compiled source.
    pub source_name: String,
    /// Compiler semantics version (the `hpdl-compiler` crate version).
    pub compiler_semantics_version: String,
    /// PIR format version of the emitted envelope.
    pub pir_version: PirVersion,
    /// Canonical content hash of the emitted PIR, hex-encoded.
    pub content_hash: String,
    /// Resolved module closure, sorted by name.
    pub modules: Vec<ResolvedModule>,
}

impl BuildManifest {
    /// Build a manifest from a compile's envelope and resolved modules.
    ///
    /// `modules` is expected already sorted by name (as produced by
    /// [`crate::ResolvedModules::into_vec`]).
    #[must_use]
    pub fn new(source_name: &str, envelope: &PirEnvelope, modules: Vec<ResolvedModule>) -> Self {
        Self {
            source_name: source_name.to_string(),
            compiler_semantics_version: env!("CARGO_PKG_VERSION").to_string(),
            pir_version: envelope.version,
            content_hash: envelope.content_hash.to_hex(),
            modules,
        }
    }

    /// Serialize the manifest to its stable, line-oriented text form.
    ///
    /// Header fields appear in a fixed order, followed by one `module =` line
    /// per resolved module (already sorted). No `HashMap` iteration is involved,
    /// so output is fully deterministic.
    ///
    /// # Example
    ///
    /// ```
    /// use hpdl_compiler::{Compiler, CompileOptions};
    /// # const SRC: &str = r#"PlanetDefinition(language_version:"1.0",body:CelestialBody(id:BodyId("x"),properties:PhysicalProperties(mass_kg:None,radius_m:None,surface_gravity_mms2:None,orbital_period_s:None,rotation_period_s:None),traits:TraitSet([]),modules:[],statuses:[],actions:[],migrations:[]))"#;
    /// let result = Compiler::new(CompileOptions::default()).compile_ron_str("x", SRC).unwrap();
    /// let text = result.manifest().to_text();
    /// assert!(text.contains("source_name = x"));
    /// ```
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("source_name = {}\n", self.source_name));
        out.push_str(&format!(
            "compiler_semantics_version = {}\n",
            self.compiler_semantics_version
        ));
        out.push_str(&format!("pir_version = {}\n", self.pir_version));
        out.push_str(&format!("content_hash = {}\n", self.content_hash));
        for module in &self.modules {
            out.push_str(&format!(
                "module = {} {} {}\n",
                module.name, module.version, module.content_hash
            ));
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use hpdl_model::{BodyId, CelestialBody, PhysicalProperties, PlanetDefinition, TraitSet};
    use hpdl_pir::encode_planet_definition;

    fn envelope() -> PirEnvelope {
        let body = CelestialBody::new(
            BodyId::new("earth").unwrap(),
            PhysicalProperties::default(),
            TraitSet::empty(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        let def = PlanetDefinition::new("1.0", body).unwrap();
        encode_planet_definition(&def).unwrap()
    }

    #[test]
    fn manifest_text_is_stable() {
        let env = envelope();
        let m1 = BuildManifest::new("earth", &env, vec![]);
        let m2 = BuildManifest::new("earth", &env, vec![]);
        assert_eq!(m1.to_text(), m2.to_text());
    }

    #[test]
    fn manifest_includes_fields() {
        let env = envelope();
        let manifest = BuildManifest::new(
            "earth",
            &env,
            vec![ResolvedModule {
                name: "hpdl.atmosphere".to_string(),
                version: "1.2.0".to_string(),
                content_hash: "aaaa".to_string(),
            }],
        );
        let text = manifest.to_text();
        assert!(text.contains("source_name = earth"));
        assert!(text.contains("pir_version = 2.0"));
        assert!(text.contains(&format!("content_hash = {}", env.content_hash.to_hex())));
        assert!(text.contains("module = hpdl.atmosphere 1.2.0 aaaa"));
    }
}
