//! Action capability tokens and read/write field sets (HPDL feature 10).
//!
//! A [`Capability`] is a namespaced authority token an action declares it needs
//! to mutate state (least privilege). A [`ReadWriteSet`] bundles an action's
//! declared read and write field paths together with its capability
//! requirements, and reports any mutating write **not** backed by a declared
//! capability.
//!
//! These are *declaration* concepts — they describe what an action promises it
//! needs — and live in `hpdl-model` because both the authored declaration
//! (validated by `hpdl-sema`) and the runtime event record (`hpdl-runtime-api`)
//! share the same vocabulary. The runtime crate re-exports these types.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::Error;

/// A namespaced capability token.
///
/// Names runtime authority an action needs — e.g. `state.atmosphere.write` or
/// `portal.cross-domain`. Valid tokens are 1–64 characters from `[A-Za-z0-9_\-.]`.
///
/// Capabilities are **declared requirements**: `hpdl-sema` validates that an
/// action's mutating writes are backed by declared capabilities. Whether the
/// acting module actually *holds* a capability at execution time is enforced by
/// the external engine (least-privilege authorization), not here.
///
/// # Example
///
/// ```
/// use hpdl_model::Capability;
///
/// let c = Capability::new("portal.cross-domain").unwrap();
/// assert_eq!(c.as_str(), "portal.cross-domain");
/// assert!(Capability::new("bad cap!").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Capability(String);

impl Capability {
    /// Construct a validated `Capability`.
    ///
    /// Returns [`Error::InvalidCapability`] on an empty, too-long, or
    /// invalid-character token.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if s.is_empty()
            || s.len() > 64
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(Error::InvalidCapability(s));
        }
        Ok(Self(s))
    }

    /// The underlying token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validate a dotted field path (e.g. `state.atmosphere.co2`).
///
/// Each dot-separated segment must be non-empty and drawn from `[A-Za-z0-9_\-]`.
pub(crate) fn validate_field_path(path: &str) -> Result<(), Error> {
    if path.is_empty() {
        return Err(Error::InvalidFieldPath(path.to_owned()));
    }
    for segment in path.split('.') {
        if segment.is_empty()
            || !segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err(Error::InvalidFieldPath(path.to_owned()));
        }
    }
    Ok(())
}

/// Deterministic, ordered read/write field paths plus the declared capability
/// requirements that back any mutating writes.
///
/// Reads, writes, and capabilities are each a `BTreeSet`, so iteration order is
/// total and stable (never hashmap order). [`ReadWriteSet::uncovered_writes`]
/// reports any write path not backed by a declared capability — the
/// declaration-level analogue of an "unauthorized" write.
///
/// # Example
///
/// ```
/// use hpdl_model::{Capability, ReadWriteSet};
///
/// let rws = ReadWriteSet::new(
///     ["state.temperature.mean"],
///     ["state.atmosphere.co2"],
///     [Capability::new("state.atmosphere.write").unwrap()],
/// ).unwrap();
/// assert!(rws.uncovered_writes().is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadWriteSet {
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
    capabilities: BTreeSet<Capability>,
}

