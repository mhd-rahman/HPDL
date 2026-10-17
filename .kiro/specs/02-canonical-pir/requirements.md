# Requirements — Canonical PIR

> **Feature:** `02-canonical-pir`  
> **Design doc:** `docs/design/02-canonical-pir.md`  
> **Crate:** `hpdl-pir`  
> **Status:** ✅ Complete

---

## Overview

Define the canonical **Planetary Interchange Representation** (PIR) encoding format for HPDL v1. PIR is a versioned, deterministic binary format that preserves all semantics from the source definitions while being suitable for hashing, storage, and transmission to the HORIZON runtime.

This crate is a **published contract crate** — its public API is SemVer-stable and must not depend on compiler internals.

---

## Functional Requirements

### REQ-01 — `PirVersion` type

A strongly-typed PIR format version identifier.

**Acceptance criteria:**
- `PirVersion` is a newtype wrapping a `u32`.
- `PirVersion::V1` constant exists for HPDL v1.
- `PirVersion` implements `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Display`.
- `Display` renders as `"PIR/1"` for version 1.

---

### REQ-02 — `PirEnvelope` — versioned wrapper

The top-level PIR structure that wraps any encoded definition with its format version.

**Acceptance criteria:**
- `PirEnvelope { version: PirVersion, payload: Vec<u8> }`.
- `PirEnvelope::new(version, payload) -> Self` constructor.
- `version()` and `payload()` accessor methods.
- `PirEnvelope` implements `Clone`, `Debug`, `PartialEq`, `Eq`.

---

### REQ-03 — `CanonicalHash` type

A domain-separated cryptographic hash digest of PIR bytes.

**Acceptance criteria:**
- `CanonicalHash` wraps a fixed-size byte array (e.g. `[u8; 32]` for SHA-256).
- `CanonicalHash::compute(domain: &str, pir_bytes: &[u8]) -> Self` — computes domain-separated hash.
- Domain string is UTF-8, prepended to the input before hashing.
- `as_bytes()` returns `&[u8; 32]`.
- `to_hex() -> String` returns lowercase hex encoding.
- `CanonicalHash` implements `Copy`, `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`.

---

### REQ-04 — Canonical encoding rules

PIR encoding must be deterministic across all platforms (Linux, macOS, Windows).

**Acceptance criteria:**
- **Field ordering:** Struct fields encoded in the order they appear in the source type definition (lexicographic field names if using serde).
- **String encoding:** UTF-8 bytes prefixed with varint length.
- **Integer encoding:** Fixed-width little-endian for `u32`, `u64`, `u128`, `i64` where applicable.
- **Collections:** Length-prefixed, elements in iteration order (`BTreeMap`/`BTreeSet` guarantee lexicographic).
- **Option:** 0x00 byte for `None`, 0x01 byte followed by encoded value for `Some`.
- **No floating-point:** All numeric values use scaled integers (e.g. milli-units).
- Encoding is idempotent: encoding the same logical value twice produces byte-for-byte identical output.

---

### REQ-05 — `encode_planet_definition` function

Canonical encoder for a `PlanetDefinition` into PIR bytes.

**Acceptance criteria:**
- `fn encode_planet_definition(def: &PlanetDefinition) -> Result<Vec<u8>, Error>`.
- Returns a deterministic byte sequence for the given definition.
- Same `PlanetDefinition` on Linux, macOS, and Windows produces identical bytes.
- Includes a version prefix in the payload identifying PIR v1.

---

### REQ-06 — `decode_planet_definition` function

Bounded decoder that rejects malformed or oversized input.

**Acceptance criteria:**
- `fn decode_planet_definition(bytes: &[u8]) -> Result<PlanetDefinition, Error>`.
- Returns `Error::VersionMismatch` if the version byte(s) don't match PIR v1.
- Returns `Error::MalformedEncoding` if lengths are inconsistent, nesting exceeds limits, or data is truncated.
- Returns `Error::OversizedInput` if total input exceeds a hardcoded limit (e.g. 10MB).
- Enforces maximum nesting depth (e.g. 64 levels).
- Never panics on malformed input; always returns a typed error.

---

### REQ-07 — Cross-platform determinism test

**Acceptance criteria:**
- A golden test encodes a known `PlanetDefinition`, computes its canonical hash, and asserts the hash matches a frozen hex string.
- The same test passes on Linux, macOS, and Windows with byte-for-byte identical PIR output.
- The frozen hash is stored in `tests/golden/pir_v1_hash.txt`.

---

### REQ-08 — Roundtrip fuzz boundary

**Acceptance criteria:**
- A test constructs a `PlanetDefinition` with boundary values (empty strings, max-length IDs, min/max scale bounds, retrograde rotation, all-`None` properties).
- Encodes it, decodes it, asserts equality.
- Encode → decode → encode produces identical bytes on the second encode.

---

### REQ-09 — Reject malformed input

**Acceptance criteria:**
- Tests confirm that truncated bytes, invalid UTF-8, lengths exceeding declared sizes, and excessive nesting all return `Error::MalformedEncoding`.
- A test with input > 10MB returns `Error::OversizedInput`.
- A test with nesting depth > 64 returns `Error::ExcessiveNesting`.

---

### REQ-10 — `Error` enum

All fallible PIR operations return a typed error.

**Acceptance criteria:**
- Variants: `VersionMismatch`, `MalformedEncoding`, `OversizedInput`, `ExcessiveNesting`, `InvalidUtf8`, `UnexpectedEof`.
- Derived with `thiserror::Error`.
- `Error` implements `Debug`, `Display`, `std::error::Error`, `PartialEq`, `Eq`.

---

### REQ-11 — No dependency on `hpdl-compiler`

Cross-cutting constraint.

**Acceptance criteria:**
- `hpdl-pir/Cargo.toml` dependencies list includes only: `hpdl-model`, `thiserror`, `sha2` (or equivalent hash crate), and optionally `bincode` or similar for encoding.
- Does **not** depend on `hpdl-compiler`, `hpdl-sema`, `hpdl-ron`, or `hpdl-cli`.
- CI verifies no reverse dependency exists.

---

## Non-Functional Requirements

| ID | Requirement |
|---|---|
| NFR-01 | `hpdl-pir` is a published contract crate with SemVer stability. |
| NFR-02 | Every public item has rustdoc with a usage example. |
| NFR-03 | `cargo doc -p hpdl-pir --no-deps` produces no warnings. |
| NFR-04 | Encoding/decoding is constant-time for a given structure size (no data-dependent branches that leak via timing). |
| NFR-05 | Canonical hash domain separation prevents collision attacks across different entity types. |

---

## Out of Scope

- LSP integration, syntax highlighting, or editor tooling.
- Network transmission protocol (PIR is a payload; transport is separate).
- Encryption or authentication (PIR bytes are plaintext; wrapping layers handle security).
- Compression (may be applied externally to PIR blobs).

---

## Open Decisions (require explicit approval)

| # | Decision |
|---|---|
| OD-09 | Hash algorithm choice: SHA-256 assumed; confirm or specify BLAKE3/SHA-3. |
| OD-10 | Maximum PIR blob size limit (10MB assumed; adjust based on expected planet complexity). |
| OD-11 | Binary encoding library: `bincode`, `postcard`, or custom? Must guarantee canonical field order. |
