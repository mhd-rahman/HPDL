//! World-law and custom-reality declarations (HPDL feature 11, domain 16).
//!
//! A [`WorldLaw`] is a pure, immutable **declaration** that a body opts into a
//! versioned, registered law module — alternative physics, a magic system, or
//! any custom reality rule. HPDL ships **no built-in or universal magic**: every
//! law is explicit. Declaring a law does not execute it; the external HORIZON
//! engine (and its registered module implementations) runs the contract.
//!
//! # Invariants enforced here and in `hpdl-sema`
//!
//! - **Modes are exactly two:** [`LawMode::Additive`] or an explicit
//!   [`LawMode::Replaces`] of named domains (type-level, no last-write-wins).
//! - **One authoritative law per exclusive domain** — enforced by the single
//!   semantic validator (`E6003`).
//! - **Magic affects physical state only through a declared bridge** — a magic
//!   law that reaches a physical domain with no [`MagicBridge`] fails (`E6004`).
//! - **Custom dimensions never coerce to SI** without a declared bridge. The
//!   `physics` namespace prefix marks physical domains; everything else (e.g.
//!   `arcane.aether`) is a custom dimension.
//!
//! # Design constraints honoured here
//!
//! - **No `f32`/`f64`.** A bridge records a module *contract*, not a conversion.
//! - **Deterministic.** Claimed/replaced/source domains are `BTreeSet`s.
//! - **Checked, bounded construction.** Every constructor validates tokens and
//!   enforces collection bounds, returning a typed [`Error`].

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Error, ModuleRef};

/// Maximum length for a law-domain token.
const MAX_DOMAIN_LEN: usize = 64;

/// Maximum number of world laws a single body may declare.
pub const MAX_WORLD_LAWS: usize = 256;

/// Maximum number of domains a single law may claim (or replace).
pub const MAX_DOMAINS_PER_LAW: usize = 64;

/// Maximum number of source dimensions a single magic bridge may map.
pub const MAX_BRIDGE_SOURCES: usize = 64;

/// The reserved namespace prefix marking a *physical* domain.
///
/// A [`LawDomain`] whose first dotted segment is `physics` (e.g.
/// `physics.gravity`) is a physical domain; any other first segment (e.g.
/// `arcane.aether`) is a custom dimension. Magic reaches physical state only
/// through a declared [`MagicBridge`].
pub const PHYSICAL_NAMESPACE: &str = "physics";

/// A validated, namespaced domain token a world law may claim authority over.
///
/// Valid tokens are 1–64 characters from `[A-Za-z0-9_\-.]` with non-empty
/// dotted segments — e.g. `physics.gravity`, `physics.thermodynamics`,
/// `arcane.aether`. There is **no built-in closed list** of domains: the
/// taxonomy is open so alternative physics and magic can be expressed, while the
/// `physics` prefix gives the magic-bridge rule something concrete to test.
///
/// # Example
///
/// ```
/// use hpdl_model::LawDomain;
///
/// let gravity = LawDomain::new("physics.gravity").unwrap();
/// assert!(gravity.is_physical());
///
/// let aether = LawDomain::new("arcane.aether").unwrap();
/// assert!(aether.is_custom());
///
/// assert!(LawDomain::new("bad domain!").is_err());
/// assert!(LawDomain::new("trailing.").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LawDomain(String);

impl LawDomain {
    /// Construct a validated `LawDomain`.
    ///
    /// Returns [`Error::InvalidLawDomain`] if the token is empty, longer than 64
    /// characters, contains characters outside `[A-Za-z0-9_\-.]`, or has an empty
    /// dotted segment (e.g. a leading, trailing, or doubled dot).
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if s.is_empty() || s.len() > MAX_DOMAIN_LEN {
            return Err(Error::InvalidLawDomain(s));
        }
        for segment in s.split('.') {
            if segment.is_empty()
                || !segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            {
                return Err(Error::InvalidLawDomain(s));
            }
        }
        Ok(Self(s))
    }

    /// The underlying token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this domain lives in the reserved physical namespace
    /// (first segment == [`PHYSICAL_NAMESPACE`]).
    pub fn is_physical(&self) -> bool {
        let first = self.0.split('.').next().unwrap_or(&self.0);
        first == PHYSICAL_NAMESPACE
    }

    /// Whether this domain is a custom (non-physical) dimension.
    pub fn is_custom(&self) -> bool {
        !self.is_physical()
    }
}

