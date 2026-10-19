use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::{BaselineRef, CutPoints, Error, EvaluationWindow, EvaluatorRef, StatusDimension};

/// A declaration that a named status field exists on a body, with its scale bounds.
///
/// Status values are **derived** at runtime by versioned evaluators — they are
/// never directly written by actions. This struct only declares that the status
/// exists and what its numeric scale is.
///
/// # Example
///
/// ```
/// use hpdl_model::StatusDeclaration;
///
/// let s = StatusDeclaration::new("AtmosphericStability", 0, 100).unwrap();
/// assert_eq!(s.name(), "AtmosphericStability");
/// assert_eq!(s.scale_min(), 0);
/// assert_eq!(s.scale_max(), 100);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct StatusDeclaration {
    name: String,
    scale_min: i64,
    scale_max: i64,
    /// The independent dimension this status measures. `None` = undeclared.
    #[serde(default)]
    dimension: Option<StatusDimension>,
    /// The planet-specific baseline the evaluator measures against.
    #[serde(default)]
    baseline: Option<BaselineRef>,
    /// The versioned evaluator that derives this status.
    #[serde(default)]
    evaluator: Option<EvaluatorRef>,
    /// The ordered cut points partitioning the scale into bands (Amendment A1).
    #[serde(default)]
    cut_points: Option<CutPoints>,
    /// The temporal window the evaluator aggregates over.
    #[serde(default)]
    window: Option<EvaluationWindow>,
}

impl StatusDeclaration {
    /// Construct a `StatusDeclaration`.
    ///
    /// Returns [`Error::InvalidName`] if the name is empty or too long.
    /// Returns [`Error::InvalidScale`] if `scale_min >= scale_max`.
    pub fn new(name: impl Into<String>, scale_min: i64, scale_max: i64) -> Result<Self, Error> {
        let name = name.into();
        validate_name(&name)?;
        if scale_min >= scale_max {
            return Err(Error::InvalidScale {
                scale_min,
                scale_max,
            });
        }
        Ok(Self {
            name,
            scale_min,
            scale_max,
            dimension: None,
            baseline: None,
            evaluator: None,
            cut_points: None,
            window: None,
        })
    }

    /// Attach a [`StatusDimension`] (builder style).
    #[must_use]
    pub fn with_dimension(mut self, dimension: StatusDimension) -> Self {
        self.dimension = Some(dimension);
        self
    }

    /// Attach a [`BaselineRef`] (builder style).
    #[must_use]
    pub fn with_baseline(mut self, baseline: BaselineRef) -> Self {
        self.baseline = Some(baseline);
        self
    }

    /// Attach an [`EvaluatorRef`] (builder style).
    #[must_use]
    pub fn with_evaluator(mut self, evaluator: EvaluatorRef) -> Self {
        self.evaluator = Some(evaluator);
        self
    }

    /// Attach [`CutPoints`] (builder style).
    #[must_use]
    pub fn with_cut_points(mut self, cut_points: CutPoints) -> Self {
        self.cut_points = Some(cut_points);
        self
    }

    /// Attach an [`EvaluationWindow`] (builder style).
    #[must_use]
    pub fn with_window(mut self, window: EvaluationWindow) -> Self {
        self.window = Some(window);
        self
    }

    /// Return the status name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the lower bound of the status scale (inclusive).
    pub fn scale_min(&self) -> i64 {
        self.scale_min
    }

    /// Return the upper bound of the status scale (inclusive).
    pub fn scale_max(&self) -> i64 {
        self.scale_max
    }

    /// Return the declared dimension, if any. `None` = undeclared.
    pub fn dimension(&self) -> Option<&StatusDimension> {
        self.dimension.as_ref()
    }

    /// Return the declared baseline reference, if any.
    pub fn baseline(&self) -> Option<&BaselineRef> {
        self.baseline.as_ref()
    }

    /// Return the declared evaluator reference, if any.
    pub fn evaluator(&self) -> Option<&EvaluatorRef> {
        self.evaluator.as_ref()
    }

    /// Return the declared cut points, if any.
    pub fn cut_points(&self) -> Option<&CutPoints> {
        self.cut_points.as_ref()
    }

    /// Return the declared evaluation window, if any.
    pub fn window(&self) -> Option<&EvaluationWindow> {
        self.window.as_ref()
    }
}

pub(crate) fn validate_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::InvalidName(name.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn valid_declaration() {
        let s = StatusDeclaration::new("Vitality", 0, 100).unwrap();
        assert_eq!(s.name(), "Vitality");
        assert_eq!(s.scale_min(), 0);
        assert_eq!(s.scale_max(), 100);
    }

    #[test]
    fn equal_min_max_invalid() {
        assert!(matches!(
            StatusDeclaration::new("S", 50, 50),
            Err(Error::InvalidScale { .. })
        ));
    }

    #[test]
    fn inverted_scale_invalid() {
        assert!(matches!(
            StatusDeclaration::new("S", 100, 0),
            Err(Error::InvalidScale { .. })
        ));
    }

    #[test]
    fn negative_scale_valid() {
        // Negative scales are allowed (e.g. -100..=100)
        assert!(StatusDeclaration::new("Temperature", -100, 100).is_ok());
    }

    #[test]
    fn empty_name_invalid() {
        assert!(matches!(
            StatusDeclaration::new("", 0, 100),
            Err(Error::InvalidName(_))
        ));
    }
}
