//! Planetary environment declaration types (HPDL feature 08).
//!
//! These types are pure, immutable **declarations** of a body's environment:
//! atmosphere, thermal/climate profile, hydrosphere, cryosphere, biosphere, and
//! content-addressed visual asset references. Declaring an atmosphere is not a
//! climate solver; declaring a biosphere does not simulate an ecosystem. The
//! external HORIZON engine alone evolves state.
//!
//! # Design constraints honoured here
//!
//! - **No `f32`/`f64`.** Mole fractions and surface coverage are parts-per-million
//!   (`u32`); pressure is pascals (`u64`); temperature is milli-kelvin (`u64`).
//! - **No Earth defaults.** Absence lives in the `Option` fields; a missing
//!   biosphere is the explicit [`Biosphere::Absent`]/[`Biosphere::Unknown`] value,
//!   never a zero score.
//! - **Checked construction.** Every constructor validates and returns [`Error`].
//! - **Deterministic.** Gas components and asset references use `BTreeMap`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::{Error, ModuleRef};

/// Maximum length for an environment name (gas id, asset name, custom kind/tier).
const MAX_NAME_LEN: usize = 32;

/// Parts-per-million scale: a mole fraction or coverage of `1.0` is `1_000_000`.
pub const PPM_SCALE: u32 = 1_000_000;

/// Length of a lowercase hex SHA-256 digest string.
const DIGEST_LEN: usize = 64;

/// Validate an environment name (gas id, asset name, or custom kind/tier label).
///
/// Valid names are 1–32 characters drawn from `[A-Za-z0-9_\-.]`.
fn validate_env_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > MAX_NAME_LEN
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(Error::InvalidEnvironmentName(name.to_owned()));
    }
    Ok(())
}

/// A validated gas identifier (e.g. `n2`, `co2`, `arcane.aether`).
///
/// # Example
///
/// ```
/// use hpdl_model::GasId;
///
/// assert_eq!(GasId::new("co2").unwrap().as_str(), "co2");
/// assert!(GasId::new("bad gas!").is_err());
/// ```
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, SchemaWrite,
    SchemaRead,
)]
pub struct GasId(String);

impl GasId {
    /// Construct a validated `GasId`.
    ///
    /// Returns [`Error::InvalidEnvironmentName`] if the name is empty, longer than
    /// 32 characters, or contains characters outside `[A-Za-z0-9_\-.]`.
    pub fn new(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        validate_env_name(&name)?;
        Ok(Self(name))
    }

    /// Return the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated content digest: a lowercase hex SHA-256 (exactly 64 hex chars).
///
/// # Example
///
/// ```
/// use hpdl_model::Digest;
///
/// let d = Digest::new("a".repeat(64)).unwrap();
/// assert_eq!(d.as_str().len(), 64);
///
/// assert!(Digest::new("tooshort").is_err());
/// assert!(Digest::new("A".repeat(64)).is_err()); // uppercase rejected
/// ```
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, SchemaWrite,
    SchemaRead,
)]
pub struct Digest(String);

impl Digest {
    /// Construct a validated `Digest`.
    ///
    /// Returns [`Error::InvalidAssetDigest`] unless the string is exactly 64
    /// characters, all of which are lowercase hexadecimal (`0-9a-f`).
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if s.len() != DIGEST_LEN
            || !s
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            return Err(Error::InvalidAssetDigest(s));
        }
        Ok(Self(s))
    }

    /// Return the underlying digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A normalized gas composition: mole fractions in parts-per-million.
///
/// Components sum to `1_000_000` ppm (100%) within an explicit, caller-supplied
/// tolerance so the normalization rule is auditable. The 12-plus atmosphere
/// components are *inputs*, not weighted scores — `Composition` records fractions,
/// it does not compute a habitability or pollution number.
///
/// # Example
///
/// ```
/// use hpdl_model::{Composition, GasId};
///
/// // A nitrogen/oxygen/argon mix summing to exactly 1_000_000 ppm.
/// let comp = Composition::new(
///     [
///         (GasId::new("n2").unwrap(), 780_800),
///         (GasId::new("o2").unwrap(), 209_500),
///         (GasId::new("ar").unwrap(), 9_700),
///     ],
///     0, // zero tolerance: must sum exactly
/// ).unwrap();
/// assert_eq!(comp.component_ppm(&GasId::new("o2").unwrap()), Some(209_500));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct Composition {
    components: BTreeMap<GasId, u32>,
    tolerance_ppm: u32,
}

