//! Error types for the HPDL syntax parser.

use std::io;
use thiserror::Error;

/// Errors produced by the HPDL syntax parser.
#[derive(Debug, Error)]
pub enum Error {
    /// Full parser implementation is not yet complete.
    #[error("Parser unimplemented: {0}")]
    Unimplemented(String),

    /// Lexical analysis error.
    #[error("Lexer error: {0}")]
    Lexer(String),

    /// Syntax parsing error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Semantic validation failed; carries the rendered diagnostics.
    #[error("Semantic validation failed:\n{0}")]
    Validation(String),

    /// Semantic validation failed (from hpdl-sema).
    #[error("Semantic validation error: {0}")]
    Sema(#[from] hpdl_sema::Error),

    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Invalid UTF-8 encoding.
    #[error("Invalid UTF-8 encoding: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    /// Security limit exceeded.
    #[error("Security limit exceeded: {0}")]
    Security(String),
}

impl Error {
    /// Create a lexer error.
    pub fn lexer(msg: impl Into<String>) -> Self {
        Self::Lexer(msg.into())
    }

    /// Create a parse error.
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Create a security error.
    pub fn security(msg: impl Into<String>) -> Self {
        Self::Security(msg.into())
    }

    /// Create a semantic-validation error carrying rendered diagnostics.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
}