/// How a world law composes with the body's existing domain authority.
///
/// Composition is **intersection, not last-write-wins**: a law either adds
/// behavior ([`LawMode::Additive`]) or explicitly becomes the single
/// authoritative law for the named domains ([`LawMode::Replaces`]). Two laws
/// claiming authority over the same exclusive domain is a conflict (`E6003`),
/// never a silent override.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LawMode {
    /// The law adds behavior without displacing any existing domain authority.
    Additive,
    /// The law is the single authoritative law for each named domain, explicitly
    /// displacing any default/standard behavior there.
    Replaces(BTreeSet<LawDomain>),
}

/// A declared mapping from one or more custom (magic) dimensions to a single
/// physical dimension.
///
/// A `MagicBridge` is a **contract, not a computation**. It names the source
/// custom dimension(s), the target physical dimension, and the versioned bridge
/// module that performs the fixed-point lowering *at runtime*. HPDL validates
/// that a bridge is declared where one is required and that its tokens and
/// module reference are well-formed; it performs no unit conversion.
///
/// # Example
///
/// ```
/// use hpdl_model::{LawDomain, MagicBridge, ModuleRef};
///
/// let bridge = MagicBridge::new(
///     [LawDomain::new("arcane.aether").unwrap()],
///     LawDomain::new("physics.energy").unwrap(),
///     ModuleRef::new("hpdl.bridge.aether-energy", "1.0").unwrap(),
/// ).unwrap();
/// assert_eq!(bridge.target().as_str(), "physics.energy");
///
/// // A non-physical target is rejected: a bridge must reach physical state.
/// assert!(MagicBridge::new(
///     [LawDomain::new("arcane.aether").unwrap()],
///     LawDomain::new("arcane.mana").unwrap(),
///     ModuleRef::new("m", "1.0").unwrap(),
/// ).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MagicBridge {
    sources: BTreeSet<LawDomain>,
    target: LawDomain,
    module: ModuleRef,
}

impl MagicBridge {
    /// Construct a validated `MagicBridge`.
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyBridgeSources`] if no source dimensions are provided.
    /// - [`Error::TooManyBridgeSources`] if more than [`MAX_BRIDGE_SOURCES`] are
    ///   provided.
    /// - [`Error::NonPhysicalBridgeTarget`] if the target is not in the physical
    ///   namespace (a bridge exists to reach physical state).
    pub fn new(
        sources: impl IntoIterator<Item = LawDomain>,
        target: LawDomain,
        module: ModuleRef,
    ) -> Result<Self, Error> {
        let sources: BTreeSet<LawDomain> = sources.into_iter().collect();
        if sources.is_empty() {
            return Err(Error::EmptyBridgeSources);
        }
        if sources.len() > MAX_BRIDGE_SOURCES {
            return Err(Error::TooManyBridgeSources(sources.len()));
        }
        if !target.is_physical() {
            return Err(Error::NonPhysicalBridgeTarget(target.as_str().to_owned()));
        }
        Ok(Self {
            sources,
            target,
            module,
        })
    }

    /// The source custom dimensions, in deterministic order.
    pub fn sources(&self) -> impl Iterator<Item = &LawDomain> {
        self.sources.iter()
    }

    /// The target physical dimension.
    pub fn target(&self) -> &LawDomain {
        &self.target
    }

    /// The versioned bridge module reference.
    pub fn module(&self) -> &ModuleRef {
        &self.module
    }
}

/// A body's declaration that it adopts a versioned world-law module.
///
/// A `WorldLaw` carries the law's module reference, its composition
/// [`LawMode`], the set of domains it claims authority over, and an optional
/// [`MagicBridge`] (present iff the law reaches physical state from a custom
/// dimension). It is a declaration only — nothing is executed here.
///
/// # Example
///
/// ```
/// use hpdl_model::{LawDomain, LawMode, MagicBridge, ModuleRef, WorldLaw};
///
/// // A magic law that affects physics declares a bridge.
/// let law = WorldLaw::new(
///     ModuleRef::new("hpdl.law.aethermancy", "1.0").unwrap(),
///     LawMode::Additive,
///     [LawDomain::new("arcane.aether").unwrap(), LawDomain::new("physics.energy").unwrap()],
///     Some(MagicBridge::new(
///         [LawDomain::new("arcane.aether").unwrap()],
///         LawDomain::new("physics.energy").unwrap(),
///         ModuleRef::new("hpdl.bridge.aether-energy", "1.0").unwrap(),
///     ).unwrap()),
/// ).unwrap();
///
/// assert!(law.is_magic());
/// assert!(law.touches_physical());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLaw {
    module: ModuleRef,
    mode: LawMode,
    domains: BTreeSet<LawDomain>,
    #[serde(default)]
    bridge: Option<MagicBridge>,
}