impl ReadWriteSet {
    /// Construct a validated `ReadWriteSet`.
    ///
    /// Returns [`Error::InvalidFieldPath`] if any read or write path is malformed.
    pub fn new(
        reads: impl IntoIterator<Item = impl Into<String>>,
        writes: impl IntoIterator<Item = impl Into<String>>,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Result<Self, Error> {
        let reads: BTreeSet<String> = reads.into_iter().map(Into::into).collect();
        let writes: BTreeSet<String> = writes.into_iter().map(Into::into).collect();
        for p in reads.iter().chain(writes.iter()) {
            validate_field_path(p)?;
        }
        let capabilities: BTreeSet<Capability> = capabilities.into_iter().collect();
        Ok(Self {
            reads,
            writes,
            capabilities,
        })
    }

    /// The read field paths, in deterministic order.
    pub fn reads(&self) -> impl Iterator<Item = &str> {
        self.reads.iter().map(String::as_str)
    }

    /// The write field paths, in deterministic order.
    pub fn writes(&self) -> impl Iterator<Item = &str> {
        self.writes.iter().map(String::as_str)
    }

    /// The declared capability tokens, in deterministic order.
    pub fn capabilities(&self) -> impl Iterator<Item = &Capability> {
        self.capabilities.iter()
    }

    /// Write paths that are **not** covered by any declared capability.
    ///
    /// A write to `state.<domain>.<field>` is covered when a declared capability
    /// shares the `state.<domain>` namespace prefix (the capability's leading
    /// segments up to its trailing verb). An empty result means every write is
    /// authorized at the declaration level.
    pub fn uncovered_writes(&self) -> Vec<&str> {
        let mut uncovered = Vec::new();
        for w in &self.writes {
            if !self.is_write_covered(w) {
                uncovered.push(w.as_str());
            }
        }
        uncovered
    }

    /// Whether a single write path is covered by a declared capability.
    fn is_write_covered(&self, write: &str) -> bool {
        let write_ns = match write.rsplit_once('.') {
            Some((ns, _)) => ns,
            None => write,
        };
        self.capabilities.iter().any(|cap| {
            let cap_ns = match cap.as_str().rsplit_once('.') {
                Some((ns, _)) => ns,
                None => cap.as_str(),
            };
            write_ns == cap_ns || write_ns.starts_with(&format!("{cap_ns}."))
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn capability_validation() {
        assert!(Capability::new("state.atmosphere.write").is_ok());
        assert!(Capability::new("portal.cross-domain").is_ok());
        assert!(Capability::new("").is_err());
        assert!(Capability::new("has space").is_err());
        assert!(Capability::new("a".repeat(65)).is_err());
    }

    #[test]
    fn capability_ordering_is_deterministic() {
        assert!(Capability::new("a").unwrap() < Capability::new("b").unwrap());
    }

    #[test]
    fn valid_paths_and_order() {
        let rws = ReadWriteSet::new(
            ["state.b", "state.a"],
            ["state.atmosphere.co2"],
            [Capability::new("state.atmosphere.write").unwrap()],
        )
        .unwrap();
        let reads: Vec<&str> = rws.reads().collect();
        assert_eq!(reads, vec!["state.a", "state.b"]);
    }

    #[test]
    fn malformed_path_rejected() {
        assert!(matches!(
            ReadWriteSet::new([""], [] as [&str; 0], []),
            Err(Error::InvalidFieldPath(_))
        ));
        assert!(matches!(
            ReadWriteSet::new(["bad path"], [] as [&str; 0], []),
            Err(Error::InvalidFieldPath(_))
        ));
        assert!(matches!(
            ReadWriteSet::new(["state..co2"], [] as [&str; 0], []),
            Err(Error::InvalidFieldPath(_))
        ));
    }

    #[test]
    fn write_covered_by_capability() {
        let rws = ReadWriteSet::new(
            [] as [&str; 0],
            ["state.atmosphere.co2"],
            [Capability::new("state.atmosphere.write").unwrap()],
        )
        .unwrap();
        assert!(rws.uncovered_writes().is_empty());
    }

    #[test]
    fn write_not_covered_is_reported() {
        let rws = ReadWriteSet::new(
            [] as [&str; 0],
            ["state.hydrosphere.level"],
            [Capability::new("state.atmosphere.write").unwrap()],
        )
        .unwrap();
        assert_eq!(rws.uncovered_writes(), vec!["state.hydrosphere.level"]);
    }

    #[test]
    fn write_with_no_capabilities_is_uncovered() {
        let rws = ReadWriteSet::new([] as [&str; 0], ["state.atmosphere.co2"], []).unwrap();
        assert_eq!(rws.uncovered_writes(), vec!["state.atmosphere.co2"]);
    }

    #[test]
    fn broader_capability_covers_nested_write() {
        let rws = ReadWriteSet::new(
            [] as [&str; 0],
            ["state.atmosphere.co2"],
            [Capability::new("state.write").unwrap()],
        )
        .unwrap();
        assert!(rws.uncovered_writes().is_empty());
    }
}