impl Composition {
    /// Construct a validated `Composition` from `(GasId, ppm)` pairs.
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyComposition`] if no components are provided.
    /// - [`Error::CompositionFractionOutOfRange`] if any component exceeds
    ///   `1_000_000` ppm.
    /// - [`Error::CompositionSumOutOfTolerance`] if the checked sum of all
    ///   components is outside `1_000_000 ± tolerance_ppm`.
    pub fn new(
        components: impl IntoIterator<Item = (GasId, u32)>,
        tolerance_ppm: u32,
    ) -> Result<Self, Error> {
        let mut map: BTreeMap<GasId, u32> = BTreeMap::new();
        let mut sum: u64 = 0;
        for (gas, ppm) in components {
            if ppm > PPM_SCALE {
                return Err(Error::CompositionFractionOutOfRange(ppm));
            }
            // Later duplicate keys overwrite; sum reflects the stored values.
            map.insert(gas, ppm);
        }
        if map.is_empty() {
            return Err(Error::EmptyComposition);
        }
        for ppm in map.values() {
            // Checked accumulation; u64 cannot overflow for realistic inputs but
            // we never rely on that silently.
            sum = sum
                .checked_add(u64::from(*ppm))
                .ok_or(Error::CompositionSumOutOfTolerance {
                    sum: u64::MAX,
                    tolerance: tolerance_ppm,
                })?;
        }
        let target = u64::from(PPM_SCALE);
        let tol = u64::from(tolerance_ppm);
        let low = target.saturating_sub(tol);
        let high = target.saturating_add(tol);
        if sum < low || sum > high {
            return Err(Error::CompositionSumOutOfTolerance {
                sum,
                tolerance: tolerance_ppm,
            });
        }
        Ok(Self {
            components: map,
            tolerance_ppm,
        })
    }

    /// The ppm of a specific gas, if present.
    pub fn component_ppm(&self, gas: &GasId) -> Option<u32> {
        self.components.get(gas).copied()
    }

    /// Iterate over `(gas, ppm)` pairs in deterministic (sorted) order.
    pub fn components(&self) -> impl Iterator<Item = (&GasId, u32)> {
        self.components.iter().map(|(g, p)| (g, *p))
    }

    /// The declared normalization tolerance in ppm.
    pub fn tolerance_ppm(&self) -> u32 {
        self.tolerance_ppm
    }

    /// The checked sum of all component ppm values.
    pub fn sum_ppm(&self) -> u64 {
        self.components.values().map(|p| u64::from(*p)).sum()
    }
}

/// A declared atmosphere: surface pressure, gas composition, and an optional
/// versioned climate-model contract reference.
///
/// # Example
///
/// ```
/// use hpdl_model::{AtmosphereProfile, Composition, GasId};
///
/// let comp = Composition::new([(GasId::new("co2").unwrap(), 1_000_000)], 0).unwrap();
/// let atm = AtmosphereProfile::new(610, comp, None).unwrap(); // ~Mars surface pressure
/// assert_eq!(atm.surface_pressure_pa(), 610);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct AtmosphereProfile {
    surface_pressure_pa: u64,
    composition: Composition,
    climate_model: Option<ModuleRef>,
}

impl AtmosphereProfile {
    /// Construct a validated `AtmosphereProfile`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ZeroPressure`] if `surface_pressure_pa == 0`.
    pub fn new(
        surface_pressure_pa: u64,
        composition: Composition,
        climate_model: Option<ModuleRef>,
    ) -> Result<Self, Error> {
        if surface_pressure_pa == 0 {
            return Err(Error::ZeroPressure);
        }
        Ok(Self {
            surface_pressure_pa,
            composition,
            climate_model,
        })
    }

    /// Surface pressure in pascals.
    pub fn surface_pressure_pa(&self) -> u64 {
        self.surface_pressure_pa
    }

    /// The gas composition.
    pub fn composition(&self) -> &Composition {
        &self.composition
    }

    /// The declared climate-model contract reference, if any.
    pub fn climate_model(&self) -> Option<&ModuleRef> {
        self.climate_model.as_ref()
    }
}

