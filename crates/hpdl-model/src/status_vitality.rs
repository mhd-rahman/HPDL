//! Multidimensional status / vitality declaration types (HPDL feature 09).
//!
//! A status is a **derived** indicator: a versioned evaluator measures planetary
//! state against a planet-specific baseline, over an evaluation window, and
//! classifies the result into an ordered band (Amendment A1 cut points). Nothing
//! in this module computes a status — it declares the *contract* for how one is
//! derived, and the *shape* of a result. The external HORIZON engine runs
//! evaluators.
//!
//! # Design constraints honoured here
//!
//! - **No `f32`/`f64`.** Scores, cut points, and windows are integers.
//! - **Dimensions are independent.** A missing dimension is "not applicable",
//!   never a zero score.
//! - **Statuses are derived, never authored.** The evaluator *output* contract
//!   (`StatusResult`) is produced by an evaluator, never by a definition; it lives
//!   in `hpdl-runtime-api` (relocated in feature 12). No constructor here lets a
//!   definition assign a score.
//! - **Out-of-range is rejected, never clamped** ([`CutPoints::classify`]).

use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::{Digest, Error, ModuleRef};

/// Maximum length for a custom dimension or band-label name.
const MAX_NAME_LEN: usize = 64;

/// Validate a status dimension / band-label / baseline name.
fn validate_status_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > MAX_NAME_LEN
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(Error::InvalidStatusDimensionName(name.to_owned()));
    }
    Ok(())
}

/// An independent status dimension.
///
/// Dimensions are **independent**: physical state, native-ecosystem vitality,
/// civilization viability, and optional supernatural stability do not combine into
/// a single number, and an absent dimension is "not applicable", never "zero".
///
/// # Example
///
/// ```
/// use hpdl_model::StatusDimension;
///
/// assert_eq!(StatusDimension::Physical, StatusDimension::Physical);
/// assert!(StatusDimension::custom("arcane.stability").is_ok());
/// assert!(StatusDimension::custom("bad dim!").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub enum StatusDimension {
    /// Physical state of the body.
    Physical,
    /// Native ecosystem vitality.
    NativeEcosystem,
    /// Civilization viability.
    Civilization,
    /// Optional supernatural / arcane stability.
    Supernatural,
    /// A namespaced, validated custom dimension.
    Custom(String),
}

impl StatusDimension {
    /// Construct a validated [`StatusDimension::Custom`].
    ///
    /// Returns [`Error::InvalidStatusDimensionName`] on an invalid name.
    pub fn custom(name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        validate_status_name(&name)?;
        Ok(Self::Custom(name))
    }
}

/// A reference to the versioned evaluator that derives a status.
///
/// This declares *which* evaluator computes the status; it is not executed here.
///
/// # Example
///
/// ```
/// use hpdl_model::{EvaluatorRef, ModuleRef};
///
/// let ev = EvaluatorRef::new(ModuleRef::new("hpdl.vitality.physical", "1.0").unwrap());
/// assert_eq!(ev.module().name(), "hpdl.vitality.physical");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct EvaluatorRef {
    module: ModuleRef,
}

impl EvaluatorRef {
    /// Wrap a versioned [`ModuleRef`] as an evaluator reference.
    pub fn new(module: ModuleRef) -> Self {
        Self { module }
    }

    /// The underlying versioned module reference.
    pub fn module(&self) -> &ModuleRef {
        &self.module
    }
}

/// A planet-specific, content-addressed baseline reference.
///
/// Baselines are per-planet (not global): the evaluator measures state against the
/// baseline the body declares. The referenced bytes are not fetched or verified
/// here — only the reference's shape is validated.
///
/// # Example
///
/// ```
/// use hpdl_model::{BaselineRef, Digest};
///
/// let b = BaselineRef::new("preindustrial", Digest::new("a".repeat(64)).unwrap()).unwrap();
/// assert_eq!(b.name(), "preindustrial");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct BaselineRef {
    name: String,
    digest: Digest,
}

impl BaselineRef {
    /// Construct a validated `BaselineRef`.
    ///
    /// Returns [`Error::InvalidBaselineRef`] if the logical name is invalid.
    pub fn new(name: impl Into<String>, digest: Digest) -> Result<Self, Error> {
        let name = name.into();
        if name.is_empty()
            || name.len() > MAX_NAME_LEN
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(Error::InvalidBaselineRef(name));
        }
        Ok(Self { name, digest })
    }

    /// The logical baseline name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The content digest of the baseline.
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
}

/// A validated band label.
fn validate_band_label(name: &str) -> Result<(), Error> {
    validate_status_name(name)
}

