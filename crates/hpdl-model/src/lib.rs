//! # hpdl-model
//!
//! Core domain model for HPDL v1. Provides the typed, immutable declaration
//! structs shared by every other crate in the workspace.
//!
//! ## Design constraints
//!
//! - **No workspace dependencies** other than `thiserror`. This crate is the
//!   dependency root.
//! - **No `f32`/`f64`** anywhere. Continuous quantities use scaled integers.
//! - **No Earth defaults.** Every `Option` field being `None` means
//!   *undeclared*, not *Earth-like*.
//! - **No panics** on any reachable path. All fallible operations return
//!   `Result<T, Error>`.
//! - **Deterministic order.** `BTreeSet`/`BTreeMap` used wherever order can
//!   reach output or hashes.
//!
//! ## Public types
//!
//! | Type | Purpose |
//! |---|---|
//! | [`BodyId`] | Validated identifier for a celestial body |
//! | [`PhysicalProperties`] | Physical constants (mass, radius, gravity, …) |
//! | [`TraitSet`] | Ordered set of named body traits |
//! | [`ModuleRef`] | Versioned external module reference |
//! | [`StatusDeclaration`] | Declares a named derived-status field (dimension, baseline, evaluator, cut-points, window) |
//! | [`StatusDimension`] | Independent status dimension (physical/ecosystem/civilization/…) |
//! | [`BaselineRef`] / [`EvaluatorRef`] | Versioned baseline / evaluator references |
//! | [`CutPoints`] | Ordered Amendment A1 band cut points with `classify` |
//! | [`EvaluationWindow`] | Evaluator temporal aggregation window |
//! | [`ActionDeclaration`] | Declares an action with explicit read/write sets + capabilities |
//! | [`Capability`] | Namespaced authority token an action requires |
//! | [`ReadWriteSet`] | Ordered read/write field paths + capability coverage |
//! | [`MigrationDeclaration`] | Declares a forward-only schema migration |
//! | [`CelestialIdentity`] | Declared kind of a body (star, planet, moon, …) |
//! | [`ReferenceFrame`] | Frame an orbit or spin axis is expressed in |
//! | [`ParentRef`] | Gravitational-parent reference + orbit frame |
//! | [`KeplerianElements`] | Six classical orbital elements (scaled integers) |
//! | [`RotationModel`] | Sidereal rotation + spin-axis declaration |
//! | [`EnvironmentDefinition`] | Atmosphere/thermal/hydro/cryo/biosphere/assets aggregate |
//! | [`AtmosphereProfile`] | Surface pressure + gas [`Composition`] + climate-model ref |
//! | [`Composition`] | Normalized gas mole fractions (ppm) |
//! | [`ThermalProfile`] | Mean/min/max surface temperature (milli-kelvin) |
//! | [`Hydrosphere`] / [`Cryosphere`] | Declared surface liquid / ice coverage |
//! | [`Biosphere`] | Tri-state biosphere (absent ≠ zero) |
//! | [`AssetRef`] | Content-addressed visual asset reference |
//! | [`WorldLaw`] | Declares adoption of a versioned world-law / magic module |
//! | [`LawMode`] | Composition mode: additive or explicit domain replacement |
//! | [`LawDomain`] | Namespaced physical/custom domain a law claims |
//! | [`MagicBridge`] | Declared custom-dimension → physical-dimension bridge contract |
//! | [`CelestialBody`] | Complete immutable body declaration |
//! | [`PlanetDefinition`] | Root planet file declaration |
//! | [`Error`] | Typed error enum returned by all constructors |

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
#![deny(clippy::float_arithmetic)] // no f32/f64 in canonical output

mod action_declaration;
mod body_id;
mod capability;
mod celestial_body;
mod environment;
mod error;
mod migration_declaration;
mod module_ref;
mod orbital;
mod package;
mod physical_properties;
mod planet_definition;
mod status_declaration;
mod status_vitality;
mod trait_set;
mod world_law;

pub use action_declaration::ActionDeclaration;
pub use body_id::BodyId;
pub use capability::{Capability, ReadWriteSet};
pub use celestial_body::CelestialBody;
pub use environment::{
    AssetKind, AssetRef, AtmosphereProfile, Biosphere, ComplexityTier, Composition, Cryosphere,
    Digest, EnvironmentDefinition, GasId, Hydrosphere, ThermalProfile, PPM_SCALE,
};
pub use error::Error;
pub use migration_declaration::MigrationDeclaration;
pub use module_ref::ModuleRef;
pub use orbital::{
    CelestialIdentity, KeplerianElements, ParentRef, ReferenceFrame, RotationModel,
    ECCENTRICITY_SCALE, FULL_CIRCLE_MDEG, MAX_TILT_MDEG,
};
pub use package::{PackageId, PackageManifest, MAX_PACKAGE_DEPS};
pub use physical_properties::PhysicalProperties;
pub use planet_definition::PlanetDefinition;
pub use status_declaration::StatusDeclaration;
pub use status_vitality::{
    BaselineRef, CutPoints, EvaluationWindow, EvaluatorRef, StatusDimension,
};
pub use trait_set::TraitSet;
pub use world_law::{
    LawDomain, LawMode, MagicBridge, WorldLaw, MAX_BRIDGE_SOURCES, MAX_DOMAINS_PER_LAW,
    MAX_WORLD_LAWS, PHYSICAL_NAMESPACE,
};
