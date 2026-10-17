//! The authoritative, format-versioned `hpdl.lock` (HPDL feature 14).
//!
//! This supersedes the provisional feature-06 [`crate::BuildLock`] line format. The
//! authoritative lock carries an explicit `lock_format_version` header and the full
//! transitive closure — each package pins an exact version, a content [`Digest`], and
//! the names of its direct dependencies — so a build can be reconstructed and
//! verified deterministically and offline. It never records a mutable label.
//!
//! Parsing performs **no** network or filesystem access and fails closed with typed
//! [`crate::Error`]s on malformed input.

use std::collections::{BTreeMap, BTreeSet};

use hpdl_model::Digest;

use crate::options::ResourceLimits;
use crate::Error;

/// The only lock format version this compiler understands.
pub const LOCK_FORMAT_VERSION: u32 = 1;

/// A single locked package: an exact version, a content digest, and the names of
/// its direct dependencies (resolved via their own entries in the closure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockedPackage {
    /// Package name.
    pub name: String,
    /// Exact pinned version string.
    pub version: String,
    /// Validated 64-hex content digest of the pinned package.
    pub content_digest: Digest,
    /// Names of the package's direct dependencies, in total order.
    pub dependencies: BTreeSet<String>,
}

/// An authoritative lockfile pinning a build's full transitive closure.
///
/// Backed by a `BTreeMap` keyed by package name so iteration order is fixed and
/// never leaks `HashMap` nondeterminism into output.
///
/// # Example
///
/// ```
/// use hpdl_compiler::Lockfile;
///
/// let text = "\
/// lock_format_version = 1
/// package = hpdl.atmosphere 1.2.0 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
/// ";
/// let lock = Lockfile::parse(text).unwrap();
/// assert_eq!(lock.format_version(), 1);
/// assert_eq!(lock.get("hpdl.atmosphere").unwrap().version, "1.2.0");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lockfile {
    format_version: u32,
    entries: BTreeMap<String, LockedPackage>,
}

impl Lockfile {
    /// Parse an authoritative lockfile from its deterministic text form.
    ///
    /// The first non-blank, non-comment line MUST be `lock_format_version = N`.
    /// Each subsequent `package =` line is `name version digest [dep-name ...]`.
    /// Blank lines and `#`-prefixed comments are ignored; lines are 1-indexed in
    /// error reports.
    ///
    /// # Errors
    ///
    /// - [`Error::LockFormat`] on a missing/unknown format version.
    /// - [`Error::LockParse`] on a malformed line, a bad digest, or a duplicate
    ///   package name.
    /// - [`Error::Limit`] if the text exceeds the default source-size bound.
    pub fn parse(text: &str) -> Result<Self, Error> {
        Self::parse_with_limits(text, &ResourceLimits::default())
    }

    /// Parse with explicit resource limits (byte-size bound).
    pub fn parse_with_limits(text: &str, limits: &ResourceLimits) -> Result<Self, Error> {
        if text.len() > limits.max_source_bytes {
            return Err(Error::Limit(format!(
                "lockfile is {} bytes, exceeds max {}",
                text.len(),
                limits.max_source_bytes
            )));
        }

        let mut format_version: Option<u32> = None;
        let mut entries: BTreeMap<String, LockedPackage> = BTreeMap::new();

        for (idx, raw) in text.lines().enumerate() {
            let line_no = idx + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix("lock_format_version") {
                let value =
                    rest.trim_start()
                        .strip_prefix('=')
                        .ok_or_else(|| Error::LockFormat {
                            reason: format!("line {line_no}: expected `lock_format_version = N`"),
                        })?;
                let parsed: u32 = value.trim().parse().map_err(|_| Error::LockFormat {
                    reason: format!("line {line_no}: format version is not a non-negative integer"),
                })?;
                if parsed != LOCK_FORMAT_VERSION {
                    return Err(Error::LockFormat {
                        reason: format!(
                            "unsupported lock_format_version {parsed}; this compiler supports {LOCK_FORMAT_VERSION}"
                        ),
                    });
                }
                format_version = Some(parsed);
                continue;
            }

            let body = line
                .strip_prefix("package")
                .map(str::trim_start)
                .and_then(|r| r.strip_prefix('=').map(str::trim_start));
            let body = body.ok_or_else(|| Error::LockParse {
                line: line_no,
                reason: "expected a `package = ...` entry".to_string(),
            })?;

            if format_version.is_none() {
                return Err(Error::LockFormat {
                    reason: format!(
                        "line {line_no}: `lock_format_version` must appear before any package entry"
                    ),
                });
            }

            let entry = Self::parse_package_line(line_no, body)?;
            if entries.insert(entry.name.clone(), entry).is_some() {
                return Err(Error::LockParse {
                    line: line_no,
                    reason: "duplicate package name".to_string(),
                });
            }
        }

        let format_version = format_version.ok_or_else(|| Error::LockFormat {
            reason: "missing `lock_format_version` header".to_string(),
        })?;

        Ok(Self {
            format_version,
            entries,
        })
    }

