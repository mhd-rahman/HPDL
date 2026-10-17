//! Source span tracking for diagnostics.

use crate::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// A span within a source file (byte offsets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// Start byte offset (inclusive).
    pub start: usize,

    /// End byte offset (exclusive).
    pub end: usize,
}

impl SourceSpan {
    /// Create a new source span.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get the length of this span.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if this span is empty.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Create a span that covers both spans.
    pub fn union(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// A source file with content and path.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Path to the source file.
    path: PathBuf,

    /// The source text content.
    content: String,

    /// Line start byte offsets for quick line/column calculation.
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// Load a source file from a path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or is not valid UTF-8.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        Ok(Self::new(path.to_path_buf(), content))
    }

    /// Create a source file from in-memory content.
    pub fn from_string(path: impl Into<PathBuf>, content: String) -> Self {
        Self::new(path.into(), content)
    }

    /// Create a new source file.
    fn new(path: PathBuf, content: String) -> Self {
        let line_starts = Self::compute_line_starts(&content);
        Self {
            path,
            content,
            line_starts,
        }
    }

    /// Get the path to this source file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the source content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Convert a byte offset to (line, column) position (both 1-indexed).
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        let line_start = self.line_starts.get(line_idx).copied().unwrap_or(0);
        let column = offset.saturating_sub(line_start);

        (line_idx + 1, column + 1)
    }

    /// Get the content of a specific line (1-indexed).
    pub fn get_line(&self, line_num: usize) -> Option<&str> {
        if line_num == 0 || line_num > self.line_starts.len() {
            return None;
        }

        let start = *self.line_starts.get(line_num - 1)?;
        let end = self
            .line_starts
            .get(line_num)
            .copied()
            .unwrap_or(self.content.len());

        let line = self.content.get(start..end)?;
        Some(line.trim_end_matches(&['\n', '\r'][..]))
    }

    /// Get a source excerpt for the given span.
    pub fn get_excerpt(&self, span: SourceSpan) -> Option<String> {
        if span.start > self.content.len() || span.end > self.content.len() {
            return None;
        }

        let (start_line, start_col) = self.line_col(span.start);
        let (end_line, _) = self.line_col(span.end);

        let mut output = String::new();

        for line_num in start_line..=end_line {
            if let Some(line_content) = self.get_line(line_num) {
                output.push_str(&format!("  {line_num} | {line_content}\n"));

                if line_num == start_line && start_line == end_line {
                    output.push_str(&format!("  {} | ", " ".repeat(line_num.to_string().len())));
                    output.push_str(&" ".repeat(start_col.saturating_sub(1)));

                    let span_len = span.len().max(1);
                    output.push_str(&"^".repeat(span_len.min(line_content.len())));
                    output.push('\n');
                }
            }
        }

        Some(output)
    }

    /// Compute line start offsets.
    fn compute_line_starts(content: &str) -> Vec<usize> {
        let mut starts = vec![0];

        for (idx, ch) in content.char_indices() {
            if ch == '\n' {
                starts.push(idx + 1);
            }
        }

        starts
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_source_span() {
        let span = SourceSpan::new(5, 10);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());

        let empty = SourceSpan::new(5, 5);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_span_union() {
        let span1 = SourceSpan::new(5, 10);
        let span2 = SourceSpan::new(8, 15);
        let union = span1.union(span2);

        assert_eq!(union.start, 5);
        assert_eq!(union.end, 15);
    }

    #[test]
    fn test_source_file_from_string() {
        let source = SourceFile::from_string("test.hpdl", "line 1\nline 2\nline 3".to_string());

        assert_eq!(source.path(), Path::new("test.hpdl"));
        assert_eq!(source.get_line(1), Some("line 1"));
        assert_eq!(source.get_line(2), Some("line 2"));
        assert_eq!(source.get_line(3), Some("line 3"));
    }

    #[test]
    fn test_line_col() {
        let source = SourceFile::from_string("test.hpdl", "hello\nworld\ntest".to_string());

        assert_eq!(source.line_col(0), (1, 1)); // 'h'
        assert_eq!(source.line_col(6), (2, 1)); // 'w'
        assert_eq!(source.line_col(12), (3, 1)); // 't'
    }
}
