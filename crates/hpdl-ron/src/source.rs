//! Source document representation with span tracking for diagnostics.

use crate::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// A source document containing RON text with path and span mapping.
#[derive(Debug, Clone)]
pub struct SourceDocument {
    /// Path or identifier for this document.
    path: PathBuf,

    /// The source text content.
    content: String,

    /// Line start offsets for quick line/column calculation.
    line_starts: Vec<usize>,
}

/// A span within a source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    /// Start byte offset (inclusive).
    pub start: usize,

    /// End byte offset (exclusive).
    pub end: usize,
}

impl SourceDocument {
    /// Load a source document from a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The file content is not valid UTF-8
    /// - The path contains traversal attempts (e.g., `..`)
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();

        // Validate path (prevent path traversal)
        Self::validate_path(path)?;

        // Read file content
        let content = fs::read_to_string(path)?;

        Ok(Self::new(path.to_path_buf(), content))
    }

    /// Create a source document from in-memory content.
    ///
    /// # Arguments
    ///
    /// * `path` - A path or identifier for diagnostic purposes
    /// * `content` - The source text content
    pub fn from_string(path: impl Into<PathBuf>, content: String) -> Self {
        Self::new(path.into(), content)
    }

    /// Create a new source document.
    fn new(path: PathBuf, content: String) -> Self {
        let line_starts = Self::compute_line_starts(&content);
        Self {
            path,
            content,
            line_starts,
        }
    }

    /// Get the path or identifier for this document.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the source content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Convert a byte offset to (line, column) position (both 1-indexed).
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset in the source
    ///
    /// # Returns
    ///
    /// A tuple of (line number, column number), both 1-indexed.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        // Find the line containing this offset
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        // Safety: line_starts always has at least one element (0)
        let line_start = self.line_starts.get(line_idx).copied().unwrap_or(0);
        let column = offset.saturating_sub(line_start);

        // Return 1-indexed line and column
        (line_idx + 1, column + 1)
    }

    /// Get a source excerpt for the given span, with context lines.
    ///
    /// # Arguments
    ///
    /// * `span` - The span to extract
    ///
    /// # Returns
    ///
    /// A formatted string showing the relevant source lines with the span highlighted.
    pub fn get_excerpt(&self, span: SourceSpan) -> Option<String> {
        if span.start > self.content.len() || span.end > self.content.len() {
            return None;
        }

        let (start_line, start_col) = self.line_col(span.start);
        let (end_line, _) = self.line_col(span.end);

        let mut output = String::new();

        // Show the lines containing the span
        for line_num in start_line..=end_line {
            if let Some(line_content) = self.get_line(line_num) {
                output.push_str(&format!("  {line_num} | {line_content}\n"));

                // Add highlight for the error span (only on single line for now)
                if line_num == start_line && start_line == end_line {
                    output.push_str(&format!("  {} | ", " ".repeat(line_num.to_string().len())));
                    output.push_str(&" ".repeat(start_col.saturating_sub(1)));

                    let span_len = span.end.saturating_sub(span.start).max(1);
                    output.push_str(&"^".repeat(span_len.min(line_content.len())));
                    output.push('\n');
                }
            }
        }

        Some(output)
    }

    /// Get the content of a specific line (1-indexed).
    fn get_line(&self, line_num: usize) -> Option<&str> {
        if line_num == 0 || line_num > self.line_starts.len() {
            return None;
        }

        let start = *self.line_starts.get(line_num - 1)?;
        let end = self
            .line_starts
            .get(line_num)
            .copied()
            .unwrap_or(self.content.len());

        // Trim trailing newline if present
        let line = self.content.get(start..end)?;
        Some(line.trim_end_matches(&['\n', '\r'][..]))
    }

    /// Validate that a path doesn't contain traversal attempts.
    fn validate_path(path: &Path) -> Result<(), Error> {
        // Check for path traversal
        for component in path.components() {
            if component == std::path::Component::ParentDir {
                return Err(Error::path_traversal(path.display().to_string()));
            }
        }
        Ok(())
    }

    /// Compute line start offsets for the content.
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_from_string() {
        let content = "line 1\nline 2\nline 3";
        let doc = SourceDocument::from_string("test.ron", content.to_string());

        assert_eq!(doc.path(), Path::new("test.ron"));
        assert_eq!(doc.content(), content);
    }

    #[test]
    fn test_line_col_calculation() {
        let content = "hello\nworld\ntest";
        let doc = SourceDocument::from_string("test.ron", content.to_string());

        // First character (h)
        assert_eq!(doc.line_col(0), (1, 1));

        // Last character of first line (o)
        assert_eq!(doc.line_col(4), (1, 5));

        // First character of second line (w)
        assert_eq!(doc.line_col(6), (2, 1));

        // First character of third line (t)
        assert_eq!(doc.line_col(12), (3, 1));
    }

    #[test]
    fn test_get_line() {
        let content = "first\nsecond\nthird";
        let doc = SourceDocument::from_string("test.ron", content.to_string());

        assert_eq!(doc.get_line(1), Some("first"));
        assert_eq!(doc.get_line(2), Some("second"));
        assert_eq!(doc.get_line(3), Some("third"));
        assert_eq!(doc.get_line(0), None);
        assert_eq!(doc.get_line(4), None);
    }

    #[test]
    fn test_source_span() {
        let span = SourceSpan::new(5, 10);
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 10);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());

        let empty = SourceSpan::new(5, 5);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_path_traversal_detection() {
        let result = SourceDocument::from_file("../../../etc/passwd");
        assert!(matches!(result, Err(Error::PathTraversal(_))));
    }

    #[test]
    fn test_excerpt_single_line() {
        let content = "let x = 42;";
        let doc = SourceDocument::from_string("test.ron", content.to_string());

        let span = SourceSpan::new(4, 5); // The 'x'
        let excerpt = doc.get_excerpt(span).unwrap();

        assert!(excerpt.contains("let x = 42;"));
        assert!(excerpt.contains("^"));
    }

    #[test]
    fn test_line_starts_computation() {
        let content = "a\nb\nc";
        let doc = SourceDocument::from_string("test.ron", content.to_string());

        // Line starts should be [0, 2, 4]
        assert_eq!(doc.line_starts, vec![0, 2, 4]);
    }

    #[test]
    fn test_empty_content() {
        let doc = SourceDocument::from_string("empty.ron", String::new());
        assert_eq!(doc.content(), "");
        assert_eq!(doc.line_col(0), (1, 1));
    }
}
