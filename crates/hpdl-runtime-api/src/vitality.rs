//! Vitality receipt: [`StatusResult`], the derived output of an evaluator.
//!
//! Relocated from `hpdl-model` in feature 12 (spec OD-12.4). Feature 09 placed
//! `StatusResult` provisionally in `hpdl-model` only because `hpdl-runtime-api`
//! was deferred (OD-09.1); it is a **runtime output contract** and now lives with
//! the other runtime contracts. The status *declaration* types
//! ([`hpdl_model::StatusDimension`], `EvaluatorRef`, `BaselineRef`, `CutPoints`,
//! `EvaluationWindow`) stay in `hpdl-model`.
//!
//! A `StatusResult` is **produced by an evaluator, never authored** by a
//! definition. No constructor here lets a definition assign a status score to a
//! planet; this type only records what an evaluator derived, for replay and audit.

use hpdl_model::{Digest, Error, StatusDimension};
use serde::{Deserialize, Serialize};

/// The derived output of a vitality evaluator — the status "result" contract.
///
/// **Replay invariant:** the same evaluator version, the same input hashes
/// (stored sorted), and the same window yield an identical `StatusResult`.
///
/// # Example
///
/// ```
/// use hpdl_model::{Digest, StatusDimension};
/// use hpdl_runtime_api::StatusResult;
///
/// let r = StatusResult::new(
///     StatusDimension::Physical,
///     42,
///     "degraded",
///     "1.0",
///     vec![Digest::new("a".repeat(64)).unwrap()],
///     "co2 above baseline",
/// ).unwrap();
/// assert_eq!(r.score(), 42);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusResult {
    dimension: StatusDimension,
    score: i64,
    band: String,
    evaluator_version: String,
    input_hashes: Vec<Digest>,
    evidence: String,
}

impl StatusResult {
    /// Construct a `StatusResult`. Input hashes are stored in sorted order so the
    /// result is deterministic regardless of input ordering.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidStatusDimensionName`] if the band label is empty,
    /// too long, or contains characters outside `[A-Za-z0-9_\-.]`.
    pub fn new(
        dimension: StatusDimension,
        score: i64,
        band: impl Into<String>,
        evaluator_version: impl Into<String>,
        mut input_hashes: Vec<Digest>,
        evidence: impl Into<String>,
    ) -> Result<Self, Error> {
        let band = band.into();
        validate_band_label(&band)?;
        input_hashes.sort();
        Ok(Self {
            dimension,
            score,
            band,
            evaluator_version: evaluator_version.into(),
            input_hashes,
            evidence: evidence.into(),
        })
    }

    /// The dimension this result belongs to.
    pub fn dimension(&self) -> &StatusDimension {
        &self.dimension
    }

    /// The derived score.
    pub fn score(&self) -> i64 {
        self.score
    }

    /// The classified band label.
    pub fn band(&self) -> &str {
        &self.band
    }

    /// The evaluator version that produced this result.
    pub fn evaluator_version(&self) -> &str {
        &self.evaluator_version
    }

    /// The sorted input hashes consumed (for replay/audit).
    pub fn input_hashes(&self) -> &[Digest] {
        &self.input_hashes
    }

    /// The evidence summary.
    pub fn evidence(&self) -> &str {
        &self.evidence
    }
}

/// Validate a band label (1–64 chars, `[A-Za-z0-9_\-.]`).
///
/// Mirrors the model's status-name rule; a bad label yields
/// [`Error::InvalidStatusDimensionName`]. (The model owns the dimension/baseline
/// vocabulary; a `StatusResult` only needs to validate its own band label.)
fn validate_band_label(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(Error::InvalidStatusDimensionName(name.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn status_result_sorts_hashes_and_validates_band() {
        let r = StatusResult::new(
            StatusDimension::Physical,
            42,
            "degraded",
            "1.0",
            vec![
                Digest::new("b".repeat(64)).unwrap(),
                Digest::new("a".repeat(64)).unwrap(),
            ],
            "evidence",
        )
        .unwrap();
        // Hashes are sorted deterministically.
        assert_eq!(r.input_hashes()[0].as_str(), &"a".repeat(64));
        assert_eq!(r.input_hashes()[1].as_str(), &"b".repeat(64));
        assert_eq!(r.band(), "degraded");

        assert!(StatusResult::new(
            StatusDimension::Physical,
            1,
            "bad band!",
            "1.0",
            vec![],
            "e"
        )
        .is_err());
    }

    /// Replay invariant: identical versioned inputs yield identical results.
    #[test]
    fn status_result_replay_is_identical() {
        let make = || {
            StatusResult::new(
                StatusDimension::NativeEcosystem,
                60,
                "stressed",
                "2.1",
                vec![Digest::new("c".repeat(64)).unwrap()],
                "ecosystem index",
            )
            .unwrap()
        };
        assert_eq!(make(), make());
    }
}
