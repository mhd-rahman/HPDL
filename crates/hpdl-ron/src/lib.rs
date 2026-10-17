//! # hpdl-ron
//!
//! RON frontend for HPDL. Deserializes `.ron` authoring files and routes
//! the result through the shared [`hpdl_sema`] validation pipeline.
//!
//! This crate deliberately avoids implementing its own semantic rules;
//! all validation lives in `hpdl-sema` so RON and native HPDL syntax
//! share identical semantics.
//!
//! ## Example Usage
//!
//! ```no_run
//! use hpdl_ron::{RonLoader, SourceDocument};
//!
//! # fn example() -> Result<(), hpdl_ron::Error> {
//! // Load RON file
//! let source = SourceDocument::from_file("planets/earth.ron")?;
//! let loader = RonLoader::new();
//! let parsed = loader.parse(&source)?;
//!
//! // Validate through shared pipeline
//! let validated = parsed.validate()?;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod diagnostic;
mod error;
mod extensions;
mod loader;
mod parsed;
mod security;
mod source;

pub use diagnostic::FrontendDiagnostic;
pub use error::Error;
pub use extensions::ExtensionConfig;
pub use loader::RonLoader;
pub use parsed::ParsedDefinition;
pub use security::SecurityConfig;
pub use source::{SourceDocument, SourceSpan};