/// Surface temperature profile in milli-kelvin (absolute temperature).
///
/// `Temperature` is distinct from a temperature *delta*; this type is absolute
/// temperature only, so values are unsigned.
///
/// # Example
///
/// ```
/// use hpdl_model::ThermalProfile;
///
/// let t = ThermalProfile::new(288_000, 184_000, 331_000).unwrap(); // ~Earth-ish
/// assert_eq!(t.mean_mk(), 288_000);
///
/// assert!(ThermalProfile::new(100, 200, 300).is_err()); // mean < min
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct ThermalProfile {
    mean_mk: u64,
    min_mk: u64,
    max_mk: u64,
}

impl ThermalProfile {
    /// Construct a validated `ThermalProfile`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidTemperatureOrder`] unless `min <= mean <= max`.
    pub fn new(mean_mk: u64, min_mk: u64, max_mk: u64) -> Result<Self, Error> {
        if !(min_mk <= mean_mk && mean_mk <= max_mk) {
            return Err(Error::InvalidTemperatureOrder {
                min: min_mk,
                mean: mean_mk,
                max: max_mk,
            });
        }
        Ok(Self {
            mean_mk,
            min_mk,
            max_mk,
        })
    }

    /// Mean surface temperature in milli-kelvin.
    pub fn mean_mk(&self) -> u64 {
        self.mean_mk
    }

    /// Minimum surface temperature in milli-kelvin.
    pub fn min_mk(&self) -> u64 {
        self.min_mk
    }

    /// Maximum surface temperature in milli-kelvin.
    pub fn max_mk(&self) -> u64 {
        self.max_mk
    }
}

/// Validate a `[0, 1_000_000]` ppm coverage value.
fn validate_coverage(ppm: u32) -> Result<(), Error> {
    if ppm > PPM_SCALE {
        return Err(Error::CoverageOutOfRange(ppm));
    }
    Ok(())
}

/// Declared surface liquid coverage.
///
/// # Example
///
/// ```
/// use hpdl_model::{Hydrosphere, GasId};
///
/// let h = Hydrosphere::new(708_000, Some(GasId::new("h2o").unwrap())).unwrap();
/// assert_eq!(h.surface_coverage_ppm(), 708_000);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct Hydrosphere {
    surface_coverage_ppm: u32,
    dominant_liquid: Option<GasId>,
}

impl Hydrosphere {
    /// Construct a validated `Hydrosphere`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CoverageOutOfRange`] if coverage exceeds `1_000_000` ppm.
    pub fn new(surface_coverage_ppm: u32, dominant_liquid: Option<GasId>) -> Result<Self, Error> {
        validate_coverage(surface_coverage_ppm)?;
        Ok(Self {
            surface_coverage_ppm,
            dominant_liquid,
        })
    }

    /// Surface coverage in ppm (`0..=1_000_000`).
    pub fn surface_coverage_ppm(&self) -> u32 {
        self.surface_coverage_ppm
    }

    /// The dominant surface liquid, if declared.
    pub fn dominant_liquid(&self) -> Option<&GasId> {
        self.dominant_liquid.as_ref()
    }
}

/// Declared ice/snow surface coverage.
///
/// # Example
///
/// ```
/// use hpdl_model::Cryosphere;
///
/// let c = Cryosphere::new(100_000).unwrap();
/// assert_eq!(c.surface_coverage_ppm(), 100_000);
/// assert!(Cryosphere::new(1_000_001).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct Cryosphere {
    surface_coverage_ppm: u32,
}

impl Cryosphere {
    /// Construct a validated `Cryosphere`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CoverageOutOfRange`] if coverage exceeds `1_000_000` ppm.
    pub fn new(surface_coverage_ppm: u32) -> Result<Self, Error> {
        validate_coverage(surface_coverage_ppm)?;
        Ok(Self {
            surface_coverage_ppm,
        })
    }

    /// Surface coverage in ppm (`0..=1_000_000`).
    pub fn surface_coverage_ppm(&self) -> u32 {
        self.surface_coverage_ppm
    }
}

/// The declared complexity tier of a present biosphere.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub enum ComplexityTier {
    /// Microbial life only.
    Microbial,
    /// Simple multicellular life.
    Simple,
    /// Complex multicellular life.
    Complex,
    /// Intelligent / tool-using life.
    Intelligent,
    /// A namespaced, validated custom tier.
    Other(String),
}

