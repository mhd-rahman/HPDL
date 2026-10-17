use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::Error;

/// Maximum allowed length for a single trait name.
const MAX_TRAIT_LEN: usize = 64;

/// An ordered, deduplicated set of trait names applied to a celestial body.
///
/// Trait names use a dot-separated namespace convention (e.g. `"geology.rocky"`).
/// Iteration order is always lexicographic, ensuring deterministic output.
///
/// # Example
///
/// ```
/// use hpdl_model::TraitSet;
///
/// let ts = TraitSet::new(["geology.rocky", "atmosphere.thin"]).unwrap();
/// let names: Vec<_> = ts.iter().cloned().collect();
/// // lexicographic order guaranteed
/// assert_eq!(names, ["atmosphere.thin", "geology.rocky"]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TraitSet(BTreeSet<String>);

impl TraitSet {
    /// Construct a `TraitSet` from an iterable of trait name strings.
    ///
    /// Returns [`Error::InvalidTraitName`] if any name is empty, exceeds
    /// 64 characters, or contains characters outside `[A-Za-z0-9_\-.]`.
    ///
    /// Duplicate names are silently deduplicated (BTreeSet semantics).
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Result<Self, Error> {
        let mut set = BTreeSet::new();
        for item in items {
            let name = item.into();
            validate_trait_name(&name)?;
            set.insert(name);
        }
        Ok(Self(set))
    }

    /// Returns an empty `TraitSet`.
    pub fn empty() -> Self {
        Self(BTreeSet::new())
    }

    /// Iterate over trait names in lexicographic order.
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }

    /// Return the number of traits in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return `true` if the set contains no traits.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return `true` if the set contains the given trait name.
    pub fn contains(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}

fn validate_trait_name(name: &str) -> Result<(), Error> {
    if name.is_empty() || name.len() > MAX_TRAIT_LEN || !name.chars().all(is_valid_trait_char) {
        return Err(Error::InvalidTraitName(name.to_owned()));
    }
    Ok(())
}

fn is_valid_trait_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn valid_traits() {
        let ts = TraitSet::new(["geology.rocky", "atmosphere.thin"]).unwrap();
        assert_eq!(ts.len(), 2);
        assert!(ts.contains("geology.rocky"));
    }

    #[test]
    fn deduplication() {
        let ts = TraitSet::new(["rocky", "rocky", "thin"]).unwrap();
        assert_eq!(ts.len(), 2);
    }

    #[test]
    fn lexicographic_order() {
        let ts = TraitSet::new(["z-trait", "a-trait", "m-trait"]).unwrap();
        let names: Vec<_> = ts.iter().collect();
        assert_eq!(names.first().map(|s| s.as_str()), Some("a-trait"));
        assert_eq!(names.get(2).map(|s| s.as_str()), Some("z-trait"));
    }

    #[test]
    fn empty_name_invalid() {
        assert!(matches!(
            TraitSet::new([""]),
            Err(Error::InvalidTraitName(_))
        ));
    }

    #[test]
    fn too_long_invalid() {
        let long = "a".repeat(65);
        assert!(matches!(
            TraitSet::new([long.as_str()]),
            Err(Error::InvalidTraitName(_))
        ));
    }

    #[test]
    fn invalid_char() {
        assert!(matches!(
            TraitSet::new(["bad name"]),
            Err(Error::InvalidTraitName(_))
        ));
    }

    #[test]
    fn empty_set() {
        let ts = TraitSet::empty();
        assert!(ts.is_empty());
    }
}
