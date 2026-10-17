# hpdl-syntax

Native `.hpdl` lexer, parser, and AST→model converter. Parses HPDL source into the
same `hpdl_model::PlanetDefinition` the RON frontend produces, and routes it through
the shared `hpdl-sema` validator so both frontends share identical semantics.

> **Grammar status:** native HPDL grammar **v1.0 is ratified** at full parity with
> the RON frontend (see [`GRAMMAR.md`](./GRAMMAR.md) and
> [ADR 0010](../../docs/rfcs/0010-native-hpdl-grammar-v1.md)). Equivalent `.hpdl` and
> `.ron` sources produce an identical model, identical validator diagnostics, and
> byte-identical canonical PIR. World laws are out of scope for v1.0.

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

## Surface and limitations

- Full parity with RON: `id`, `physical`, `traits`, `modules`, `statuses` (incl.
  the optional vitality fields), `actions` (incl. capabilities), `migrations`, and
  the optional `identity`/`parent`/`orbit`/`rotation`/`environment` blocks.
- Enum variants and newtypes are spelled as in RON (`Planet`, `Custom("x")`,
  `Present(Microbial)`, `ModuleRef(name:, version:)`); pairs are tuples `(a, b)`.
- Unit suffixes after numbers are advisory and never rescale; physical values must be
  given as integers in canonical units. Floats are rejected on canonical fields.
- **World laws** are out of scope for grammar v1.0 (the model field is
  `#[serde(skip)]`; coupled to a future PIR format decision — ADR 0005 / 0010).
- `\uXXXX` escapes and non-decimal number bases are reserved (rejected) in v1.0.

## Examples

```sh
# examples/*.hpdl are parsed in tests/integration_tests.rs
```

## Dependencies

`hpdl-model`, `hpdl-sema`, `thiserror`.
