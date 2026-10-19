//! Celestial and orbital declaration types (HPDL feature 07).
//!
//! These types are pure, immutable **declarations**. Declaring a Keplerian
//! orbit is not an orbital solver, and declaring a [`RotationModel`] does not
//! evolve a body's orientation. The external HORIZON simulation alone executes
//! any state transition.
//!
//! # Design constraints honoured here
//!
//! - **No `f32`/`f64`.** Every continuous quantity is a scaled integer:
//!   eccentricity as `e × 1_000_000`, angles in milli-degrees, distances in
//!   metres. This keeps canonical output deterministic.
//! - **No Earth defaults.** Nothing has an implicit value; absence lives in the
//!   `Option` fields on [`crate::CelestialBody`], never here.
//! - **Checked construction.** Every constructor validates ranges and returns
//!   [`Error`], never panics.

use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::{BodyId, Error};

/// Maximum length for a custom frame or celestial-kind name.
const MAX_NAME_LEN: usize = 64;

/// Upper exclusive bound for eccentricity stored as `e × 1_000_000`.
///
/// v1 supports only closed (elliptical) orbits, `e ∈ [0, 1)`.
pub const ECCENTRICITY_SCALE: u32 = 1_000_000;

/// Upper inclusive bound for inclination / axial tilt, in milli-degrees (180°).
pub const MAX_TILT_MDEG: i32 = 180_000;

/// Upper exclusive bound for a full-circle angle, in milli-degrees (360°).
pub const FULL_CIRCLE_MDEG: i32 = 360_000;

/// Validate a custom frame or celestial-kind name.
///
/// Valid names are 1–64 characters drawn from `[A-Za-z0-9_\-.]`.
fn validate_frame_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > MAX_NAME_LEN
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(Error::InvalidFrameName(name.to_owned()));
    }
    Ok(())
}

/// The reference frame an orbit's elements or a spin axis are expressed in.
///
/// `Custom` is an explicit escape hatch for a namespaced, validated frame name.
/// A `Custom` frame is accepted as a *declaration* but is treated as
/// non-executable by the semantic validator — nothing claims it can be
/// simulated.
///
/// # Example
///
/// ```
/// use hpdl_model::ReferenceFrame;
///
/// let f = ReferenceFrame::EclipticJ2000;
/// assert_eq!(f, ReferenceFrame::EclipticJ2000);
///
/// let custom = ReferenceFrame::custom("arcane.ley-aligned").unwrap();
/// assert!(matches!(custom, ReferenceFrame::Custom(_)));
///
/// assert!(ReferenceFrame::custom("bad frame!").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub enum ReferenceFrame {
    /// Earth mean equator and equinox of J2000.0.
    EquatorialJ2000,
    /// Ecliptic and equinox of J2000.0.
    EclipticJ2000,
    /// A frame fixed to the body itself (co-rotating).
    BodyFixed,
    /// A frame centred on the declared gravitational parent.
    ParentCentered,
    /// A namespaced, validated custom frame (non-executable declaration).
    Custom(String),
}

impl ReferenceFrame {
    /// Construct a [`ReferenceFrame::Custom`], validating the name.
    ///
    /// Returns [`Error::InvalidFrameName`] if the name is empty, longer than 64
    /// characters, or contains characters outside `[A-Za-z0-9_\-.]`.
    pub fn custom(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        validate_frame_name(&name)?;
        Ok(Self::Custom(name))
    }

    /// Return `true` if this frame is a supported, interpretable frame.
    ///
    /// `Custom` frames return `false`: they are valid declarations but are not
    /// treated as executable by the semantic validator.
    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Custom(_))
    }
}

/// The declared kind of a celestial body.
///
/// `Other` is an explicit escape hatch for a namespaced, validated kind name.
///
/// # Example
///
/// ```
/// use hpdl_model::CelestialIdentity;
///
/// assert_eq!(CelestialIdentity::Planet, CelestialIdentity::Planet);
///
/// let other = CelestialIdentity::other("rogue.planet").unwrap();
/// assert!(matches!(other, CelestialIdentity::Other(_)));
///
/// assert!(CelestialIdentity::other("").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub enum CelestialIdentity {
    /// A star.
    Star,
    /// A planet.
    Planet,
    /// A dwarf planet.
    DwarfPlanet,
    /// A natural satellite.
    Moon,
    /// An asteroid.
    Asteroid,
    /// A comet.
    Comet,
    /// A barycenter (centre of mass of a system).
    Barycenter,
    /// A namespaced, validated custom kind.
    Other(String),
}