    fn parse_package_line(line_no: usize, body: &str) -> Result<LockedPackage, Error> {
        let mut fields = body.split_whitespace();
        let name = fields.next().ok_or_else(|| Error::LockParse {
            line: line_no,
            reason: "missing package name".to_string(),
        })?;
        let version = fields.next().ok_or_else(|| Error::LockParse {
            line: line_no,
            reason: format!("package `{name}` is missing a version field"),
        })?;
        let digest_str = fields.next().ok_or_else(|| Error::LockParse {
            line: line_no,
            reason: format!("package `{name}` is missing a content-digest field"),
        })?;
        let content_digest = Digest::new(digest_str).map_err(|_| Error::LockParse {
            line: line_no,
            reason: format!("package `{name}` has a malformed content digest"),
        })?;

        let mut dependencies: BTreeSet<String> = BTreeSet::new();
        for dep in fields {
            dependencies.insert(dep.to_string());
        }

        Ok(LockedPackage {
            name: name.to_string(),
            version: version.to_string(),
            content_digest,
            dependencies,
        })
    }

    /// The lock format version.
    #[must_use]
    pub fn format_version(&self) -> u32 {
        self.format_version
    }

    /// Look up a locked package by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&LockedPackage> {
        self.entries.get(name)
    }

    /// Return `true` if the lock contains no package entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the number of locked packages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Serialize the lockfile to its canonical, deterministic text form.
    ///
    /// Emits the format-version header then one `package =` line per entry, sorted
    /// by name (dependencies also sorted). Round-trips through [`Lockfile::parse`].
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("lock_format_version = {}\n", self.format_version));
        for entry in self.entries.values() {
            out.push_str(&format!(
                "package = {} {} {}",
                entry.name,
                entry.version,
                entry.content_digest.as_str()
            ));
            for dep in &entry.dependencies {
                out.push(' ');
                out.push_str(dep);
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn digest(c: char) -> String {
        c.to_string().repeat(64)
    }

    #[test]
    fn parse_and_roundtrip() {
        let text = format!(
            "lock_format_version = 1\n# comment\npackage = hpdl.climate 2.0.0 {} hpdl.atmosphere\npackage = hpdl.atmosphere 1.2.0 {}\n",
            digest('a'),
            digest('b'),
        );
        let lock = Lockfile::parse(&text).unwrap();
        assert_eq!(lock.format_version(), 1);
        assert_eq!(lock.len(), 2);
        assert_eq!(lock.get("hpdl.climate").unwrap().version, "2.0.0");
        assert!(lock
            .get("hpdl.climate")
            .unwrap()
            .dependencies
            .contains("hpdl.atmosphere"));
        // Canonical re-serialization round-trips.
        let reparsed = Lockfile::parse(&lock.to_text()).unwrap();
        assert_eq!(lock, reparsed);
    }

    #[test]
    fn missing_header_rejected() {
        let text = format!("package = hpdl.a 1.0.0 {}\n", digest('a'));
        assert!(matches!(
            Lockfile::parse(&text),
            Err(Error::LockFormat { .. })
        ));
    }

    #[test]
    fn unknown_format_version_rejected() {
        let text = format!(
            "lock_format_version = 99\npackage = hpdl.a 1.0.0 {}\n",
            digest('a')
        );
        assert!(matches!(
            Lockfile::parse(&text),
            Err(Error::LockFormat { .. })
        ));
    }

    #[test]
    fn malformed_digest_rejected() {
        let text = "lock_format_version = 1\npackage = hpdl.a 1.0.0 tooshort\n";
        assert!(matches!(
            Lockfile::parse(text),
            Err(Error::LockParse { line: 2, .. })
        ));
    }

    #[test]
    fn duplicate_package_rejected() {
        let text = format!(
            "lock_format_version = 1\npackage = hpdl.a 1.0.0 {}\npackage = hpdl.a 1.0.0 {}\n",
            digest('a'),
            digest('b'),
        );
        assert!(matches!(
            Lockfile::parse(&text),
            Err(Error::LockParse { line: 3, .. })
        ));
    }

    #[test]
    fn empty_header_only_is_valid_and_empty() {
        let lock = Lockfile::parse("lock_format_version = 1\n").unwrap();
        assert!(lock.is_empty());
    }

    #[test]
    fn oversized_lock_rejected() {
        let limits = ResourceLimits {
            max_source_bytes: 8,
            max_modules: 1024,
        };
        assert!(matches!(
            Lockfile::parse_with_limits("lock_format_version = 1\n", &limits),
            Err(Error::Limit(_))
        ));
    }
}
