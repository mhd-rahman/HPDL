//! Token definitions for HPDL lexer.

use crate::SourceSpan;

/// A token with its kind, span, and original text.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind of token.
    pub kind: TokenKind,

    /// The source span of this token.
    pub span: SourceSpan,

    /// The original text of this token.
    pub text: String,
}

impl Token {
    /// Create a new token.
    pub fn new(kind: TokenKind, span: SourceSpan, text: String) -> Self {
        Self { kind, span, text }
    }
}

/// The kind of token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    /// `language` keyword
    Language,
    /// `hpdl` keyword
    Hpdl,
    /// `planet` keyword
    Planet,
    /// `body` keyword
    Body,
    /// `physical` keyword
    Physical,
    /// `traits` keyword
    Traits,
    /// `status` keyword
    Status,
    /// `action` keyword
    Action,
    /// `module` keyword
    Module,
    /// `migration` keyword
    Migration,
    /// `true` keyword
    True,
    /// `false` keyword
    False,

    // Literals
    /// String literal (e.g., `"hello"`)
    String,
    /// Integer literal (e.g., `42`, `1_000_000`)
    Integer,
    /// Float literal (e.g., `3.14`, `1.0e10`)
    Float,

    // Identifiers
    /// Identifier (e.g., `my_planet`, `kepler-186f`)
    Identifier,

    // Operators and delimiters
    /// `:` (colon)
    Colon,
    /// `,` (comma)
    Comma,
    /// `;` (semicolon)
    Semicolon,
    /// `{` (left brace)
    LeftBrace,
    /// `}` (right brace)
    RightBrace,
    /// `[` (left bracket)
    LeftBracket,
    /// `]` (right bracket)
    RightBracket,
    /// `(` (left paren)
    LeftParen,
    /// `)` (right paren)
    RightParen,

    // Special
    /// End of file
    Eof,

    /// Invalid/error token
    Error,
}

impl TokenKind {
    /// Check if this token kind is a keyword.
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            Self::Language
                | Self::Hpdl
                | Self::Planet
                | Self::Body
                | Self::Physical
                | Self::Traits
                | Self::Status
                | Self::Action
                | Self::Module
                | Self::Migration
                | Self::True
                | Self::False
        )
    }

    /// Check if this token kind is a literal.
    pub fn is_literal(self) -> bool {
        matches!(
            self,
            Self::String | Self::Integer | Self::Float | Self::True | Self::False
        )
    }

    /// Get the keyword from a string, if it is one.
    pub fn from_keyword(text: &str) -> Option<Self> {
        match text {
            "language" => Some(Self::Language),
            "hpdl" => Some(Self::Hpdl),
            "planet" => Some(Self::Planet),
            "body" => Some(Self::Body),
            "physical" => Some(Self::Physical),
            "traits" => Some(Self::Traits),
            "status" => Some(Self::Status),
            "action" => Some(Self::Action),
            "module" => Some(Self::Module),
            "migration" => Some(Self::Migration),
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_token_kind_from_keyword() {
        assert_eq!(
            TokenKind::from_keyword("language"),
            Some(TokenKind::Language)
        );
        assert_eq!(TokenKind::from_keyword("planet"), Some(TokenKind::Planet));
        assert_eq!(TokenKind::from_keyword("true"), Some(TokenKind::True));
        assert_eq!(TokenKind::from_keyword("false"), Some(TokenKind::False));
        assert_eq!(TokenKind::from_keyword("not_a_keyword"), None);
    }

    #[test]
    fn test_is_keyword() {
        assert!(TokenKind::Language.is_keyword());
        assert!(TokenKind::Planet.is_keyword());
        assert!(!TokenKind::Identifier.is_keyword());
        assert!(!TokenKind::String.is_keyword());
    }

    #[test]
    fn test_is_literal() {
        assert!(TokenKind::String.is_literal());
        assert!(TokenKind::Integer.is_literal());
        assert!(TokenKind::Float.is_literal());
        assert!(TokenKind::True.is_literal());
        assert!(TokenKind::False.is_literal());
        assert!(!TokenKind::Identifier.is_literal());
    }
}
