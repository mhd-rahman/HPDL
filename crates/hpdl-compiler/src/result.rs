//! The output of a successful compilation.

use hpdl_pir::PirEnvelope;

use crate::manifest::BuildManifest;

/// The result of a successful compile: a canonical PIR envelope and its
/// reproducible build manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileResult {
    envelope: PirEnvelope,
    manifest: BuildManifest,
}

impl CompileResult {
    /// Construct a compile result from its parts.
    pub(crate) fn new(envelope: PirEnvelope, manifest: BuildManifest) -> Self {
        Self { envelope, manifest }
    }

    /// Return a reference to the canonical PIR envelope.
    #[must_use]
    pub fn envelope(&self) -> &PirEnvelope {
        &self.envelope
    }

    /// Return a reference to the build manifest.
    #[must_use]
    pub fn manifest(&self) -> &BuildManifest {
        &self.manifest
    }

    /// Return the canonical content hash as a hex string.
    #[must_use]
    pub fn content_hash_hex(&self) -> String {
        self.envelope.content_hash.to_hex()
    }

    /// Consume the result and return the envelope and manifest.
    #[must_use]
    pub fn into_parts(self) -> (PirEnvelope, BuildManifest) {
        (self.envelope, self.manifest)
    }
}
