//! Lexer (tokenizer) for HPDL syntax.

use crate::{Error, SourceSpan, Token, TokenKind};

/// Lexer that converts source text into tokens.
#[derive(Debug)]
pub struct Lexer {
    /// The source text being lexed.
    source: String,

    /// Current position in the source (byte offset).
    pos: usize,

    /// Current line number (1-indexed).
    line: usize,

    /// Current column number (1-indexed).
    col: usize,
}

impl Lexer {
    /// Create a new lexer for the given source text.
    pub fn new(source: String) -> Self {
        Self {
            source,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Get the next token from the source.
    pub fn next_token(&mut self) -> Result<Token, Error> {
        // Skip whitespace and comments
        self.skip_whitespace_and_comments();

        // Check for EOF
        if self.is_at_end() {
            return Ok(Token::new(
                TokenKind::Eof,
                SourceSpan::new(self.pos, self.pos),
                String::new(),
            ));
        }

        let start = self.pos;
        let ch = self.current_char();

        // Match token types
        let kind = match ch {
            // Single-character tokens
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            ';' => {
                self.advance();
                TokenKind::Semicolon
            }
            '{' => {
                self.advance();
                TokenKind::LeftBrace
            }
            '}' => {
                self.advance();
                TokenKind::RightBrace
            }
            '[' => {
                self.advance();
                TokenKind::LeftBracket
            }
            ']' => {
                self.advance();
                TokenKind::RightBracket
            }
            '(' => {
                self.advance();
                TokenKind::LeftParen
            }
            ')' => {
                self.advance();
                TokenKind::RightParen
            }

            // String literals
            '"' => return self.lex_string(),

            // Numbers
            '0'..='9' => return self.lex_number(),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => return self.lex_identifier(),

            // Error: unexpected character
            _ => {
                self.advance();
                return Err(Error::lexer(format!("Unexpected character: '{}'", ch)));
            }
        };

        let text = self.source[start..self.pos].to_string();
        Ok(Token::new(kind, SourceSpan::new(start, self.pos), text))
    }

    /// Lex all tokens into a vector.
    pub fn lex_all(&mut self) -> Result<Vec<Token>, Error> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);

            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    /// Lex a string literal.
    fn lex_string(&mut self) -> Result<Token, Error> {
        let start = self.pos;
        self.advance(); // Skip opening quote

        let mut value = String::new();

        while !self.is_at_end() && self.current_char() != '"' {
            if self.current_char() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(Error::lexer(
                        "Unterminated string: unexpected EOF after escape",
                    ));
                }

                // Handle escape sequences
                match self.current_char() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    _ => {
                        return Err(Error::lexer(format!(
                            "Invalid escape sequence: \\{}",
                            self.current_char()
                        )));
                    }
                }
                self.advance();
            } else {
                value.push(self.current_char());
                self.advance();
            }
        }

        if self.is_at_end() {
            return Err(Error::lexer("Unterminated string literal"));
        }

        self.advance(); // Skip closing quote

        let text = self.source[start..self.pos].to_string();
        Ok(Token::new(
            TokenKind::String,
            SourceSpan::new(start, self.pos),
            text,
        ))
    }

    /// Lex a number (integer or float).
    fn lex_number(&mut self) -> Result<Token, Error> {
        let start = self.pos;

        // Read digits
        while !self.is_at_end()
            && (self.current_char().is_ascii_digit() || self.current_char() == '_')
        {
            self.advance();
        }

        // Check for decimal point (float)
        let mut is_float = false;
        if !self.is_at_end() && self.current_char() == '.' {
            // Peek ahead to ensure it's not just a trailing dot
            if self.pos + 1 < self.source.len() {
                let next_char = self.char_after(self.pos);
                if next_char.is_ascii_digit() {
                    is_float = true;
                    self.advance(); // Skip '.'

                    while !self.is_at_end()
                        && (self.current_char().is_ascii_digit() || self.current_char() == '_')
                    {
                        self.advance();
                    }
                }
            }
        }

        // Check for scientific notation
        if !self.is_at_end() && (self.current_char() == 'e' || self.current_char() == 'E') {
            is_float = true;
            self.advance();

            if !self.is_at_end() && (self.current_char() == '+' || self.current_char() == '-') {
                self.advance();
            }

            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                self.advance();
            }
        }

        let kind = if is_float {
            TokenKind::Float
        } else {
            TokenKind::Integer
        };
        let text = self.source[start..self.pos].to_string();
        Ok(Token::new(kind, SourceSpan::new(start, self.pos), text))
    }

    /// Lex an identifier or keyword.
    fn lex_identifier(&mut self) -> Result<Token, Error> {
        let start = self.pos;

        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                self.advance();
            } else {
                break;
            }
        }

        let text = self.source[start..self.pos].to_string();

        // Check if it's a keyword
        let kind = TokenKind::from_keyword(&text).unwrap_or(TokenKind::Identifier);

        Ok(Token::new(kind, SourceSpan::new(start, self.pos), text))
    }

    /// Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            match self.current_char() {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' => {
                    // Check for comment
                    if self.pos + 1 < self.source.len() {
                        match Some(self.char_after(self.pos)) {
                            Some('/') => {
                                // Line comment
                                self.advance(); // Skip first '/'
                                self.advance(); // Skip second '/'
                                while !self.is_at_end() && self.current_char() != '\n' {
                                    self.advance();
                                }
                            }
                            Some('*') => {
                                // Block comment
                                self.advance(); // Skip '/'
                                self.advance(); // Skip '*'

                                while !self.is_at_end() {
                                    if self.current_char() == '*'
                                        && self.pos + 1 < self.source.len()
                                        && self.char_after(self.pos) == '/'
                                    {
                                        self.advance(); // Skip '*'
                                        self.advance(); // Skip '/'
                                        break;
                                    }
                                    self.advance();
                                }
                            }
                            _ => break,
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    /// Get the current character.
    ///
    /// `pos` is a **byte** offset (spans are byte offsets), so we decode the
    /// character starting at that byte rather than treating `pos` as a char
    /// index — the two diverge as soon as the source contains any multibyte
    /// UTF-8 character (e.g. an em-dash in a comment). Returns `'\0'` at or past
    /// end of input.
    fn current_char(&self) -> char {
        self.char_at(self.pos)
    }

    /// Decode the character starting at byte offset `at`, or `'\0'` if `at` is at
    /// or past the end (or not on a char boundary). Non-panicking.
    fn char_at(&self, at: usize) -> char {
        self.source
            .get(at..)
            .and_then(|s| s.chars().next())
            .unwrap_or('\0')
    }

    /// Decode the character immediately *after* the one at byte offset `at`.
    fn char_after(&self, at: usize) -> char {
        let next = at.saturating_add(self.char_at(at).len_utf8());
        self.char_at(next)
    }

    /// Advance to the next character.
    fn advance(&mut self) {
        if self.pos < self.source.len() {
            if self.current_char() == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += self.current_char().len_utf8();
        }
    }

    /// Check if we're at the end of the source.
    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_keywords() {
        let mut lexer = Lexer::new("language hpdl planet".to_string());

        let token1 = lexer.next_token().unwrap();
        assert_eq!(token1.kind, TokenKind::Language);

        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.kind, TokenKind::Hpdl);

        let token3 = lexer.next_token().unwrap();
        assert_eq!(token3.kind, TokenKind::Planet);
    }

    #[test]
    fn test_lex_identifiers() {
        let mut lexer = Lexer::new("my_planet kepler-186f".to_string());

        let token1 = lexer.next_token().unwrap();
        assert_eq!(token1.kind, TokenKind::Identifier);
        assert_eq!(token1.text, "my_planet");

        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.kind, TokenKind::Identifier);
        assert_eq!(token2.text, "kepler-186f");
    }

    #[test]
    fn test_lex_numbers() {
        let mut lexer = Lexer::new("42 3.14 1e10 1_000_000".to_string());

        let token1 = lexer.next_token().unwrap();
        assert_eq!(token1.kind, TokenKind::Integer);
        assert_eq!(token1.text, "42");

        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.kind, TokenKind::Float);
        assert_eq!(token2.text, "3.14");

        let token3 = lexer.next_token().unwrap();
        assert_eq!(token3.kind, TokenKind::Float);
        assert_eq!(token3.text, "1e10");

        let token4 = lexer.next_token().unwrap();
        assert_eq!(token4.kind, TokenKind::Integer);
        assert_eq!(token4.text, "1_000_000");
    }

    #[test]
    fn test_lex_string() {
        let mut lexer = Lexer::new(r#""hello world" "with\nnewline""#.to_string());

        let token1 = lexer.next_token().unwrap();
        assert_eq!(token1.kind, TokenKind::String);

        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.kind, TokenKind::String);
    }

    #[test]
    fn test_lex_operators() {
        let mut lexer = Lexer::new(": , ; { } [ ] ( )".to_string());

        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Colon);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Comma);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Semicolon);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::LeftBrace);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::RightBrace);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::LeftBracket);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::RightBracket);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::LeftParen);
        assert_eq!(lexer.next_token().unwrap().kind, TokenKind::RightParen);
    }

    #[test]
    fn test_lex_comments() {
        let mut lexer = Lexer::new("// line comment\nplanet /* block */ body".to_string());

        let token1 = lexer.next_token().unwrap();
        assert_eq!(token1.kind, TokenKind::Planet);

        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.kind, TokenKind::Body);
    }

    #[test]
    fn multibyte_chars_in_comments_do_not_desync_position() {
        // Regression: `pos` is a byte offset. A multibyte char (em-dash, 3 bytes
        // in UTF-8) in a comment previously desynced `current_char`/`is_at_end`
        // and produced a spurious `'\0'` error near EOF. The tokens after the
        // comment must still lex, and lexing must terminate cleanly at EOF.
        let src = "// ratified — see ADR 0010 (em-dash: — ✓)\nplanet body".to_string();
        let mut lexer = Lexer::new(src);
        let tokens = lexer.lex_all().expect("multibyte comment must lex cleanly");
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![TokenKind::Planet, TokenKind::Body, TokenKind::Eof]
        );
    }

    #[test]
    fn multibyte_char_in_string_lexes() {
        // A multibyte char inside a string literal must round-trip without a
        // position desync on the following tokens.
        let mut lexer = Lexer::new("\"arcane—aether\" planet".to_string());
        let s = lexer.next_token().unwrap();
        assert_eq!(s.kind, TokenKind::String);
        let p = lexer.next_token().unwrap();
        assert_eq!(p.kind, TokenKind::Planet);
    }
}
