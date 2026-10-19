//! Module-registry declaration vocabulary (HPDL feature 14, registry concern).
//!
//! These are *source-independent declarations* describing a registered module
//! implementation: its identity ([`PackageId`]), the kind of contract it
//! implements ([`ModuleKind`]), and its content [`Digest`]. They resolve nothing
//! and execute nothing — resolution of a definition's references against a set of
//! [`RegisteredModule`]s lives in `hpdl-compiler`, and execution is the external
//! HORIZON engine's concern.
//!
//! A registry answers three questions and no others (ADR 0011): does a
//! `name@version` **exist**, is its **kind** the one the reference site requires,
//! and does its content **digest** match. It makes no authenticity / trust-root
//! judgement (that is deferred — ADR 0008 OD-14.4).

use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::{Digest, PackageId};

/// The kind of contract a registered module implements.
///
/// Each module reference site in the model requires a specific kind, so a
/// resolver can reject a kind mismatch (e.g. a climate model used where an
/// evaluator is required) as a typed error rather than silently accepting it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, SchemaWrite,
    SchemaRead,
)]
pub enum ModuleKind {
    /// A versioned evaluator that derives a status (`EvaluatorRef`).
    Evaluator,
    /// A climate / atmosphere model (`AtmosphereProfile.climate_model`).
    ClimateModel,
    /// An alternative-physics or magic world-law module (`WorldLaw.module`).
    WorldLaw,
    /// A custom-dimension → physical-dimension bridge module
    /// (`MagicBridge.module`).
    MagicBridge,
    /// A standard-library / body module (`CelestialBody.modules`).
    Library,
    /// A content-addressed baseline artifact (`BaselineRef`). Not a code module;
    /// a registry still resolves its bytes by digest.
    Baseline,
}

impl ModuleKind {
    /// A stable, lowercase identifier for the kind (useful in diagnostics).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Evaluator => "evaluator",
            Self::ClimateModel => "climate-model",
            Self::WorldLaw => "world-law",
            Self::MagicBridge => "magic-bridge",
            Self::Library => "library",
            Self::Baseline => "baseline",
        }
    }
}

/// A registered module implementation: its exact identity, the contract kind it
/// implements, and its content digest.
///
/// A *declaration* only — it records what a registered module *is*. It fetches no
/// bytes, verifies no signatures, and resolves nothing. Because the identity is a
/// [`PackageId`], a mutable version label is rejected at construction: a registry
/// never resolves `latest` or a range.
///
/// # Example
///
/// ```
/// use hpdl_model::{Digest, ModuleKind, PackageId, RegisteredModule};
///
/// let m = RegisteredModule::new(
///     PackageId::new("hpdl.vitality.eco", "1.0.0").unwrap(),
///     ModuleKind::Evaluator,
///     Digest::new("a".repeat(64)).unwrap(),
/// );
/// assert_eq!(m.kind(), ModuleKind::Evaluator);
/// assert_eq!(m.id().name(), "hpdl.vitality.eco");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct RegisteredModule {
    id: PackageId,
    kind: ModuleKind,
    digest: Digest,
}

impl RegisteredModule {
    /// Construct a registered-module descriptor.
    ///
    /// Infallible: the [`PackageId`] (exact version) and [`Digest`] (64-hex) are
    /// already validated by their own constructors.
    #[must_use]
    pub fn new(id: PackageId, kind: ModuleKind, digest: Digest) -> Self {
        Self { id, kind, digest }
    }

    /// The registered module's identity (name + exact version).
    #[must_use]
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    /// The contract kind this module implements.
    #[must_use]
    pub fn kind(&self) -> ModuleKind {
        self.kind
    }

    /// The content digest of the registered module.
    #[must_use]
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn kind_strings_are_stable() {
        assert_eq!(ModuleKind::Evaluator.as_str(), "evaluator");
        assert_eq!(ModuleKind::ClimateModel.as_str(), "climate-model");
        assert_eq!(ModuleKind::WorldLaw.as_str(), "world-law");
        assert_eq!(ModuleKind::MagicBridge.as_str(), "magic-bridge");
        assert_eq!(ModuleKind::Library.as_str(), "library");
        assert_eq!(ModuleKind::Baseline.as_str(), "baseline");
    }

    #[test]
    fn registered_module_accessors() {
        let m = RegisteredModule::new(
            PackageId::new("hpdl.climate.gcm", "2.0.0").unwrap(),
            ModuleKind::ClimateModel,
            Digest::new("b".repeat(64)).unwrap(),
        );
        assert_eq!(m.id().version(), "2.0.0");
        assert_eq!(m.kind(), ModuleKind::ClimateModel);
        assert_eq!(m.digest().as_str(), "b".repeat(64));
    }

    #[test]
    fn registered_module_rejects_mutable_version_via_package_id() {
        // The exactness guarantee comes from PackageId.
        assert!(PackageId::new("hpdl.a", "latest").is_err());
    }
}
