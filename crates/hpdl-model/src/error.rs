/// All errors produced by `hpdl-model` constructors.
///
/// Every fallible constructor in this crate returns `Result<T, Error>`.
/// Variants are exhaustive — callers must handle each explicitly.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A body identifier was empty, too long (> 128 chars), or contained
    /// characters outside `[A-Za-z0-9_-]`.
    #[error("invalid body id {0:?}: must be 1–128 chars, alphanumeric, '-', or '_'")]
    InvalidId(String),

    /// A trait name was empty, too long (> 64 chars), or contained characters
    /// outside `[A-Za-z0-9_\-.]`.
    #[error("invalid trait name {0:?}: must be 1–64 chars, alphanumeric, '-', '_', or '.'")]
    InvalidTraitName(String),

    /// A module reference had an empty name or version, or exceeded length limits.
    #[error("invalid module ref — name and version must each be 1–128 / 1–64 non-empty chars")]
    InvalidModuleRef,

    /// A `StatusDeclaration` had `scale_min >= scale_max`.
    #[error(
        "invalid scale: scale_min ({scale_min}) must be strictly less than scale_max ({scale_max})"
    )]
    InvalidScale {
        /// The lower bound that was provided.
        scale_min: i64,
        /// The upper bound that was provided.
        scale_max: i64,
    },

    /// An `ActionDeclaration` attempted to write to a status field.
    ///
    /// Status values are derived, never written by actions.
    #[error("action {action_name:?} declares a write to status field {field:?}: status writes are forbidden")]
    StatusWriteForbidden {
        /// The name of the action that made the forbidden write declaration.
        action_name: String,
        /// The status field path that was declared as a write target.
        field: String,
    },

    /// A `MigrationDeclaration` had `to_version <= from_version`.
    #[error(
        "invalid migration versions: to_version ({to}) must be strictly greater than from_version ({from})"
    )]
    InvalidMigrationVersions {
        /// The source schema version.
        from: u32,
        /// The target schema version (must be > `from`).
        to: u32,
    },

    /// A `CelestialBody` contained two actions with the same name.
    #[error("duplicate action name {0:?} within the same body")]
    DuplicateActionName(String),

    /// A `CelestialBody` contained two status declarations with the same name.
    #[error("duplicate status name {0:?} within the same body")]
    DuplicateStatusName(String),

    /// A `PlanetDefinition` had an empty or too-long `language_version`.
    #[error("invalid language_version {0:?}: must be 1–32 non-empty chars")]
    InvalidLanguageVersion(String),

    /// A status or action name was empty, too long, or used invalid characters.
    #[error("invalid name {0:?}: must be 1–128 chars, alphanumeric, '-', or '_'")]
    InvalidName(String),

    /// A custom reference-frame or celestial-kind name was empty, too long
    /// (> 64 chars), or contained characters outside `[A-Za-z0-9_\-.]`.
    #[error("invalid frame/kind name {0:?}: must be 1–64 chars, alphanumeric, '-', '_', or '.'")]
    InvalidFrameName(String),

    /// A `KeplerianElements` had a non-positive semi-major axis.
    #[error("invalid semi-major axis: must be strictly positive, got {0} m")]
    InvalidSemiMajorAxis(u128),

    /// A `KeplerianElements` eccentricity was outside the v1 bound `[0, 1)`.
    ///
    /// Eccentricity is stored as `e × 1_000_000`, so the valid range is
    /// `0..1_000_000`. Parabolic (`e = 1`) and hyperbolic (`e > 1`) orbits are
    /// rejected in v1 (see OD-07.3), never clamped.
    #[error("invalid eccentricity: {0} (×1e-6) is outside [0, 1_000_000); parabolic/hyperbolic orbits are not supported in v1")]
    InvalidEccentricity(u32),

    /// An orbital or rotation angle was outside its documented bound.
    ///
    /// Angles are stored in milli-degrees. Inclination and axial tilt are
    /// bounded to `[0, 180_000]`; longitude of ascending node, argument of
    /// periapsis, and mean anomaly at epoch are bounded to `[0, 360_000)`.
    #[error("invalid angle for {field:?}: {value} milli-degrees is outside the allowed range")]
    InvalidAngle {
        /// The name of the angular field that was out of range.
        field: &'static str,
        /// The provided value in milli-degrees.
        value: i32,
    },

    /// A `RotationModel` had a sidereal period of zero.
    ///
    /// Negative periods are allowed (retrograde); zero is not.
    #[error("invalid sidereal rotation period: must be non-zero (negative = retrograde)")]
    InvalidRotationPeriod,

    /// An environment name (gas id, asset name, or custom kind/tier) was empty,
    /// too long (> 32 chars), or contained characters outside `[A-Za-z0-9_\-.]`.
    #[error("invalid environment name {0:?}: must be 1–32 chars, alphanumeric, '-', '_', or '.'")]
    InvalidEnvironmentName(String),

    /// A `Composition` was declared with no gas components.
    #[error("empty composition: a declared atmosphere must have at least one gas component")]
    EmptyComposition,

    /// A single gas component's mole fraction exceeded 1_000_000 ppm (100%).
    #[error("composition component fraction {0} ppm exceeds 1_000_000 (100%)")]
    CompositionFractionOutOfRange(u32),

    /// A `Composition`'s component ppm values did not sum to 1_000_000 within
    /// the declared tolerance.
    #[error("composition sum {sum} ppm is outside 1_000_000 ± {tolerance} ppm")]
    CompositionSumOutOfTolerance {
        /// The actual sum of all component ppm values.
        sum: u64,
        /// The declared tolerance in ppm.
        tolerance: u32,
    },

    /// An `AtmosphereProfile` was declared with zero surface pressure.
    ///
    /// A body with no atmosphere omits the profile entirely; a declared
    /// atmosphere must have a positive pressure.
    #[error("invalid atmosphere: surface pressure must be strictly positive (got 0 Pa)")]
    ZeroPressure,

    /// A `ThermalProfile` violated `min <= mean <= max`.
    #[error(
        "invalid thermal profile: require min_mk ({min}) <= mean_mk ({mean}) <= max_mk ({max})"
    )]
    InvalidTemperatureOrder {
        /// Minimum surface temperature in milli-kelvin.
        min: u64,
        /// Mean surface temperature in milli-kelvin.
        mean: u64,
        /// Maximum surface temperature in milli-kelvin.
        max: u64,
    },

    /// A hydrosphere or cryosphere surface coverage exceeded 1_000_000 ppm (100%).
    #[error("invalid surface coverage {0} ppm: must be in [0, 1_000_000]")]
    CoverageOutOfRange(u32),

    /// An `AssetRef` digest was not a 64-character lowercase hexadecimal string.
    #[error("invalid asset digest {0:?}: must be exactly 64 lowercase hex characters (SHA-256)")]
    InvalidAssetDigest(String),

    /// A custom status-dimension or band-label name was empty, too long (> 64
    /// chars), or contained characters outside `[A-Za-z0-9_\-.]`.
    #[error("invalid status name {0:?}: must be 1–64 chars, alphanumeric, '-', '_', or '.'")]
    InvalidStatusDimensionName(String),

    /// A `CutPoints` declaration violated an Amendment A1 rule.
    #[error("invalid cut points: {reason}")]
    InvalidCutPoints {
        /// Human-readable reason the cut points were rejected.
        reason: String,
    },

    /// A score passed to classification was outside the cut-points scale.
    ///
    /// Out-of-range scores are rejected, never clamped.
    #[error("score {score} is outside the status scale [{min}, {max}]")]
    ScoreOutOfRange {
        /// The score that was out of range.
        score: i64,
        /// The inclusive scale minimum.
        min: i64,
        /// The inclusive scale maximum.
        max: i64,
    },

    /// An `EvaluationWindow` was declared with a zero span.
    #[error("invalid evaluation window: span must be strictly positive")]
    ZeroEvaluationWindow,

    /// A `BaselineRef` had an invalid logical name.
    #[error("invalid baseline ref name {0:?}: must be 1–64 chars, alphanumeric, '-', '_', or '.'")]
    InvalidBaselineRef(String),

    /// A capability token was empty, too long (> 64 chars), or contained
    /// characters outside `[A-Za-z0-9_\-.]`.
    #[error("invalid capability {0:?}: must be 1–64 chars, alphanumeric, '-', '_', or '.'")]
    InvalidCapability(String),

    /// A read or write field path was empty or had an empty/invalid dotted
    /// segment (segments must be `[A-Za-z0-9_\-]`).
    #[error("invalid field path {0:?}: must be a non-empty dotted path of valid segments")]
    InvalidFieldPath(String),

    /// A world-law domain token was empty, too long (> 64 chars), contained
    /// characters outside `[A-Za-z0-9_\-.]`, or had an empty dotted segment.
    #[error("invalid law domain {0:?}: must be 1–64 chars, dotted segments of alphanumeric, '-', or '_'")]
    InvalidLawDomain(String),

    /// A `WorldLaw` was declared claiming no domains (or a `Replaces` set was empty).
    #[error("invalid world law: a law must claim at least one domain")]
    EmptyLawDomains,

    /// A `WorldLaw` (or its `Replaces` set) claimed more domains than allowed.
    #[error("invalid world law: {0} domains exceeds the maximum per law")]
    TooManyLawDomains(usize),

    /// A `MagicBridge` was declared with no source dimensions.
    #[error("invalid magic bridge: at least one source dimension is required")]
    EmptyBridgeSources,

    /// A `MagicBridge` declared more source dimensions than allowed.
    #[error("invalid magic bridge: {0} source dimensions exceeds the maximum")]
    TooManyBridgeSources(usize),

    /// A `MagicBridge` targeted a non-physical dimension.
    ///
    /// A bridge exists to let magic reach physical state, so its target must be
    /// in the reserved `physics` namespace.
    #[error("invalid magic bridge: target {0:?} is not a physical (physics.*) dimension")]
    NonPhysicalBridgeTarget(String),

    /// A `PackageId` version was a mutable label (`latest`, a range operator, a
    /// wildcard, or embedded whitespace) rather than an exact pin.
    ///
    /// The standard library never resolves a mutable reference; only an exact
    /// version is accepted.
    #[error(
        "mutable package version {0:?}: packages must pin an exact version, never a label or range"
    )]
    MutablePackageVersion(String),

    /// A `PackageManifest` declared more direct dependencies than allowed.
    #[error("invalid package manifest: {0} dependencies exceeds the maximum")]
    TooManyDependencies(usize),

    /// A `PackageManifest` listed itself as a dependency.
    #[error("invalid package manifest: package {0:?} depends on itself")]
    SelfDependency(String),
}
