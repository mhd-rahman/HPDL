//! Parser for HPDL syntax.

use crate::diagnostic::{ParseError, ParseErrors};
use crate::{ast::*, Error, Lexer, SourceSpan, Token, TokenKind};

/// Parser that converts tokens into an AST.
#[derive(Debug)]
pub struct Parser {
    /// The token stream.
    tokens: Vec<Token>,

    /// Current position in the token stream.
    pos: usize,

    /// Errors collected during a recovering parse. Empty for the strict
    /// single-error path.
    errors: Vec<ParseError>,
}

impl Parser {
    /// Create a new parser from a token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    /// Parse tokens from source text.
    pub fn from_source(source: String) -> Result<Self, Error> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.lex_all()?;
        Ok(Self::new(tokens))
    }

    /// Parse the complete file (strict: stops at the first error).
    pub fn parse_file(&mut self) -> Result<FileNode, Error> {
        let language = self.parse_language_decl()?;
        let planet = self.parse_planet_decl()?;

        // Expect EOF
        self.expect(TokenKind::Eof)?;

        Ok(FileNode { language, planet })
    }

    /// Parse the complete file with error recovery.
    ///
    /// Unlike [`Self::parse_file`], this collects multiple independent syntax
    /// errors in a single pass. It synchronizes at field boundaries (commas) and
    /// the closing brace of the planet body, so a mistake in one field does not
    /// hide errors in later fields.
    ///
    /// Returns the AST on success, or every collected [`ParseError`] on failure.
    /// When errors are returned the AST is discarded (callers that want a
    /// best-effort partial AST can be added later); the contract here is
    /// "report as many real problems as possible".
    pub fn parse_file_recover(&mut self) -> Result<FileNode, ParseErrors> {
        self.errors.clear();

        // The language declaration is the anchor; if it fails we cannot
        // meaningfully recover the rest, so record and bail.
        let language = match self.parse_language_decl() {
            Ok(l) => Some(l),
            Err(e) => {
                self.record(e);
                None
            }
        };

        // Parse the planet header, recovering the field list even if parts fail.
        let planet = self.parse_planet_decl_recover();

        // Drain any trailing tokens before EOF as a single trailing-garbage error.
        if !self.is_at_end() {
            let span = self.current().span;
            self.record(Error::parse(format!(
                "unexpected trailing tokens starting at {:?}",
                self.current().kind
            )));
            // Advance to EOF so we don't loop.
            while !self.is_at_end() {
                self.advance();
            }
            let _ = span;
        }

        if self.errors.is_empty() {
            // Safe: no errors means both parsed.
            match (language, planet) {
                (Some(language), Some(planet)) => Ok(FileNode { language, planet }),
                _ => Err(ParseErrors::new(std::mem::take(&mut self.errors))),
            }
        } else {
            Err(ParseErrors::new(std::mem::take(&mut self.errors)))
        }
    }

    /// Record an [`Error`] as a [`ParseError`] at the current span.
    fn record(&mut self, err: Error) {
        let span = self.current().span;
        let message = match err {
            Error::Parse(m) | Error::Lexer(m) => m,
            other => other.to_string(),
        };
        self.errors.push(ParseError::new(message, span));
    }

    /// Recovering version of `parse_planet_decl`: parses `planet <id> { ... }`,
    /// collecting field-level errors and synchronizing between fields.
    fn parse_planet_decl_recover(&mut self) -> Option<PlanetNode> {
        let start = self.current().span;

        if self.expect(TokenKind::Planet).is_err() {
            self.record(Error::parse("expected 'planet'"));
            return None;
        }

        let identifier = match self.expect(TokenKind::Identifier) {
            Ok(tok) => tok.text.clone(),
            Err(_) => {
                self.record(Error::parse("expected planet identifier"));
                // Try to continue to the body if a brace is near.
                String::new()
            }
        };

        if self.expect(TokenKind::LeftBrace).is_err() {
            self.record(Error::parse("expected '{' to open planet body"));
            return None;
        }

        let fields = self.parse_field_list_recover();

        if self.expect(TokenKind::RightBrace).is_err() {
            self.record(Error::parse("expected '}' to close planet body"));
        }

        let end = self.previous().span;
        Some(PlanetNode {
            identifier,
            fields,
            span: start.union(end),
        })
    }

    /// Recovering field list: on a field error, record it and skip to the next
    /// synchronization point (`,` or `}`), then continue.
    fn parse_field_list_recover(&mut self) -> Vec<FieldNode> {
        let mut fields = Vec::new();

        loop {
            // Stop at the closing brace or EOF.
            if self.check(TokenKind::RightBrace) || self.is_at_end() {
                break;
            }

            match self.parse_field() {
                Ok(field) => {
                    fields.push(field);
                    // Consume a separating comma if present.
                    if self.check(TokenKind::Comma) {
                        self.advance();
                    } else if !self.check(TokenKind::RightBrace) && !self.is_at_end() {
                        // Missing comma between fields is itself an error, but we
                        // can keep going.
                        self.record(Error::parse("expected ',' between fields"));
                        self.synchronize_field();
                    }
                }
                Err(e) => {
                    self.record(e);
                    // Skip to the next field boundary and keep collecting.
                    if !self.synchronize_field() {
                        break;
                    }
                }
            }
        }

        fields
    }

    /// Advance until just past the next field separator (`,`) or to the closing
    /// brace / EOF. Returns `true` if more fields may follow (a `,` was consumed),
    /// `false` if we reached `}` or EOF.
    fn synchronize_field(&mut self) -> bool {
        while !self.is_at_end() {
            match self.current().kind {
                TokenKind::Comma => {
                    self.advance();
                    // If the comma is immediately followed by `}`, no more fields.
                    return !self.check(TokenKind::RightBrace);
                }
                TokenKind::RightBrace => return false,
                _ => {
                    self.advance();
                }
            }
        }
        false
    }

    /// Parse a language declaration: `language hpdl "1.0";`
    fn parse_language_decl(&mut self) -> Result<LanguageDeclNode, Error> {
        let start = self.current().span;

        self.expect(TokenKind::Language)?;
        self.expect(TokenKind::Hpdl)?;

        let version_token = self.expect(TokenKind::String)?;
        // Extract version string (remove quotes)
        let version = version_token.text.trim_matches('"').to_string();

        self.expect(TokenKind::Semicolon)?;

        let end = self.previous().span;

        Ok(LanguageDeclNode {
            version,
            span: start.union(end),
        })
    }

    /// Parse a planet declaration: `planet identifier { fields }`
    fn parse_planet_decl(&mut self) -> Result<PlanetNode, Error> {
        let start = self.current().span;

        self.expect(TokenKind::Planet)?;

        let id_token = self.expect(TokenKind::Identifier)?;
        let identifier = id_token.text.clone();

        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_field_list()?;

        self.expect(TokenKind::RightBrace)?;

        let end = self.previous().span;

        Ok(PlanetNode {
            identifier,
            fields,
            span: start.union(end),
        })
    }

    /// Parse a list of fields: `field1: value1, field2: value2, ...`
    fn parse_field_list(&mut self) -> Result<Vec<FieldNode>, Error> {
        let mut fields = Vec::new();

        // Allow empty field list
        if self.check(TokenKind::RightBrace) {
            return Ok(fields);
        }

        loop {
            fields.push(self.parse_field()?);

            // Check for comma (and allow trailing comma)
            if self.check(TokenKind::Comma) {
                self.advance();

                // Allow trailing comma before closing brace
                if self.check(TokenKind::RightBrace) {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(fields)
    }

    /// Parse a single field: `name: value`.
    ///
    /// Field names may be identifiers or contextual keywords (e.g. `physical`,
    /// `traits`, `status`, `action`, `module`, `migration`, `body`). These words
    /// are keywords at the lexical level but are valid field names here, so we
    /// accept them by their text.
    fn parse_field(&mut self) -> Result<FieldNode, Error> {
        let start = self.current().span;

        let name_token = self.expect_field_name()?;
        let name = name_token.text.clone();

        self.expect(TokenKind::Colon)?;

        let value = self.parse_value()?;

        let end = value.span();

        Ok(FieldNode {
            name,
            value,
            span: start.union(end),
        })
    }

    /// Parse a value (literal, list, or object).
    fn parse_value(&mut self) -> Result<ValueNode, Error> {
        let token = self.current();

        match token.kind {
            TokenKind::String => {
                let span = token.span;
                let value = token.text.trim_matches('"').to_string();
                self.advance();
                Ok(ValueNode::String { value, span })
            }

            TokenKind::Integer | TokenKind::Float => self.parse_number(),

            TokenKind::True => {
                let span = token.span;
                self.advance();
                Ok(ValueNode::Boolean { value: true, span })
            }

            TokenKind::False => {
                let span = token.span;
                self.advance();
                Ok(ValueNode::Boolean { value: false, span })
            }

            TokenKind::LeftBracket => self.parse_list(),

            TokenKind::LeftBrace => self.parse_object(),

            _ => Err(Error::parse(format!(
                "Expected value, got {:?}",
                token.kind
            ))),
        }
    }

    /// Parse a number (with optional unit).
    fn parse_number(&mut self) -> Result<ValueNode, Error> {
        let token = self.current();
        let span = token.span;
        let text = &token.text;

        // Parse the number. Integers are kept as i128 to preserve planetary
        // magnitudes exactly; only genuine floats go through f64.
        let is_float = token.kind == TokenKind::Float;
        let cleaned = text.replace('_', "");

        let node = if is_float {
            let value = cleaned
                .parse::<f64>()
                .map_err(|e| Error::parse(format!("Invalid float: {}", e)))?;
            self.advance();
            let unit = self.parse_optional_unit();
            ValueNode::Float { value, unit, span }
        } else {
            let value = cleaned
                .parse::<i128>()
                .map_err(|e| Error::parse(format!("Invalid integer: {}", e)))?;
            self.advance();
            let unit = self.parse_optional_unit();
            ValueNode::Integer { value, unit, span }
        };

        Ok(node)
    }

    /// Consume an optional unit-suffix identifier following a number.
    fn parse_optional_unit(&mut self) -> Option<String> {
        if self.check(TokenKind::Identifier) {
            let unit_token = self.advance();
            Some(unit_token.text.clone())
        } else {
            None
        }
    }

    /// Parse a list: `[value1, value2, ...]`
    fn parse_list(&mut self) -> Result<ValueNode, Error> {
        let start = self.current().span;
        self.expect(TokenKind::LeftBracket)?;

        let mut elements = Vec::new();

        // Allow empty list
        if !self.check(TokenKind::RightBracket) {
            loop {
                elements.push(self.parse_value()?);

                if self.check(TokenKind::Comma) {
                    self.advance();

                    // Allow trailing comma
                    if self.check(TokenKind::RightBracket) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        self.expect(TokenKind::RightBracket)?;
        let end = self.previous().span;

        Ok(ValueNode::List {
            elements,
            span: start.union(end),
        })
    }

    /// Parse an object: `{ field1: value1, field2: value2 }`
    fn parse_object(&mut self) -> Result<ValueNode, Error> {
        let start = self.current().span;
        self.expect(TokenKind::LeftBrace)?;

        let fields = self.parse_field_list()?;

        self.expect(TokenKind::RightBrace)?;
        let end = self.previous().span;

        Ok(ValueNode::Object {
            fields,
            span: start.union(end),
        })
    }

    /// Expect a field-name token: an identifier or a contextual keyword that is
    /// permitted as a field name.
    fn expect_field_name(&mut self) -> Result<Token, Error> {
        let kind = self.current().kind;
        let is_field_name = matches!(
            kind,
            TokenKind::Identifier
                | TokenKind::Body
                | TokenKind::Physical
                | TokenKind::Traits
                | TokenKind::Status
                | TokenKind::Action
                | TokenKind::Module
                | TokenKind::Migration
        );
        if is_field_name {
            Ok(self.advance())
        } else {
            Err(Error::parse(format!("Expected field name, got {:?}", kind)))
        }
    }

    /// Expect a specific token kind and advance.
    fn expect(&mut self, kind: TokenKind) -> Result<Token, Error> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(Error::parse(format!(
                "Expected {:?}, got {:?}",
                kind,
                self.current().kind
            )))
        }
    }

    /// Check if the current token matches the given kind.
    fn check(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    /// Advance to the next token and return the previous one.
    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.previous()
    }

    /// Get the current token.
    fn current(&self) -> &Token {
        // Parser invariant: token stream always has at least EOF token from lexer
        if let Some(token) = self.tokens.get(self.pos) {
            token
        } else if let Some(token) = self.tokens.last() {
            token
        } else {
            // This branch should never execute if lexer is correct (always produces EOF)
            // Return first token as fallback (lexer guarantees at least one token)
            // Safe: lexer always produces at least EOF token
            if let Some(token) = self.tokens.first() {
                token
            } else {
                // Ultimate fallback - should never happen
                panic!("Parser created with empty token stream - lexer contract violated")
            }
        }
    }

    /// Get the previous token.
    fn previous(&self) -> Token {
        // Safe because we never call previous() before advancing past first token
        let idx = self.pos.saturating_sub(1);
        if let Some(token) = self.tokens.get(idx) {
            return token.clone();
        }

        // Fallback: return first token (lexer guarantees at least EOF exists)
        if let Some(token) = self.tokens.first() {
            return token.clone();
        }

        // This should never happen - lexer always produces at least EOF
        // But we must provide a fallback to satisfy the type system
        Token {
            kind: TokenKind::Eof,
            span: SourceSpan { start: 0, end: 0 },
            text: String::new(),
        }
    }

    /// Check if we're at the end of the token stream.
    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_language_decl() {
        let source = r#"language hpdl "1.0";"#.to_string();
        let mut parser = Parser::from_source(source).unwrap();

        let lang = parser.parse_language_decl().unwrap();
        assert_eq!(lang.version, "1.0");
    }

    #[test]
    fn test_parse_simple_planet() {
        let source = r#"
            language hpdl "1.0";
            planet test {
                id: "test-planet"
            }
        "#
        .to_string();

        let mut parser = Parser::from_source(source).unwrap();
        let file = parser.parse_file().unwrap();

        assert_eq!(file.language.version, "1.0");
        assert_eq!(file.planet.identifier, "test");
        assert_eq!(file.planet.fields.len(), 1);
    }

    #[test]
    fn test_parse_with_numbers() {
        let source = r#"
            language hpdl "1.0";
            planet earth {
                mass: 5972000000000000000000000 kg,
                radius: 6371000 m
            }
        "#
        .to_string();

        let mut parser = Parser::from_source(source).unwrap();
        let file = parser.parse_file().unwrap();

        assert_eq!(file.planet.fields.len(), 2);
    }

    #[test]
    fn test_parse_with_list() {
        let source = r#"
            language hpdl "1.0";
            planet test {
                tags: ["terrestrial", "habitable"]
            }
        "#
        .to_string();

        let mut parser = Parser::from_source(source).unwrap();
        let file = parser.parse_file().unwrap();

        assert_eq!(file.planet.fields.len(), 1);
        if let ValueNode::List { elements, .. } = &file.planet.fields[0].value {
            assert_eq!(elements.len(), 2);
        } else {
            panic!("Expected list");
        }
    }
}