/// Ordered lower-bound cut points partitioning a status scale into named bands
/// (Amendment A1).
///
/// Each band `i` is the half-open interval `[cut_i, cut_{i+1})`; the final band
/// includes the scale maximum. The first cut must equal the scale minimum, cuts
/// must be strictly increasing, and every cut must lie within `[scale_min,
/// scale_max]`.
///
/// # Example
///
/// ```
/// use hpdl_model::CutPoints;
///
/// // Amendment A1 example: scale 0..=100, cuts {0,25,50,75}.
/// let cp = CutPoints::new(0, 100, [
///     (0, "critical".to_string()),
///     (25, "degraded".to_string()),
///     (50, "stressed".to_string()),
///     (75, "stable".to_string()),
/// ]).unwrap();
///
/// assert_eq!(cp.classify(24).unwrap(), "critical");  // [0,25)
/// assert_eq!(cp.classify(25).unwrap(), "degraded");  // [25,50)
/// assert_eq!(cp.classify(49).unwrap(), "degraded");
/// assert_eq!(cp.classify(100).unwrap(), "stable");   // last band includes max
///
/// // Out-of-range is rejected, never clamped.
/// assert!(cp.classify(101).is_err());
/// assert!(cp.classify(-1).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct CutPoints {
    scale_min: i64,
    scale_max: i64,
    cuts: Vec<(i64, String)>,
}

impl CutPoints {
    /// Construct validated `CutPoints`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCutPoints`] if `scale_min >= scale_max`, the cut
    /// list is empty, the first cut does not equal `scale_min`, any cut is out of
    /// range, any band label is invalid, or the cuts are not strictly increasing.
    pub fn new(
        scale_min: i64,
        scale_max: i64,
        cuts: impl IntoIterator<Item = (i64, String)>,
    ) -> Result<Self, Error> {
        if scale_min >= scale_max {
            return Err(Error::InvalidCutPoints {
                reason: format!("scale_min ({scale_min}) must be < scale_max ({scale_max})"),
            });
        }
        let cuts: Vec<(i64, String)> = cuts.into_iter().collect();
        if cuts.is_empty() {
            return Err(Error::InvalidCutPoints {
                reason: "at least one cut point is required".to_string(),
            });
        }
        // First cut must equal scale_min.
        if let Some((first, _)) = cuts.first() {
            if *first != scale_min {
                return Err(Error::InvalidCutPoints {
                    reason: format!("first cut ({first}) must equal scale_min ({scale_min})"),
                });
            }
        }
        let mut prev: Option<i64> = None;
        for (value, label) in &cuts {
            validate_band_label(label)?;
            if *value < scale_min || *value > scale_max {
                return Err(Error::InvalidCutPoints {
                    reason: format!("cut {value} is outside the scale [{scale_min}, {scale_max}]"),
                });
            }
            if let Some(p) = prev {
                if *value <= p {
                    return Err(Error::InvalidCutPoints {
                        reason: format!("cuts must be strictly increasing: {p} then {value}"),
                    });
                }
            }
            prev = Some(*value);
        }
        Ok(Self {
            scale_min,
            scale_max,
            cuts,
        })
    }

    /// Classify an in-range score into its band label.
    ///
    /// Band `i` is `[cut_i, cut_{i+1})`; the final band includes `scale_max`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ScoreOutOfRange`] if `score` is outside `[scale_min,
    /// scale_max]`. Out-of-range scores are **rejected, never clamped**.
    pub fn classify(&self, score: i64) -> Result<&str, Error> {
        if score < self.scale_min || score > self.scale_max {
            return Err(Error::ScoreOutOfRange {
                score,
                min: self.scale_min,
                max: self.scale_max,
            });
        }
        // Find the last cut whose value is <= score. Cuts are strictly increasing
        // and the first equals scale_min, so for any in-range score at least the
        // first cut matches.
        let mut chosen: &str = "";
        for (value, label) in &self.cuts {
            if *value <= score {
                chosen = label;
            } else {
                break;
            }
        }
        Ok(chosen)
    }

    /// The inclusive scale minimum.
    pub fn scale_min(&self) -> i64 {
        self.scale_min
    }

    /// The inclusive scale maximum.
    pub fn scale_max(&self) -> i64 {
        self.scale_max
    }

    /// Iterate the ordered `(cut, label)` pairs.
    pub fn cuts(&self) -> impl Iterator<Item = (i64, &str)> {
        self.cuts.iter().map(|(v, l)| (*v, l.as_str()))
    }
}

/// The temporal window an evaluator aggregates over.
///
/// # Example
///
/// ```
/// use hpdl_model::EvaluationWindow;
///
/// let w = EvaluationWindow::new(1000, Some("surface".to_string())).unwrap();
/// assert_eq!(w.span(), 1000);
/// assert!(EvaluationWindow::new(0, None).is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct EvaluationWindow {
    span: u64,
    scope: Option<String>,
}

impl EvaluationWindow {
    /// Construct a validated `EvaluationWindow`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ZeroEvaluationWindow`] if `span == 0`. Returns
    /// [`Error::InvalidStatusDimensionName`] if a scope string is invalid.
    pub fn new(span: u64, scope: Option<String>) -> Result<Self, Error> {
        if span == 0 {
            return Err(Error::ZeroEvaluationWindow);
        }
        if let Some(s) = &scope {
            validate_status_name(s)?;
        }
        Ok(Self { span, scope })
    }

    /// The aggregation span (ticks or seconds; `> 0`).
    pub fn span(&self) -> u64 {
        self.span
    }

