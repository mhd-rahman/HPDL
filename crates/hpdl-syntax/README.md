# hpdl-syntax

Native `.hpdl` lexer, parser, and AST→model converter. Parses HPDL source into the
same `hpdl_model::PlanetDefinition` the RON frontend produces, and routes it through
the shared `hpdl-sema` validator so both frontends share identical semantics.

> **Grammar status:** the HPDL language grammar is **not yet ratified**. This crate
> implements a concrete subset so authoring and tests have a target. See
> [`GRAMMAR.md`](./GRAMMAR.md). Treat all syntax as illustrative.

## Pipeline

```text
.hpdl → Lexer → Tokens → Parser → AST → Converter → PlanetDefinition → (hpdl-sema)
```

## Usage

```rust
use hpdl_syntax::{parse_and_validate, SourceFile};

# fn main() -> Result<(), hpdl_syntax::Error> {
let source = SourceFile::from_string("earth.hpdl", r#"
    language hpdl "1.0";
    planet earth {
        id: "earth",
        physical: { mass_kg: 5972000000000000000000000, radius_m: 6371000 },
    }
"#.to_string());

// Parse only:
let definition = hpdl_syntax::parse_file(&source)?;

// Or parse and validate through the shared pipeline:
let validated = parse_and_validate(&source)?;
assert_eq!(validated.definition().body().id().as_str(), "earth");
# Ok(())
# }
```

## Entry points

- `parse_file(&SourceFile) -> Result<PlanetDefinition, Error>` — lex + parse + convert.
- `parse_file_with_limits(&SourceFile, &ParseLimits)` — same, with explicit resource
  bounds for untrusted input.
- `parse_and_validate(&SourceFile) -> Result<ValidatedDefinition, Error>` — the full
  frontend pipeline, routing through `hpdl-sema`.

## Security

`ParseLimits` bounds source size (default 10 MiB) and identifier length (default 256).
Oversized source fails closed with `Error::Security` before any parsing work.
`#![forbid(unsafe_code)]`; `unwrap`/`expect`/panicking-index are denied in non-test
code.

## Known limitations

- `modules`, `statuses`, `actions`, and `migrations` are not yet extracted from HPDL
  source (the converter returns empty collections). The RON frontend supports these
  today; native syntax will follow once the grammar is ratified.
- Unit suffixes after numbers are accepted but not yet used to rescale; physical values
  must be given as integers in canonical units.

## Examples

```sh
# examples/*.hpdl are parsed in tests/integration_tests.rs
```

## Dependencies

`hpdl-model`, `hpdl-sema`, `thiserror`.