impl WorldLaw {
    /// Construct a validated `WorldLaw`.
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyLawDomains`] if the law claims no domains.
    /// - [`Error::TooManyLawDomains`] if the claimed set (or a `Replaces` set)
    ///   exceeds [`MAX_DOMAINS_PER_LAW`].
    pub fn new(
        module: ModuleRef,
        mode: LawMode,
        domains: impl IntoIterator<Item = LawDomain>,
        bridge: Option<MagicBridge>,
    ) -> Result<Self, Error> {
        let domains: BTreeSet<LawDomain> = domains.into_iter().collect();
        if domains.is_empty() {
            return Err(Error::EmptyLawDomains);
        }
        if domains.len() > MAX_DOMAINS_PER_LAW {
            return Err(Error::TooManyLawDomains(domains.len()));
        }
        if let LawMode::Replaces(replaced) = &mode {
            if replaced.is_empty() {
                return Err(Error::EmptyLawDomains);
            }
            if replaced.len() > MAX_DOMAINS_PER_LAW {
                return Err(Error::TooManyLawDomains(replaced.len()));
            }
        }
        Ok(Self {
            module,
            mode,
            domains,
            bridge,
        })
    }

    /// The law's module reference.
    pub fn module(&self) -> &ModuleRef {
        &self.module
    }

    /// The law's composition mode.
    pub fn mode(&self) -> &LawMode {
        &self.mode
    }

    /// All domains the law claims, in deterministic order.
    pub fn domains(&self) -> impl Iterator<Item = &LawDomain> {
        self.domains.iter()
    }

    /// The declared magic bridge, if any.
    pub fn bridge(&self) -> Option<&MagicBridge> {
        self.bridge.as_ref()
    }

    /// The domains for which this law claims **exclusive authority**.
    ///
    /// For a [`LawMode::Replaces`] law this is the replaced-domain set; for an
    /// [`LawMode::Additive`] law it is empty (additive laws coexist). The single
    /// semantic validator uses this to enforce one-authoritative-law-per-domain.
    pub fn authoritative_domains(&self) -> BTreeSet<LawDomain> {
        match &self.mode {
            LawMode::Additive => BTreeSet::new(),
            LawMode::Replaces(replaced) => replaced.clone(),
        }
    }

    /// Whether the law reaches any physical domain (claimed domain or bridge
    /// target in the [`PHYSICAL_NAMESPACE`]).
    pub fn touches_physical(&self) -> bool {
        self.domains.iter().any(LawDomain::is_physical)
            || self
                .bridge
                .as_ref()
                .is_some_and(|b| b.target().is_physical())
    }

    /// Whether the law involves any custom (magic) dimension (a claimed domain
    /// or a bridge source outside the physical namespace).
    pub fn is_magic(&self) -> bool {
        self.domains.iter().any(LawDomain::is_custom)
            || self
                .bridge
                .as_ref()
                .is_some_and(|b| b.sources().any(LawDomain::is_custom))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn module(name: &str) -> ModuleRef {
        ModuleRef::new(name, "1.0").unwrap()
    }

    // ----- LawDomain -----

    #[test]
    fn domain_validation() {
        assert!(LawDomain::new("physics.gravity").is_ok());
        assert!(LawDomain::new("arcane.aether").is_ok());
        assert!(LawDomain::new("single").is_ok());
        assert!(LawDomain::new("").is_err());
        assert!(LawDomain::new("bad domain!").is_err());
        assert!(LawDomain::new(".leading").is_err());
        assert!(LawDomain::new("trailing.").is_err());
        assert!(LawDomain::new("double..dot").is_err());
        assert!(LawDomain::new("a".repeat(65)).is_err());
    }

    #[test]
    fn domain_physical_vs_custom() {
        assert!(LawDomain::new("physics.gravity").unwrap().is_physical());
        assert!(LawDomain::new("physics").unwrap().is_physical());
        assert!(LawDomain::new("arcane.aether").unwrap().is_custom());
        assert!(LawDomain::new("physicsx.not").unwrap().is_custom());
    }

    #[test]
    fn domain_ordering_is_deterministic() {
        assert!(LawDomain::new("a.b").unwrap() < LawDomain::new("a.c").unwrap());
    }

    // ----- MagicBridge -----

    #[test]
    fn bridge_valid() {
        let b = MagicBridge::new(
            [LawDomain::new("arcane.aether").unwrap()],
            LawDomain::new("physics.energy").unwrap(),
            module("hpdl.bridge.aether-energy"),
        )
        .unwrap();
        assert_eq!(b.target().as_str(), "physics.energy");
        assert_eq!(b.sources().count(), 1);
    }

    #[test]
    fn bridge_empty_sources_rejected() {
        assert!(matches!(
            MagicBridge::new(
                [] as [LawDomain; 0],
                LawDomain::new("physics.energy").unwrap(),
                module("m"),
            ),
            Err(Error::EmptyBridgeSources)
        ));
    }

    #[test]
    fn bridge_non_physical_target_rejected() {
        assert!(matches!(
            MagicBridge::new(
                [LawDomain::new("arcane.aether").unwrap()],
                LawDomain::new("arcane.mana").unwrap(),
                module("m"),
            ),
            Err(Error::NonPhysicalBridgeTarget(_))
        ));
    }

    // ----- WorldLaw -----

    #[test]
    fn additive_law_has_no_authoritative_domains() {
        let law = WorldLaw::new(
            module("hpdl.law.extra"),
            LawMode::Additive,
            [LawDomain::new("physics.gravity").unwrap()],
            None,
        )
        .unwrap();
        assert!(law.authoritative_domains().is_empty());
        assert!(law.touches_physical());
        assert!(!law.is_magic());
    }

    #[test]
    fn replaces_law_is_authoritative() {
        let replaced: BTreeSet<LawDomain> = [LawDomain::new("physics.gravity").unwrap()].into();
        let law = WorldLaw::new(
            module("hpdl.law.newtonian-override"),
            LawMode::Replaces(replaced.clone()),
            [LawDomain::new("physics.gravity").unwrap()],
            None,
        )
        .unwrap();
        assert_eq!(law.authoritative_domains(), replaced);
    }

    #[test]
    fn magic_law_classification() {
        let law = WorldLaw::new(
            module("hpdl.law.aethermancy"),
            LawMode::Additive,
            [
                LawDomain::new("arcane.aether").unwrap(),
                LawDomain::new("physics.energy").unwrap(),
            ],
            Some(
                MagicBridge::new(
                    [LawDomain::new("arcane.aether").unwrap()],
                    LawDomain::new("physics.energy").unwrap(),
                    module("hpdl.bridge.aether-energy"),
                )
                .unwrap(),
            ),
        )
        .unwrap();
        assert!(law.is_magic());
        assert!(law.touches_physical());
        assert!(law.bridge().is_some());
    }

    #[test]
    fn empty_domains_rejected() {
        assert!(matches!(
            WorldLaw::new(module("m"), LawMode::Additive, [] as [LawDomain; 0], None),
            Err(Error::EmptyLawDomains)
        ));
    }

    #[test]
    fn too_many_domains_rejected() {
        let many: Vec<LawDomain> = (0..=MAX_DOMAINS_PER_LAW)
            .map(|i| LawDomain::new(format!("arcane.d{i}")).unwrap())
            .collect();
        assert!(matches!(
            WorldLaw::new(module("m"), LawMode::Additive, many, None),
            Err(Error::TooManyLawDomains(_))
        ));
    }
}
