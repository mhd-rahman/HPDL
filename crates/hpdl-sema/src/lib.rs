//! # hpdl-sema
//!
//! Shared semantic validation pipeline for HPDL. Every frontend (RON,
//! native HPDL syntax) routes its parsed output through this crate so
//! validation rules are defined exactly once.
//!
//! Depends on [`hpdl_model`]. Does **not** depend on `hpdl-pir`,
//! `hpdl-compiler`, or `hpdl-cli`.
//!
//! # Architecture
//!
//! The validation pipeline consists of several specialized validators:
//! - **Type Checker**: Validates field types, required fields, and enum variants
//! - **Constraint Solver**: Validates physical constraints and cross-field consistency
//! - **Unit Registry**: Validates units, dimensions, and numeric values
//! - **Status Validator**: Validates status threshold declarations (Amendment A1)
//! - **Action Validator**: Validates action declarations and enforces status immutability
//! - **Module Validator**: Validates module references and world law declarations
//!
//! # Example
//!
//! ```rust,ignore
//! use hpdl_sema::{Validator, ValidatorConfig};
//! use hpdl_model::PlanetDefinition;
//!
//! let config = ValidatorConfig::default();
//! let validator = Validator::new(config);
//!
//! match validator.validate(definition) {
//!     Ok(validated) => println!("Validation succeeded"),
//!     Err(diagnostics) => {
//!         for diag in diagnostics {
//!             eprintln!("{}", diag);
//!         }
//!     }
//! }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

pub mod action_validator;
pub mod constraint_solver;
pub mod diagnostic;
pub mod environment_validator;
pub mod module_validator;
pub mod orbital_validator;
pub mod resolver;
pub mod status_validator;
pub mod type_checker;
pub mod unit_registry;
pub mod validated;
pub mod validator;

pub use diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
pub use resolver::{ModuleResolver, ResolverError};
pub use validated::{ValidatedDefinition, ValidationMetadata};
pub use validator::{Validator, ValidatorConfig};

use thiserror::Error;

/// Errors produced during semantic validation.
#[derive(Debug, Error)]
pub enum Error {
    /// Validation failed with one or more diagnostics.
    #[error("validation failed with {} diagnostic(s)", .0.len())]
    ValidationFailed(Vec<Diagnostic>),

    /// Generic validation error.
    #[error("validation error: {0}")]
    Generic(String),
}