impl ComplexityTier {
    /// Construct a validated [`ComplexityTier::Other`].
    ///
    /// Returns [`Error::InvalidEnvironmentName`] on an invalid name.
    pub fn other(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        validate_env_name(&name)?;
        Ok(Self::Other(name))
    }
}

/// The declared biosphere state of a body.
///
/// `Absent` and `Unknown` are **distinct, explicit** states. A missing biosphere
/// is "not applicable" ([`Biosphere::Absent`]) or "undetermined"
/// ([`Biosphere::Unknown`]) — it is never represented as a zero score.
///
/// # Example
///
/// ```
/// use hpdl_model::{Biosphere, ComplexityTier};
///
/// let dead = Biosphere::Absent;
/// let living = Biosphere::Present(ComplexityTier::Microbial);
/// assert_ne!(dead, living);
/// assert_ne!(Biosphere::Absent, Biosphere::Unknown);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub enum Biosphere {
    /// No native biosphere (not applicable).
    Absent,
    /// Biosphere presence is undetermined.
    Unknown,
    /// A biosphere is present, with a declared complexity tier.
    Present(ComplexityTier),
}

/// The kind of a referenced visual asset.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub enum AssetKind {
    /// A surface texture.
    Texture,
    /// A 3D mesh.
    Mesh,
    /// A terrain heightmap.
    Heightmap,
    /// A namespaced, validated custom kind.
    Other(String),
}

impl AssetKind {
    /// Construct a validated [`AssetKind::Other`].
    ///
    /// Returns [`Error::InvalidEnvironmentName`] on an invalid name.
    pub fn other(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        validate_env_name(&name)?;
        Ok(Self::Other(name))
    }
}

/// A content-addressed reference to a visual asset.
///
/// `AssetRef` records a logical name, a kind, and a required content digest. It
/// does **not** fetch, decode, or verify the referenced bytes — only that a
/// well-formed digest is present.
///
/// # Example
///
/// ```
/// use hpdl_model::{AssetRef, AssetKind, Digest};
///
/// let asset = AssetRef::new(
///     "surface-albedo",
///     AssetKind::Texture,
///     Digest::new("0".repeat(64)).unwrap(),
/// ).unwrap();
/// assert_eq!(asset.name(), "surface-albedo");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct AssetRef {
    name: String,
    kind: AssetKind,
    digest: Digest,
}

impl AssetRef {
    /// Construct a validated `AssetRef`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEnvironmentName`] if the logical name is invalid.
    /// (The digest is already validated by [`Digest::new`].)
    pub fn new(name: impl Into<String>, kind: AssetKind, digest: Digest) -> Result<Self, Error> {
        let name = name.into();
        validate_env_name(&name)?;
        Ok(Self { name, kind, digest })
    }

    /// The logical asset name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The asset kind.
    pub fn kind(&self) -> &AssetKind {
        &self.kind
    }

    /// The content digest.
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
}

/// A complete, immutable declaration of a body's environment.
///
/// Every sub-profile except the biosphere is `Option` (absent = undeclared); the
/// biosphere is a tri-state value because absence is itself meaningful.
///
/// # Example
///
/// ```
/// use hpdl_model::{
///     AtmosphereProfile, Biosphere, Composition, EnvironmentDefinition, GasId,
/// };
///
/// let comp = Composition::new([(GasId::new("n2").unwrap(), 1_000_000)], 0).unwrap();
/// let atm = AtmosphereProfile::new(101_325, comp, None).unwrap();
///
/// let env = EnvironmentDefinition::new(Biosphere::Unknown)
///     .with_atmosphere(atm);
/// assert!(env.atmosphere().is_some());
/// assert_eq!(env.biosphere(), &Biosphere::Unknown);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct EnvironmentDefinition {
    #[serde(default)]
    atmosphere: Option<AtmosphereProfile>,
    #[serde(default)]
    thermal: Option<ThermalProfile>,
    #[serde(default)]
    hydrosphere: Option<Hydrosphere>,
    #[serde(default)]
    cryosphere: Option<Cryosphere>,
    biosphere: Biosphere,
    #[serde(default)]
    assets: BTreeMap<String, AssetRef>,
}

