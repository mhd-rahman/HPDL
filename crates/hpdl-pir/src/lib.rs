//! # hpdl-pir — HPDL Planetary Intermediate Representation
//!
//! This crate defines the canonical binary encoding for HPDL planet definitions.
//!
//! ## Key Concepts
//!
//! - **PIR Version**: Format versioning for backward compatibility tracking.
//! - **PIR Envelope**: Versioned container with content hash for integrity verification.
//! - **Canonical Encoding**: Deterministic serialization guaranteeing identical bytes
//!   for identical planet definitions across platforms and compiler versions.
//! - **Content Addressing**: SHA-256 hashing with domain separation for tamper detection.
//!
//! ## Usage
//!
//! ```
//! use hpdl_model::{PlanetDefinition, BodyId, CelestialBody, PhysicalProperties, TraitSet};
//! use hpdl_pir::{encode_planet_definition, decode_planet_definition};
//!
//! // Create a planet definition
//! let body = CelestialBody::new(
//!     BodyId::new("earth").unwrap(),
//!     PhysicalProperties::default(),
//!     TraitSet::empty(),
//!     vec![], vec![], vec![], vec![],
//! ).unwrap();
//! let planet = PlanetDefinition::new("1.0", body).unwrap();
//!
//! // Encode to PIR
//! let envelope = encode_planet_definition(&planet).expect("encoding failed");
//! assert!(envelope.verify_hash());
//!
//! // Serialize envelope for storage/transmission
//! let bytes = wincode::serialize(&envelope).expect("envelope serialization failed");
//!
//! // Decode back
//! let decoded = decode_planet_definition(&bytes).expect("decoding failed");
//! assert_eq!(decoded.body().id(), planet.body().id());
//! ```
//!
//! ## Stability Guarantees
//!
//! This is a **published contract crate**. Breaking changes require a major version bump
//! and explicit migration path. The PIR format version is independent of the crate version.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::float_arithmetic
)]
#![warn(missing_docs, rustdoc::broken_intra_doc_links)]

pub mod decoding;
pub mod encoding;
pub mod envelope;
pub mod error;
pub mod hash;
pub mod version;

// Re-export primary API
pub use decoding::decode_planet_definition;
pub use encoding::encode_planet_definition;
pub use envelope::PirEnvelope;
pub use error::Error;
pub use hash::CanonicalHash;
pub use version::PirVersion;
