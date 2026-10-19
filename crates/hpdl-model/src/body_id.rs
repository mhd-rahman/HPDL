use std::fmt;

use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};

use crate::Error;

/// Maximum allowed length for a [`BodyId`] string.
const MAX_LEN: usize = 128;

/// A strongly-typed, validated identifier for a celestial body.
///
/// # Validity
///
/// A `BodyId` string must:
/// - Be between 1 and 128 characters (inclusive).
/// - Contain only ASCII alphanumeric characters, hyphens (`-`), or underscores (`_`).
///
/// # Example
///
/// ```
/// use hpdl_model::BodyId;
///
/// let id = BodyId::new("proxima-b").unwrap();
/// assert_eq!(id.as_str(), "proxima-b");
///
/// assert!(BodyId::new("").is_err());
/// assert!(BodyId::new("bad id!").is_err());
/// ```
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, SchemaWrite, SchemaRead)]
pub struct BodyId(String);

impl BodyId {
    /// Create a new `BodyId`, validating the string.
    ///
    /// Returns [`Error::InvalidId`] if the string is empty, longer than 128
    /// characters, or contains characters outside `[A-Za-z0-9_-]`.
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        if s.is_empty() || s.len() > MAX_LEN || !s.chars().all(is_valid_char) {
            return Err(Error::InvalidId(s));
        }
        Ok(Self(s))
    }

    /// Return the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_valid_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

impl fmt::Display for BodyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for BodyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BodyId({:?})", self.0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn valid_ids() {
        assert!(BodyId::new("earth").is_ok());
        assert!(BodyId::new("proxima-b").is_ok());
        assert!(BodyId::new("planet_42").is_ok());
        // exactly 128 chars
        let long = "a".repeat(128);
        assert!(BodyId::new(long).is_ok());
    }

    #[test]
    fn empty_is_invalid() {
        assert_eq!(BodyId::new(""), Err(Error::InvalidId(String::new())));
    }

    #[test]
    fn too_long_is_invalid() {
        let s = "a".repeat(129);
        assert!(matches!(BodyId::new(&s), Err(Error::InvalidId(_))));
    }

    #[test]
    fn invalid_chars() {
        assert!(matches!(BodyId::new("bad id"), Err(Error::InvalidId(_))));
        assert!(matches!(BodyId::new("bad!"), Err(Error::InvalidId(_))));
        assert!(matches!(BodyId::new("has.dot"), Err(Error::InvalidId(_))));
    }

    #[test]
    fn equality_and_ordering() {
        let a = BodyId::new("alpha").unwrap();
        let b = BodyId::new("alpha").unwrap();
        let c = BodyId::new("beta").unwrap();
        assert_eq!(a, b);
        assert!(a < c);
    }

    #[test]
    fn display() {
        let id = BodyId::new("proxima-b").unwrap();
        assert_eq!(id.to_string(), "proxima-b");
    }
}