impl EnvironmentDefinition {
    /// Construct an `EnvironmentDefinition` with the given biosphere state and no
    /// other declared sub-profiles. Use the `with_*` setters to add them.
    pub fn new(biosphere: Biosphere) -> Self {
        Self {
            atmosphere: None,
            thermal: None,
            hydrosphere: None,
            cryosphere: None,
            biosphere,
            assets: BTreeMap::new(),
        }
    }

    /// Attach an [`AtmosphereProfile`] (builder style).
    #[must_use]
    pub fn with_atmosphere(mut self, atmosphere: AtmosphereProfile) -> Self {
        self.atmosphere = Some(atmosphere);
        self
    }

    /// Attach a [`ThermalProfile`] (builder style).
    #[must_use]
    pub fn with_thermal(mut self, thermal: ThermalProfile) -> Self {
        self.thermal = Some(thermal);
        self
    }

    /// Attach a [`Hydrosphere`] (builder style).
    #[must_use]
    pub fn with_hydrosphere(mut self, hydrosphere: Hydrosphere) -> Self {
        self.hydrosphere = Some(hydrosphere);
        self
    }

    /// Attach a [`Cryosphere`] (builder style).
    #[must_use]
    pub fn with_cryosphere(mut self, cryosphere: Cryosphere) -> Self {
        self.cryosphere = Some(cryosphere);
        self
    }

    /// Add an [`AssetRef`], keyed by its logical name (later entries overwrite).
    #[must_use]
    pub fn with_asset(mut self, asset: AssetRef) -> Self {
        self.assets.insert(asset.name().to_owned(), asset);
        self
    }

    /// The declared atmosphere, if any.
    pub fn atmosphere(&self) -> Option<&AtmosphereProfile> {
        self.atmosphere.as_ref()
    }

    /// The declared thermal profile, if any.
    pub fn thermal(&self) -> Option<&ThermalProfile> {
        self.thermal.as_ref()
    }

    /// The declared hydrosphere, if any.
    pub fn hydrosphere(&self) -> Option<&Hydrosphere> {
        self.hydrosphere.as_ref()
    }

    /// The declared cryosphere, if any.
    pub fn cryosphere(&self) -> Option<&Cryosphere> {
        self.cryosphere.as_ref()
    }

    /// The biosphere state (always present as an explicit tri-state value).
    pub fn biosphere(&self) -> &Biosphere {
        &self.biosphere
    }