impl CelestialIdentity {
    /// Construct a [`CelestialIdentity::Other`], validating the kind name.
    ///
    /// Returns [`Error::InvalidFrameName`] on an empty, too-long, or
    /// invalid-character name (same rule as custom frames).
    pub fn other(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        validate_frame_name(&name)?;
        Ok(Self::Other(name))
    }
}

/// A reference to a body's gravitational parent and the frame its orbit is
/// expressed relative to.
///
/// A [`ParentRef`] does not know the id of the body that declares it, so a
/// degenerate self-parent (`target` equal to the owning body) is detected by
/// the semantic validator rather than at construction.
///
/// # Example
///
/// ```
/// use hpdl_model::{BodyId, ParentRef, ReferenceFrame};
///
/// let parent = ParentRef::new(
///     BodyId::new("sol").unwrap(),
///     ReferenceFrame::EclipticJ2000,
/// );
/// assert_eq!(parent.target().as_str(), "sol");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct ParentRef {
    target: BodyId,
    frame: ReferenceFrame,
}

impl ParentRef {
    /// Construct a [`ParentRef`]. This cannot fail: both inputs are already
    /// validated by their own constructors.
    pub fn new(target: BodyId, frame: ReferenceFrame) -> Self {
        Self { target, frame }
    }

    /// Return the parent body's identifier.
    pub fn target(&self) -> &BodyId {
        &self.target
    }

    /// Return the reference frame the child's orbit is expressed in.
    pub fn frame(&self) -> &ReferenceFrame {
        &self.frame
    }
}

/// The six classical Keplerian orbital elements, stored as scaled integers.
///
/// Units and scales (no `f32`/`f64` anywhere):
///
/// | Field | Unit | Range |
/// |---|---|---|
/// | `semi_major_axis_m` | metres | `> 0` |
/// | `eccentricity_micro` | `e × 1_000_000` | `[0, 1_000_000)` |
/// | `inclination_mdeg` | milli-degrees | `[0, 180_000]` |
/// | `lon_ascending_node_mdeg` | milli-degrees | `[0, 360_000)` |
/// | `arg_periapsis_mdeg` | milli-degrees | `[0, 360_000)` |
/// | `mean_anomaly_epoch_mdeg` | milli-degrees | `[0, 360_000)` |
///
/// # Example
///
/// ```
/// use hpdl_model::{KeplerianElements, ReferenceFrame};
///
/// // A roughly Earth-like orbit (illustrative values only — not Earth defaults).
/// let orbit = KeplerianElements::new(
///     149_598_000_000,       // ~1 AU in metres
///     16_700,                // e ≈ 0.0167
///     0,                     // inclination
///     174_000,               // longitude of ascending node
///     288_000,               // argument of periapsis
///     0,                     // mean anomaly at epoch
///     ReferenceFrame::EclipticJ2000,
/// ).unwrap();
/// assert_eq!(orbit.eccentricity_micro(), 16_700);
///
/// // e = 1.0 (parabolic) is rejected in v1.
/// assert!(KeplerianElements::new(
///     1, 1_000_000, 0, 0, 0, 0, ReferenceFrame::EclipticJ2000,
/// ).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct KeplerianElements {
    semi_major_axis_m: u128,
    eccentricity_micro: u32,
    inclination_mdeg: i32,
    lon_ascending_node_mdeg: i32,
    arg_periapsis_mdeg: i32,
    mean_anomaly_epoch_mdeg: i32,
    frame: ReferenceFrame,
}

/// Validate a `[0, 360_000)` full-circle angle.
fn validate_circle_angle(value: i32, field: &'static str) -> Result<(), Error> {
    if !(0..FULL_CIRCLE_MDEG).contains(&value) {
        return Err(Error::InvalidAngle { field, value });
    }
    Ok(())
}

/// Validate a `[0, 180_000]` tilt/inclination angle.
fn validate_tilt_angle(value: i32, field: &'static str) -> Result<(), Error> {
    if !(0..=MAX_TILT_MDEG).contains(&value) {
        return Err(Error::InvalidAngle { field, value });
    }
    Ok(())
}

