# hpdl-ron

The RON input adapter for HPDL. It deserializes `.ron` authoring files into the
shared `hpdl_model::PlanetDefinition` and routes them through the one semantic
validator in `hpdl-sema`. It contains **no** semantic rules of its own, so RON and
native HPDL syntax share identical semantics.

## Usage

```rust
use hpdl_ron::{RonLoader, SourceDocument};

# fn main() -> Result<(), hpdl_ron::Error> {
let loader = RonLoader::new();

// From a string (e.g. embedded content):
let parsed = loader.load_string("earth.ron", std::fs::read_to_string("...").unwrap_or_default())?;

// Or from a file (path traversal is rejected):
// let parsed = loader.load_file("planets/earth.ron")?;

// Validate through the shared pipeline:
let validated = parsed.validate()?;
# Ok(())
# }
```

Runnable examples:

```sh
cargo run -p hpdl-ron --example basic_usage
cargo run -p hpdl-ron --example validation_workflow
cargo run -p hpdl-ron --example ron_error_handling
```

## Failure modes

Two distinct stages fail distinctly:

- **Parse failures** — malformed RON, wrong shape, or invalid field values return
  `Error::ParseWithSpan` / `Error::Parse`.
- **Semantic failures** — well-formed RON that violates HPDL rules (e.g. zero mass)
  parses successfully but fails `ParsedDefinition::validate()` via `hpdl-sema`.

## Security

- **File size** is enforced on the parse path (`SecurityConfig::max_file_size`,
  default 10 MB); oversized input returns `Error::FileSizeExceeded`.
- **Path traversal** is rejected when loading from a file path
  (`Error::PathTraversal`).
- `SecurityConfig` also exposes nesting-depth and collection-length validators for
  callers that want to enforce those bounds explicitly.
- `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/panicking-index in non-test code.

## Test coverage

- `tests/validation_integration.rs` — fixtures parse and validate end-to-end.
- `tests/security_tests.rs` — enforced size limit + config validators.
- `tests/malformed_input_tests.rs` — no panic on garbage; typed parse errors.
- `tests/path_security_tests.rs` — traversal rejection.
- `tests/fixtures/*.ron` — minimal, earth-like, gas-giant, dwarf-planet, and an
  intentionally semantically-invalid example.

## Conventions

See [`CONVENTIONS.md`](./CONVENTIONS.md) for the RON authoring shape and the
extension-field mechanism (RFC 0002).

## Dependencies

`hpdl-model`, `hpdl-sema`, `ron`, `serde`, `thiserror`. Must not depend on
`hpdl-pir` or `hpdl-compiler`.
