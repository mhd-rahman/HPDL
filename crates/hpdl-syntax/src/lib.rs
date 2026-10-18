//! # hpdl-syntax
//!
//! Native HPDL syntax parser. Provides lexer and parser for `.hpdl` files,
//! transforming them into the same `PlanetDefinition` model used by the RON frontend.
//!
//! ## Architecture
//!
//! ```text
//! .hpdl file → Lexer → Tokens → Parser → AST → Converter → PlanetDefinition
//! ```
//!
//! ## Example
//!
//! ```no_run
//! use hpdl_syntax::{parse_file, SourceFile};
//!
//! # fn example() -> Result<(), hpdl_syntax::Error> {
//! let source = SourceFile::from_path("planet.hpdl")?;
//! let definition = parse_file(&source)?;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

pub mod ast;
mod convert;
mod diagnostic;
mod error;
mod lexer;
mod limits;
mod parser;
mod span;
mod token;

pub use ast::{FieldNode, FileNode, LanguageDeclNode, PlanetNode, ValueNode};
pub use diagnostic::{ParseError, ParseErrors};
pub use error::Error;
pub use lexer::Lexer;
pub use limits::ParseLimits;
pub use parser::Parser;
pub use span::{SourceFile, SourceSpan};
pub use token::{Token, TokenKind};

use hpdl_model::PlanetDefinition;

/// Parse an HPDL source file into a PlanetDefinition.
///
/// # Errors
///
/// Returns an error if lexing, parsing, or conversion fails.
///
/// # Example
///
/// ```no_run
/// use hpdl_syntax::{parse_file, SourceFile};
///
/// # fn example() -> Result<(), hpdl_syntax::Error> {
/// let source = SourceFile::from_string("test.hpdl", r#"
///     language hpdl "1.0";
///     planet earth {
///         id: "earth",
///         physical: {
///             mass_kg: 5972000000000000000000000,
///             radius_m: 6371000,
///         },
///         traits: {
///             tags: ["terrestrial"],
///         },
///         modules: [],
///         statuses: [],
///         actions: [],
///         migrations: [],
///     }
/// "#.to_string());
///
/// let definition = parse_file(&source)?;
/// assert_eq!(definition.language_version(), "1.0");
/// # Ok(())
/// # }
/// ```
pub fn parse_file(source: &SourceFile) -> Result<PlanetDefinition, Error> {
    parse_file_with_limits(source, &ParseLimits::default())
}

/// Parse an HPDL source file, enforcing the given resource [`ParseLimits`].
///
/// Use this on untrusted input to bound the work the parser will attempt.
///
/// # Errors
///
/// Returns [`Error::Security`] if a limit is exceeded, or a lex/parse/convert
/// error otherwise.
pub fn parse_file_with_limits(
    source: &SourceFile,
    limits: &ParseLimits,
) -> Result<PlanetDefinition, Error> {
    // Enforce the source-size budget before doing any work.
    limits.check_source_size(source.content())?;

    // Lex under the given limits (bounds identifier length).
    let mut lexer = Lexer::new_with_limits(source.content().to_string(), *limits);
    let tokens = lexer.lex_all()?;

    // Parse under the same limits (bounds value-nesting depth).
    let mut parser = Parser::new_with_limits(tokens, *limits);
    let ast = parser.parse_file()?;

    // Convert to model
    convert::file_to_planet_definition(ast)
}

/// Parse an HPDL source file, collecting **all** recoverable syntax errors.
///
/// Unlike [`parse_file`], which stops at the first error, this uses the parser's
/// recovery mode to report multiple independent problems in one pass. It
/// synchronizes at field boundaries inside the planet body.
///
/// On success returns the [`FileNode`] AST; on failure returns every collected
/// [`ParseError`] (see [`ParseErrors`]).
///
/// # Example
///
/// ```
/// use hpdl_syntax::{parse_source_collect_errors, SourceFile};
///
/// // Two bad fields: a missing value and a stray token. Both are reported.
/// let source = SourceFile::from_string("bad.hpdl", r#"
///     language hpdl "1.0";
///     planet earth {
///         id: ,
///         physical: { mass_kg: 1 } @@@
///     }
/// "#.to_string());
///
/// let errors = parse_source_collect_errors(&source).unwrap_err();
/// assert!(errors.len() >= 1);
/// ```
pub fn parse_source_collect_errors(source: &SourceFile) -> Result<FileNode, ParseErrors> {
    let mut lexer = Lexer::new(source.content().to_string());
    let tokens = match lexer.lex_all() {
        Ok(t) => t,
        Err(e) => {
            // A lexer error can't be recovered field-by-field; report it as a
            // single diagnostic spanning the start of input.
            let msg = match e {
                Error::Lexer(m) | Error::Parse(m) => m,
                other => other.to_string(),
            };
            return Err(ParseErrors::new(vec![ParseError::new(
                msg,
                SourceSpan::new(0, 0),
            )]));
        }
    };

    let mut parser = Parser::new_with_limits(tokens, ParseLimits::default());
    parser.parse_file_recover()
}

/// Parse an HPDL source file and route the result through the shared
/// [`hpdl_sema`] semantic validator.
///
/// This is the convenience entry point that mirrors the RON frontend's
/// `ParsedDefinition::validate`: both frontends converge on the *same* validator,
/// so native HPDL and RON share identical semantics.
///
/// # Errors
///
/// Returns [`Error::Parse`] if lexing/parsing/conversion fails, or
/// [`Error::Validation`] carrying the semantic diagnostics if validation fails.
///
/// # Example
///
/// ```
/// use hpdl_syntax::{parse_and_validate, SourceFile};
///
/// let source = SourceFile::from_string("earth.hpdl", r#"
///     language hpdl "1.0";
///     planet earth {
///         id: "earth",
///         physical: { mass_kg: 5972000000000000000000000, radius_m: 6371000 },
///     }
/// "#.to_string());
///
/// let validated = parse_and_validate(&source).expect("valid");
/// assert_eq!(validated.definition().body().id().as_str(), "earth");
/// ```
pub fn parse_and_validate(source: &SourceFile) -> Result<hpdl_sema::ValidatedDefinition, Error> {
    let definition = parse_file(source)?;
    let validator = hpdl_sema::Validator::default_config();
    validator.validate(definition).map_err(|diags| {
        let rendered = diags
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        Error::validation(rendered)
    })
}