impl KeplerianElements {
    /// Construct a [`KeplerianElements`], validating every field.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidSemiMajorAxis`] if `semi_major_axis_m == 0`.
    /// - [`Error::InvalidEccentricity`] if `eccentricity_micro >= 1_000_000`.
    /// - [`Error::InvalidAngle`] if any angle is outside its documented range.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        semi_major_axis_m: u128,
        eccentricity_micro: u32,
        inclination_mdeg: i32,
        lon_ascending_node_mdeg: i32,
        arg_periapsis_mdeg: i32,
        mean_anomaly_epoch_mdeg: i32,
        frame: ReferenceFrame,
    ) -> Result<Self, Error> {
        if semi_major_axis_m == 0 {
            return Err(Error::InvalidSemiMajorAxis(semi_major_axis_m));
        }
        if eccentricity_micro >= ECCENTRICITY_SCALE {
            return Err(Error::InvalidEccentricity(eccentricity_micro));
        }
        validate_tilt_angle(inclination_mdeg, "inclination")?;
        validate_circle_angle(lon_ascending_node_mdeg, "lon_ascending_node")?;
        validate_circle_angle(arg_periapsis_mdeg, "arg_periapsis")?;
        validate_circle_angle(mean_anomaly_epoch_mdeg, "mean_anomaly_epoch")?;

        Ok(Self {
            semi_major_axis_m,
            eccentricity_micro,
            inclination_mdeg,
            lon_ascending_node_mdeg,
            arg_periapsis_mdeg,
            mean_anomaly_epoch_mdeg,
            frame,
        })
    }

    /// Semi-major axis in metres.
    pub fn semi_major_axis_m(&self) -> u128 {
        self.semi_major_axis_m
    }

    /// Eccentricity as `e × 1_000_000`.
    pub fn eccentricity_micro(&self) -> u32 {
        self.eccentricity_micro
    }

    /// Inclination in milli-degrees.
    pub fn inclination_mdeg(&self) -> i32 {
        self.inclination_mdeg
    }

    /// Longitude of ascending node in milli-degrees.
    pub fn lon_ascending_node_mdeg(&self) -> i32 {
        self.lon_ascending_node_mdeg
    }

    /// Argument of periapsis in milli-degrees.
    pub fn arg_periapsis_mdeg(&self) -> i32 {
        self.arg_periapsis_mdeg
    }

    /// Mean anomaly at epoch in milli-degrees.
    pub fn mean_anomaly_epoch_mdeg(&self) -> i32 {
        self.mean_anomaly_epoch_mdeg
    }

    /// The reference frame the elements are expressed in.
    pub fn frame(&self) -> &ReferenceFrame {
        &self.frame
    }
}

/// A declaration of a body's sidereal rotation and spin-axis orientation.
///
/// This is a declaration only; it does not evolve orientation over time.
///
/// # Example
///
/// ```
/// use hpdl_model::{RotationModel, ReferenceFrame};
///
/// let rot = RotationModel::new(86_164, 23_440, ReferenceFrame::EclipticJ2000).unwrap();
/// assert_eq!(rot.sidereal_period_s(), 86_164);
///
/// // Retrograde rotation is allowed (negative period).
/// assert!(RotationModel::new(-20_996_800, 177_000, ReferenceFrame::EclipticJ2000).is_ok());
///
/// // A zero period is rejected.
/// assert!(RotationModel::new(0, 0, ReferenceFrame::EclipticJ2000).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct RotationModel {
    sidereal_period_s: i64,
    axial_tilt_mdeg: i32,
    frame: ReferenceFrame,
}

impl RotationModel {
    /// Construct a [`RotationModel`], validating its inputs.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidRotationPeriod`] if `sidereal_period_s == 0`.
    /// - [`Error::InvalidAngle`] if `axial_tilt_mdeg` is outside `[0, 180_000]`.
    pub fn new(
        sidereal_period_s: i64,
        axial_tilt_mdeg: i32,
        frame: ReferenceFrame,
    ) -> Result<Self, Error> {
        if sidereal_period_s == 0 {
            return Err(Error::InvalidRotationPeriod);
        }
        validate_tilt_angle(axial_tilt_mdeg, "axial_tilt")?;
        Ok(Self {
            sidereal_period_s,
            axial_tilt_mdeg,
            frame,
        })
    }

    /// Sidereal rotation period in seconds (negative = retrograde).
    pub fn sidereal_period_s(&self) -> i64 {
        self.sidereal_period_s
    }

    /// Axial tilt (obliquity) in milli-degrees.
    pub fn axial_tilt_mdeg(&self) -> i32 {
        self.axial_tilt_mdeg
    }