    /// Iterate over asset references in deterministic (sorted-by-name) order.
    pub fn assets(&self) -> impl Iterator<Item = &AssetRef> {
        self.assets.values()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    // ----- names / digests -----

    #[test]
    fn gas_id_valid_and_invalid() {
        assert_eq!(GasId::new("co2").unwrap().as_str(), "co2");
        assert!(GasId::new("").is_err());
        assert!(GasId::new("has space").is_err());
        assert!(GasId::new("a".repeat(33)).is_err());
    }

    #[test]
    fn digest_rules() {
        assert!(Digest::new("a".repeat(64)).is_ok());
        assert!(Digest::new("0123456789abcdef".repeat(4)).is_ok());
        assert!(Digest::new("tooshort").is_err());
        assert!(Digest::new("a".repeat(63)).is_err());
        assert!(Digest::new("a".repeat(65)).is_err());
        assert!(Digest::new("A".repeat(64)).is_err()); // uppercase
        assert!(Digest::new("g".repeat(64)).is_err()); // non-hex
    }

    // ----- composition -----

    #[test]
    fn composition_exact_sum_ok() {
        let c = Composition::new(
            [
                (GasId::new("n2").unwrap(), 780_000),
                (GasId::new("o2").unwrap(), 220_000),
            ],
            0,
        )
        .unwrap();
        assert_eq!(c.sum_ppm(), 1_000_000);
        assert_eq!(c.component_ppm(&GasId::new("n2").unwrap()), Some(780_000));
    }

    #[test]
    fn composition_within_tolerance_ok() {
        // Sum = 999_990, tolerance 10 → ok.
        let c = Composition::new([(GasId::new("co2").unwrap(), 999_990)], 10);
        assert!(c.is_ok());
    }

    #[test]
    fn composition_just_outside_tolerance_rejected() {
        // Sum = 999_989, tolerance 10 → low bound 999_990 → rejected.
        let c = Composition::new([(GasId::new("co2").unwrap(), 999_989)], 10);
        assert!(matches!(c, Err(Error::CompositionSumOutOfTolerance { .. })));
    }

    #[test]
    fn composition_empty_rejected() {
        let c = Composition::new(std::iter::empty(), 0);
        assert!(matches!(c, Err(Error::EmptyComposition)));
    }

    #[test]
    fn composition_component_over_100pct_rejected() {
        let c = Composition::new([(GasId::new("co2").unwrap(), 1_000_001)], 0);
        assert!(matches!(
            c,
            Err(Error::CompositionFractionOutOfRange(1_000_001))
        ));
    }

    // ----- atmosphere / thermal -----

    #[test]
    fn atmosphere_zero_pressure_rejected() {
        let comp = Composition::new([(GasId::new("n2").unwrap(), 1_000_000)], 0).unwrap();
        assert!(matches!(
            AtmosphereProfile::new(0, comp, None),
            Err(Error::ZeroPressure)
        ));
    }

    #[test]
    fn thermal_ordering() {
        assert!(ThermalProfile::new(250, 250, 250).is_ok()); // equal ok
        assert!(ThermalProfile::new(288_000, 184_000, 331_000).is_ok());
        assert!(matches!(
            ThermalProfile::new(100, 200, 300),
            Err(Error::InvalidTemperatureOrder { .. })
        ));
        assert!(matches!(
            ThermalProfile::new(250, 100, 200),
            Err(Error::InvalidTemperatureOrder { .. })
        ));
    }

    // ----- hydrosphere / cryosphere -----

    #[test]
    fn coverage_bounds() {
        assert!(Hydrosphere::new(1_000_000, None).is_ok());
        assert!(matches!(
            Hydrosphere::new(1_000_001, None),
            Err(Error::CoverageOutOfRange(_))
        ));
        assert!(Cryosphere::new(0).is_ok());
        assert!(matches!(
            Cryosphere::new(2_000_000),
            Err(Error::CoverageOutOfRange(_))
        ));
    }

    // ----- biosphere -----

    #[test]
    fn biosphere_states_distinct() {
        assert_ne!(Biosphere::Absent, Biosphere::Unknown);
        assert_ne!(
            Biosphere::Present(ComplexityTier::Microbial),
            Biosphere::Present(ComplexityTier::Complex)
        );
        assert!(ComplexityTier::other("silicon-based").is_ok());
        assert!(ComplexityTier::other("bad tier!").is_err());
    }

    // ----- assets -----

    #[test]
    fn asset_ref_valid_and_digest_enforced() {
        let d = Digest::new("f".repeat(64)).unwrap();
        let a = AssetRef::new("albedo", AssetKind::Texture, d).unwrap();
        assert_eq!(a.name(), "albedo");
        assert!(matches!(a.kind(), AssetKind::Texture));
        assert!(AssetRef::new(
            "bad name!",
            AssetKind::Mesh,
            Digest::new("a".repeat(64)).unwrap()
        )
        .is_err());
        assert!(AssetKind::other("decal").is_ok());
    }

    // ----- aggregate -----

    #[test]
    fn environment_builder_and_accessors() {
        let comp = Composition::new([(GasId::new("n2").unwrap(), 1_000_000)], 0).unwrap();
        let env = EnvironmentDefinition::new(Biosphere::Present(ComplexityTier::Microbial))
            .with_atmosphere(AtmosphereProfile::new(101_325, comp, None).unwrap())
            .with_thermal(ThermalProfile::new(288_000, 184_000, 331_000).unwrap())
            .with_hydrosphere(Hydrosphere::new(708_000, None).unwrap())
            .with_cryosphere(Cryosphere::new(100_000).unwrap())
            .with_asset(
                AssetRef::new(
                    "albedo",
                    AssetKind::Texture,
                    Digest::new("a".repeat(64)).unwrap(),
                )
                .unwrap(),
            );

        assert!(env.atmosphere().is_some());
        assert!(env.thermal().is_some());
        assert!(env.hydrosphere().is_some());
        assert!(env.cryosphere().is_some());
        assert_eq!(env.assets().count(), 1);
        assert_eq!(
            env.biosphere(),
            &Biosphere::Present(ComplexityTier::Microbial)
        );
    }

    #[test]
    fn environment_absent_biosphere_is_a_value() {
        let env = EnvironmentDefinition::new(Biosphere::Absent);
        assert_eq!(env.biosphere(), &Biosphere::Absent);
        assert!(env.atmosphere().is_none());
    }
}