    /// The optional scope descriptor.
    pub fn scope(&self) -> Option<&str> {
        self.scope.as_deref()
    }
}

// NOTE: `StatusResult` — the derived evaluator *output* contract — was relocated
// to `hpdl-runtime-api` in feature 12 (spec 12, OD-12.4). It is a runtime output,
// not a declaration, and now lives with the other runtime contracts. The status
// *declaration* types above (`StatusDimension`, `EvaluatorRef`, `BaselineRef`,
// `CutPoints`, `EvaluationWindow`) remain here in `hpdl-model`.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn dimension_custom() {
        assert!(StatusDimension::custom("arcane.stability").is_ok());
        assert!(StatusDimension::custom("").is_err());
        assert!(StatusDimension::custom("has space").is_err());
    }

    #[test]
    fn evaluator_ref_wraps_module() {
        let ev = EvaluatorRef::new(ModuleRef::new("hpdl.vitality.physical", "1.0").unwrap());
        assert_eq!(ev.module().name(), "hpdl.vitality.physical");
        assert_eq!(ev.module().version(), "1.0");
    }

    #[test]
    fn baseline_ref_validates_name() {
        assert!(BaselineRef::new("preindustrial", Digest::new("a".repeat(64)).unwrap()).is_ok());
        assert!(matches!(
            BaselineRef::new("bad name!", Digest::new("a".repeat(64)).unwrap()),
            Err(Error::InvalidBaselineRef(_))
        ));
    }

    fn a1_cuts() -> CutPoints {
        CutPoints::new(
            0,
            100,
            [
                (0, "critical".to_string()),
                (25, "degraded".to_string()),
                (50, "stressed".to_string()),
                (75, "stable".to_string()),
            ],
        )
        .unwrap()
    }

    #[test]
    fn classify_boundaries() {
        let cp = a1_cuts();
        assert_eq!(cp.classify(0).unwrap(), "critical");
        assert_eq!(cp.classify(24).unwrap(), "critical"); // [0,25)
        assert_eq!(cp.classify(25).unwrap(), "degraded"); // [25,50)
        assert_eq!(cp.classify(49).unwrap(), "degraded");
        assert_eq!(cp.classify(50).unwrap(), "stressed");
        assert_eq!(cp.classify(74).unwrap(), "stressed");
        assert_eq!(cp.classify(75).unwrap(), "stable");
        assert_eq!(cp.classify(100).unwrap(), "stable"); // last band includes max
    }

    #[test]
    fn classify_out_of_range_rejected() {
        let cp = a1_cuts();
        assert!(matches!(
            cp.classify(-1),
            Err(Error::ScoreOutOfRange { .. })
        ));
        assert!(matches!(
            cp.classify(101),
            Err(Error::ScoreOutOfRange { .. })
        ));
    }

    #[test]
    fn cut_points_rejections() {
        // first cut != scale_min
        assert!(matches!(
            CutPoints::new(0, 100, [(10, "x".to_string())]),
            Err(Error::InvalidCutPoints { .. })
        ));
        // empty
        assert!(matches!(
            CutPoints::new(0, 100, std::iter::empty()),
            Err(Error::InvalidCutPoints { .. })
        ));
        // out of range
        assert!(matches!(
            CutPoints::new(0, 100, [(0, "a".to_string()), (150, "b".to_string())]),
            Err(Error::InvalidCutPoints { .. })
        ));
        // not strictly increasing (duplicate)
        assert!(matches!(
            CutPoints::new(
                0,
                100,
                [
                    (0, "a".to_string()),
                    (50, "b".to_string()),
                    (50, "c".to_string())
                ]
            ),
            Err(Error::InvalidCutPoints { .. })
        ));
        // bad scale
        assert!(matches!(
            CutPoints::new(100, 0, [(100, "a".to_string())]),
            Err(Error::InvalidCutPoints { .. })
        ));
    }

    /// Sub-integer boundaries (24.5, 49.999) require a scaled integer axis. On a
    /// deci-scaled 0..=1000 axis, 24.5 → 245 and 49.999 → 499 (truncating sub-deci).
    #[test]
    fn classify_scaled_sub_integer_boundaries() {
        let cp = CutPoints::new(
            0,
            1000,
            [
                (0, "critical".to_string()),
                (250, "degraded".to_string()),
                (500, "stressed".to_string()),
            ],
        )
        .unwrap();
        assert_eq!(cp.classify(245).unwrap(), "critical"); // 24.5 → [0,250)
        assert_eq!(cp.classify(250).unwrap(), "degraded"); // 25.0 → [250,500)
        assert_eq!(cp.classify(499).unwrap(), "degraded"); // 49.9 → [250,500)
    }

    #[test]
    fn window_rejects_zero() {
        assert!(EvaluationWindow::new(1, None).is_ok());
        assert!(matches!(
            EvaluationWindow::new(0, None),
            Err(Error::ZeroEvaluationWindow)
        ));
        assert!(EvaluationWindow::new(10, Some("surface".to_string())).is_ok());
        assert!(EvaluationWindow::new(10, Some("bad scope!".to_string())).is_err());
    }
}