    /// The reference frame the spin axis is expressed in.
    pub fn frame(&self) -> &ReferenceFrame {
        &self.frame
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    // ----- ReferenceFrame -----

    #[test]
    fn reference_frame_custom_valid() {
        let f = ReferenceFrame::custom("arcane.ley-aligned").unwrap();
        assert!(matches!(f, ReferenceFrame::Custom(_)));
        assert!(!f.is_supported());
    }

    #[test]
    fn reference_frame_closed_variants_supported() {
        assert!(ReferenceFrame::EquatorialJ2000.is_supported());
        assert!(ReferenceFrame::EclipticJ2000.is_supported());
        assert!(ReferenceFrame::BodyFixed.is_supported());
        assert!(ReferenceFrame::ParentCentered.is_supported());
    }

    #[test]
    fn reference_frame_custom_invalid() {
        assert!(matches!(
            ReferenceFrame::custom("bad frame!"),
            Err(Error::InvalidFrameName(_))
        ));
        assert!(matches!(
            ReferenceFrame::custom(""),
            Err(Error::InvalidFrameName(_))
        ));
        let long = "a".repeat(65);
        assert!(matches!(
            ReferenceFrame::custom(long),
            Err(Error::InvalidFrameName(_))
        ));
    }

    // ----- CelestialIdentity -----

    #[test]
    fn identity_other_valid() {
        let k = CelestialIdentity::other("rogue.planet").unwrap();
        assert!(matches!(k, CelestialIdentity::Other(_)));
    }

    #[test]
    fn identity_other_invalid() {
        assert!(matches!(
            CelestialIdentity::other("has space"),
            Err(Error::InvalidFrameName(_))
        ));
    }

    // ----- KeplerianElements -----

    fn ok_orbit() -> KeplerianElements {
        KeplerianElements::new(
            149_598_000_000,
            16_700,
            0,
            174_000,
            288_000,
            0,
            ReferenceFrame::EclipticJ2000,
        )
        .unwrap()
    }

    #[test]
    fn kepler_valid() {
        let o = ok_orbit();
        assert_eq!(o.semi_major_axis_m(), 149_598_000_000);
        assert_eq!(o.eccentricity_micro(), 16_700);
        assert_eq!(o.frame(), &ReferenceFrame::EclipticJ2000);
    }

    #[test]
    fn kepler_zero_semi_major_axis_rejected() {
        assert!(matches!(
            KeplerianElements::new(0, 0, 0, 0, 0, 0, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidSemiMajorAxis(0))
        ));
    }

    #[test]
    fn kepler_eccentricity_boundary() {
        // e = 0.999999 ok
        assert!(
            KeplerianElements::new(1, 999_999, 0, 0, 0, 0, ReferenceFrame::EclipticJ2000).is_ok()
        );
        // e = 1.0 rejected
        assert!(matches!(
            KeplerianElements::new(1, 1_000_000, 0, 0, 0, 0, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidEccentricity(1_000_000))
        ));
    }

    #[test]
    fn kepler_inclination_boundary() {
        assert!(
            KeplerianElements::new(1, 0, 180_000, 0, 0, 0, ReferenceFrame::EclipticJ2000).is_ok()
        );
        assert!(matches!(
            KeplerianElements::new(1, 0, 180_001, 0, 0, 0, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidAngle { .. })
        ));
        assert!(matches!(
            KeplerianElements::new(1, 0, -1, 0, 0, 0, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidAngle { .. })
        ));
    }

    #[test]
    fn kepler_circle_angle_boundary() {
        // node 359_999 ok, 360_000 rejected
        assert!(
            KeplerianElements::new(1, 0, 0, 359_999, 0, 0, ReferenceFrame::EclipticJ2000).is_ok()
        );
        assert!(matches!(
            KeplerianElements::new(1, 0, 0, 360_000, 0, 0, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidAngle {
                field: "lon_ascending_node",
                ..
            })
        ));
        assert!(matches!(
            KeplerianElements::new(1, 0, 0, 0, 360_000, 0, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidAngle {
                field: "arg_periapsis",
                ..
            })
        ));
        assert!(matches!(
            KeplerianElements::new(1, 0, 0, 0, 0, -5, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidAngle {
                field: "mean_anomaly_epoch",
                ..
            })
        ));
    }

    // ----- ParentRef -----

    #[test]
    fn parent_ref_basic() {
        let p = ParentRef::new(BodyId::new("sol").unwrap(), ReferenceFrame::EclipticJ2000);
        assert_eq!(p.target().as_str(), "sol");
        assert_eq!(p.frame(), &ReferenceFrame::EclipticJ2000);
    }

    // ----- RotationModel -----

    #[test]
    fn rotation_valid_and_retrograde() {
        assert!(RotationModel::new(86_164, 23_440, ReferenceFrame::EclipticJ2000).is_ok());
        assert!(RotationModel::new(-20_996_800, 177_000, ReferenceFrame::EclipticJ2000).is_ok());
    }

    #[test]
    fn rotation_zero_period_rejected() {
        assert!(matches!(
            RotationModel::new(0, 0, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidRotationPeriod)
        ));
    }

    #[test]
    fn rotation_tilt_boundary() {
        assert!(RotationModel::new(1, 180_000, ReferenceFrame::EclipticJ2000).is_ok());
        assert!(matches!(
            RotationModel::new(1, 180_001, ReferenceFrame::EclipticJ2000),
            Err(Error::InvalidAngle {
                field: "axial_tilt",
                ..
            })
        ));
    }
}
